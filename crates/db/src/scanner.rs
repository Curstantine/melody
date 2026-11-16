use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tracing::{info, warn};
use walkdir::WalkDir;

use metadata::extract_metadata;
use model::NewTrack;

use crate::{
    is_supported_audio_path, DbError, LibraryDatabase, ScanSummary, SQL_IN_CLAUSE_CHUNK_SIZE,
};

impl LibraryDatabase {
    pub async fn index_files_parallel<F: FnMut(usize, usize)>(
        &self,
        files: &[PathBuf],
        mut progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let total = files.len();
        if let Some(ref mut cb) = progress_cb {
            cb(0, total);
        }

        if files.is_empty() {
            return Ok(ScanSummary::default());
        }

        let num_workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 16)
            .min(total);

        let files = Arc::new(files.to_vec());
        let next_idx = Arc::new(AtomicUsize::new(0));
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);

        let mut handles = Vec::with_capacity(num_workers);
        for w in 0..num_workers {
            let files = Arc::clone(&files);
            let next_idx = Arc::clone(&next_idx);
            let tx = tx.clone();
            let handle = std::thread::Builder::new()
                .name(format!("melody-indexer-{}", w))
                .spawn(move || loop {
                    let idx = next_idx.fetch_add(1, Ordering::Relaxed);
                    if idx >= files.len() {
                        break;
                    }
                    let path = &files[idx];
                    let parsed = extract_metadata(path);
                    if tx.blocking_send((path.clone(), parsed)).is_err() {
                        break;
                    }
                });
            if let Ok(h) = handle {
                handles.push(h);
            }
        }
        drop(tx);

        let mut summary = ScanSummary::default();
        let mut batch: Vec<NewTrack> = Vec::with_capacity(100);
        let notify_interval = (total / 100).max(5);

        while let Some((path, parsed)) = rx.recv().await {
            summary.scanned_files += 1;

            match parsed {
                Ok(track) => {
                    batch.push(track);
                    if batch.len() >= 100 {
                        self.flush_scan_batch(&batch, &mut summary).await;
                        batch.clear();
                    }
                }
                Err(e) => {
                    warn!("Failed to extract metadata for {:?}: {}", path, e);
                    summary.failed_files += 1;
                }
            }

            if let Some(ref mut cb) = progress_cb {
                if total <= 100
                    || summary.scanned_files == total
                    || summary.scanned_files % notify_interval == 0
                {
                    cb(summary.scanned_files, total);
                }
            }
        }

        if !batch.is_empty() {
            self.flush_scan_batch(&batch, &mut summary).await;
        }

        for h in handles {
            let _ = h.join();
        }

        if let Some(ref mut cb) = progress_cb {
            cb(summary.scanned_files, total);
        }

        Ok(summary)
    }

    /// Recursively scan directories and index all supported audio files.
    ///
    /// Candidate audio files are gathered first to determine the total workload,
    /// enabling the progress callback to report `(current, total)`. Metadata
    /// extraction is dispatched to parallel worker threads to maximize CPU utilization.
    pub async fn scan_directories<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        dir_paths: &[P],
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut candidate_files: Vec<PathBuf> = Vec::new();
        for dir_path in dir_paths {
            for entry in WalkDir::new(dir_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() && is_supported_audio_path(path) {
                    candidate_files.push(path.to_path_buf());
                }
            }
        }

        candidate_files.sort();
        candidate_files.dedup();

        let summary = self
            .index_files_parallel(&candidate_files, progress_cb)
            .await?;

        info!(
            "Directory scan finished: {} files scanned, {} tracks indexed, {} errors",
            summary.scanned_files, summary.inserted_tracks, summary.failed_files
        );

        Ok(summary)
    }

    /// Recursively scan directory and index all supported audio files.
    pub async fn scan_directory<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        dir_path: P,
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        self.scan_directories(&[dir_path], progress_cb).await
    }

    /// Filters a list of file paths, returning only those that do NOT currently exist in the database.
    pub async fn filter_new_paths<P: AsRef<Path>>(
        &self,
        paths: &[P],
    ) -> Result<Vec<PathBuf>, DbError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        let string_paths: Vec<String> = paths
            .iter()
            .map(|p| p.as_ref().to_string_lossy().to_string())
            .collect();

        let mut existing_set = HashSet::new();
        for chunk in string_paths.chunks(SQL_IN_CLAUSE_CHUNK_SIZE) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(", ");
            let query = format!(
                "SELECT file_path FROM tracks WHERE file_path IN ({})",
                placeholders
            );
            let mut rows = self
                .conn
                .query(&query, turso::params_from_iter(chunk.iter().cloned()))
                .await?;
            while let Some(row) = rows.next().await? {
                let p: String = row.get(0)?;
                existing_set.insert(p);
            }
        }

        let mut new_paths = Vec::new();
        for path in paths {
            let p_ref = path.as_ref();
            let p_str = p_ref.to_string_lossy();
            if !existing_set.contains(p_str.as_ref()) {
                new_paths.push(p_ref.to_path_buf());
            }
        }

        new_paths.sort();
        new_paths.dedup();
        Ok(new_paths)
    }

    /// Recursively scans candidate paths (files or directories) and indexes ONLY new supported audio files.
    ///
    /// Candidate files that are already indexed are skipped without extracting metadata,
    /// avoiding redundant FFmpeg demuxing and database writes.
    pub async fn scan_new_files<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        paths: &[P],
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        let mut candidate_files: Vec<PathBuf> = Vec::new();

        for p in paths {
            let path = p.as_ref();
            if path.is_dir() {
                for entry in WalkDir::new(path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path();
                    if entry_path.is_file() && is_supported_audio_path(entry_path) {
                        candidate_files.push(entry_path.to_path_buf());
                    }
                }
            } else if path.is_file() && is_supported_audio_path(path) {
                candidate_files.push(path.to_path_buf());
            }
        }

        candidate_files.sort();
        candidate_files.dedup();

        let new_files = self.filter_new_paths(&candidate_files).await?;
        let summary = self.index_files_parallel(&new_files, progress_cb).await?;

        info!(
            "New files scan finished: {} candidate files checked, {} new files scanned, {} tracks inserted, {} errors",
            candidate_files.len(),
            summary.scanned_files,
            summary.inserted_tracks,
            summary.failed_files
        );

        Ok(summary)
    }

    /// Recursively scan directories and index ONLY new supported audio files.
    pub async fn scan_directories_new<P: AsRef<Path>, F: FnMut(usize, usize)>(
        &self,
        dir_paths: &[P],
        progress_cb: Option<F>,
    ) -> Result<ScanSummary, DbError> {
        self.scan_new_files(dir_paths, progress_cb).await
    }

    /// Inserts a batch within a transaction, falling back to individual inserts
    /// (and counting failures) when the batched statement errors out.
    async fn flush_scan_batch(&self, batch: &[NewTrack], summary: &mut ScanSummary) {
        match self.insert_tracks_batch(batch).await {
            Ok(inserted) => {
                summary.inserted_tracks += inserted;
            }
            Err(e) => {
                warn!(
                    "Batch insertion failed for {} tracks: {}. Falling back to individual inserts...",
                    batch.len(),
                    e
                );
                for t in batch {
                    match self.insert_or_update_track(t).await {
                        Ok(_) => summary.inserted_tracks += 1,
                        Err(err) => {
                            warn!("Failed to insert track {:?}: {}", t.file_path, err);
                            summary.failed_files += 1;
                        }
                    }
                }
            }
        }
    }
}
