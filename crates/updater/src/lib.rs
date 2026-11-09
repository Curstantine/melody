use std::process::Command;
use tracing::{info, warn};
use velopack::{sources::GithubSource, UpdateCheck, UpdateManager};

/// Initializes Velopack runtime hooks early in the application lifecycle.
pub fn init() {
    velopack::VelopackApp::build().run();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub release_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Available(UpdateInfo),
    Downloading {
        info: UpdateInfo,
        percent: i16,
    },
    ReadyToRestart {
        info: UpdateInfo,
    },
    UpToDate,
    Failed(String),
}

/// Checks for updates using Velopack's native GithubSource and UpdateManager.
/// If the application is not running from a Velopack installation, update checking is skipped.
pub async fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    tokio::task::spawn_blocking(check_for_updates_blocking)
        .await
        .map_err(|e| format!("Task join error: {e}"))?
}

fn check_for_updates_blocking() -> Result<Option<UpdateInfo>, String> {
    let repo_url = env!("CARGO_PKG_REPOSITORY");
    let current_version_str = env!("CARGO_PKG_VERSION");

    info!("Checking for updates via Velopack from '{repo_url}' (current version: v{current_version_str})...");

    let source = GithubSource::new(repo_url, None, false);
    let um = match UpdateManager::new(source, None, None) {
        Ok(um) => um,
        Err(e) => {
            info!("Update check skipped (not running from a Velopack installation: {e})");
            return Ok(None);
        }
    };

    match um.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(info)) => {
            let version = info.TargetFullRelease.Version.to_string();
            let release_url = format!("{repo_url}/releases/tag/v{version}");
            info!("Velopack update available: v{version} (URL: {release_url})");
            Ok(Some(UpdateInfo {
                version: format!("v{version}"),
                release_url,
            }))
        }
        Ok(UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty) => {
            info!("Velopack confirmed app is up to date");
            Ok(None)
        }
        Err(e) => {
            warn!("Velopack update check error: {e}");
            Err(e.to_string())
        }
    }
}

/// Downloads available updates using Velopack, streaming progress (0..100) to `progress_cb`.
pub fn download_update<F: FnMut(i16) + Send + 'static>(mut progress_cb: F) -> Result<(), String> {
    let repo_url = env!("CARGO_PKG_REPOSITORY");
    info!("Downloading update via Velopack from '{repo_url}'...");

    let source = GithubSource::new(repo_url, None, false);
    let um = UpdateManager::new(source, None, None).map_err(|e| e.to_string())?;

    match um.check_for_updates().map_err(|e| e.to_string())? {
        UpdateCheck::UpdateAvailable(updates) => {
            let (tx, rx) = std::sync::mpsc::channel();
            let handle = std::thread::spawn(move || {
                while let Ok(progress) = rx.recv() {
                    progress_cb(progress);
                }
            });

            um.download_updates(&updates, Some(tx))
                .map_err(|e| e.to_string())?;
            let _ = handle.join();
            info!("Velopack update download completed");
            Ok(())
        }
        UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => {
            Err("No update available to download".to_string())
        }
    }
}

/// Applies downloaded updates and restarts the application.
pub fn apply_update_and_restart() -> Result<(), String> {
    let repo_url = env!("CARGO_PKG_REPOSITORY");
    info!("Applying Velopack update and restarting...");

    let source = GithubSource::new(repo_url, None, false);
    let um = UpdateManager::new(source, None, None).map_err(|e| e.to_string())?;

    match um.check_for_updates().map_err(|e| e.to_string())? {
        UpdateCheck::UpdateAvailable(updates) => {
            um.apply_updates_and_restart(&*updates)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => {
            Err("No update package available to apply".to_string())
        }
    }
}

/// Opens a web URL in the user's default desktop browser across platforms.
pub fn open_in_browser(url: &str) {
    info!("Opening URL in browser: {url}");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // 0x08000000 = CREATE_NO_WINDOW
        let _ = Command::new("cmd")
            .args(["/c", "start", "", url])
            .creation_flags(0x08000000)
            .spawn();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_status_default() {
        assert_eq!(UpdateStatus::default(), UpdateStatus::Idle);
    }
}
