use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum WatcherError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// A recursive filesystem watcher for music library folders with debounced event dispatch.
pub struct LibraryWatcher {
    watcher: RecommendedWatcher,
    watched_folders: HashSet<PathBuf>,
    stop_tx: Option<crossbeam_channel::Sender<()>>,
    debouncer_thread: Option<JoinHandle<()>>,
}

impl LibraryWatcher {
    /// Creates a new `LibraryWatcher`.
    ///
    /// Changes detected in watched folders are debounced by `debounce_duration`
    /// before being sent as batches of `PathBuf` to `event_sender`.
    pub fn new(
        event_sender: Sender<Vec<PathBuf>>,
        debounce_duration: Duration,
    ) -> Result<Self, WatcherError> {
        let (raw_tx, raw_rx) = crossbeam_channel::unbounded::<Vec<PathBuf>>();
        let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);

        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Any
                        if !event.paths.is_empty() =>
                    {
                        let _ = raw_tx.send(event.paths);
                    }
                    _ => {}
                },
                Err(e) => {
                    warn!("Filesystem watcher error: {}", e);
                }
            },
            Config::default(),
        )?;

        let debouncer_thread = std::thread::Builder::new()
            .name("melody-library-watcher".into())
            .spawn(move || {
                let mut pending: HashMap<PathBuf, Instant> = HashMap::new();
                loop {
                    let recv_result = if let Some(min_instant) = pending.values().min() {
                        let elapsed = min_instant.elapsed();
                        let timeout = if elapsed >= debounce_duration {
                            Duration::from_millis(0)
                        } else {
                            debounce_duration - elapsed
                        };
                        crossbeam_channel::select! {
                            recv(stop_rx) -> _ => break,
                            recv(raw_rx) -> msg => match msg {
                                Ok(paths) => Ok(paths),
                                Err(_) => Err(false),
                            },
                            default(timeout) => Err(true),
                        }
                    } else {
                        crossbeam_channel::select! {
                            recv(stop_rx) -> _ => break,
                            recv(raw_rx) -> msg => match msg {
                                Ok(paths) => Ok(paths),
                                Err(_) => Err(false),
                            },
                        }
                    };

                    match recv_result {
                        Ok(paths) => {
                            let now = Instant::now();
                            for path in paths {
                                pending.insert(path, now);
                            }
                        }
                        Err(true) => {}
                        Err(false) => break,
                    }

                    if pending.is_empty() {
                        continue;
                    }

                    let mut ready = Vec::new();
                    pending.retain(|path, last_time| {
                        if last_time.elapsed() >= debounce_duration {
                            ready.push(path.clone());
                            false
                        } else {
                            true
                        }
                    });

                    if !ready.is_empty() {
                        ready.retain(|p| p.exists());
                        if !ready.is_empty() && event_sender.send(ready).is_err() {
                            break;
                        }
                    }
                }
            })?;

        Ok(Self {
            watcher,
            watched_folders: HashSet::new(),
            stop_tx: Some(stop_tx),
            debouncer_thread: Some(debouncer_thread),
        })
    }

    /// Starts watching a directory recursively.
    pub fn watch_folder<P: AsRef<Path>>(&mut self, path: P) -> Result<(), WatcherError> {
        let path = path.as_ref().to_path_buf();
        if self.watched_folders.contains(&path) {
            return Ok(());
        }
        if !path.exists() {
            warn!("Directory does not exist for watching: {:?}", path);
            return Ok(());
        }
        info!("Watching library folder: {:?}", path);
        self.watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watched_folders.insert(path);
        Ok(())
    }

    /// Stops watching a directory.
    pub fn unwatch_folder<P: AsRef<Path>>(&mut self, path: P) -> Result<(), WatcherError> {
        let path = path.as_ref();
        if self.watched_folders.remove(path) {
            info!("Unwatching library folder: {:?}", path);
            let _ = self.watcher.unwatch(path);
        }
        Ok(())
    }

    /// Synchronizes the set of actively watched folders to match `folders`.
    pub fn update_folders<P: AsRef<Path>>(&mut self, folders: &[P]) -> Result<(), WatcherError> {
        let new_set: HashSet<PathBuf> = folders.iter().map(|p| p.as_ref().to_path_buf()).collect();
        let to_remove: Vec<PathBuf> = self.watched_folders.difference(&new_set).cloned().collect();
        for folder in to_remove {
            self.unwatch_folder(&folder)?;
        }
        for folder in new_set {
            if !self.watched_folders.contains(&folder) {
                let _ = self.watch_folder(&folder);
            }
        }
        Ok(())
    }

    /// Returns the list of currently watched directories.
    pub fn watched_folders(&self) -> Vec<PathBuf> {
        self.watched_folders.iter().cloned().collect()
    }

    /// Returns whether a folder is currently being watched.
    pub fn is_watching<P: AsRef<Path>>(&self, path: P) -> bool {
        self.watched_folders.contains(path.as_ref())
    }

    /// Stops watching all directories.
    pub fn stop(&mut self) {
        for folder in self.watched_folders.drain() {
            let _ = self.watcher.unwatch(&folder);
        }
    }
}

impl Drop for LibraryWatcher {
    fn drop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        for folder in &self.watched_folders {
            let _ = self.watcher.unwatch(folder);
        }
        if let Some(handle) = self.debouncer_thread.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_watcher_folder_management() {
        let (tx, _rx) = crossbeam_channel::unbounded();
        let mut watcher = LibraryWatcher::new(tx, Duration::from_millis(50)).expect("watcher init");

        let temp = std::env::temp_dir().join(format!(
            "melody_watcher_mgmt_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let dir1 = temp.join("dir1");
        let dir2 = temp.join("dir2");
        let dir3 = temp.join("dir3");
        let _ = std::fs::create_dir_all(&dir1);
        let _ = std::fs::create_dir_all(&dir2);
        let _ = std::fs::create_dir_all(&dir3);

        watcher.watch_folder(&dir1).unwrap();
        watcher.watch_folder(&dir2).unwrap();
        assert!(watcher.is_watching(&dir1));
        assert!(watcher.is_watching(&dir2));
        assert!(!watcher.is_watching(&dir3));

        // Unwatch dir1
        watcher.unwatch_folder(&dir1).unwrap();
        assert!(!watcher.is_watching(&dir1));
        assert!(watcher.is_watching(&dir2));

        // Update folders to dir2 and dir3
        watcher.update_folders(&[&dir2, &dir3]).unwrap();
        assert!(!watcher.is_watching(&dir1));
        assert!(watcher.is_watching(&dir2));
        assert!(watcher.is_watching(&dir3));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_library_watcher_detects_new_file() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut watcher =
            LibraryWatcher::new(tx, Duration::from_millis(100)).expect("watcher init");

        let temp = std::env::temp_dir().join(format!(
            "melody_watcher_detect_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp);

        watcher.watch_folder(&temp).unwrap();

        let new_file = temp.join("test_song.flac");
        std::fs::write(&new_file, b"flac content").unwrap();

        // Wait for debounced notification
        let received = rx.recv_timeout(Duration::from_millis(2000));
        assert!(received.is_ok(), "Watcher should emit debounced event");
        let paths = received.unwrap();
        assert!(
            paths.iter().any(|p| p.ends_with("test_song.flac")),
            "Emitted paths {:?} should contain test_song.flac",
            paths
        );

        let _ = std::fs::remove_dir_all(&temp);
    }
}
