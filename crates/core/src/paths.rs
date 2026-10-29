use directories::ProjectDirs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Application name identifier used for storage directories and platform registration.
/// In debug mode (`cfg(debug_assertions)`), uses `"melody.debug"` to keep dev sessions
/// isolated from the production library, config, and state.
pub fn app_name() -> &'static str {
    if cfg!(debug_assertions) {
        "melody.debug"
    } else {
        "melody"
    }
}

/// Returns true if running in a debug build.
pub fn is_debug() -> bool {
    cfg!(debug_assertions)
}

/// Returns the primary `ProjectDirs` object for Melody, if resolvable by the platform.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("moe", "curstantine", app_name())
}

static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static STATE_DIR: OnceLock<PathBuf> = OnceLock::new();
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static THUMBNAIL_CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static USER_THEMES_DIR: OnceLock<PathBuf> = OnceLock::new();
static USER_LAYOUTS_DIR: OnceLock<PathBuf> = OnceLock::new();
static PLAYLISTS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Directory where application configuration files (e.g. `config.toml`, themes) reside.
///
/// On Linux: `~/.config/melody`
/// On macOS: `~/Library/Application Support/moe.curstantine.melody`
/// On Windows: `AppData/Roaming/curstantine/melody/config`
pub fn config_dir() -> PathBuf {
    CONFIG_DIR
        .get_or_init(|| {
            if let Some(proj) = project_dirs() {
                let dir = proj.config_dir().to_path_buf();
                let _ = std::fs::create_dir_all(&dir);
                dir
            } else {
                PathBuf::from(".")
            }
        })
        .clone()
}

/// Directory where persistent application data (e.g. `library.db`) resides.
///
/// On Linux: `~/.local/share/melody`
/// On macOS: `~/Library/Application Support/moe.curstantine.melody`
/// On Windows: `AppData/Roaming/curstantine/melody/data`
pub fn data_dir() -> PathBuf {
    DATA_DIR
        .get_or_init(|| {
            if let Some(proj) = project_dirs() {
                let dir = proj.data_dir().to_path_buf();
                let _ = std::fs::create_dir_all(&dir);
                dir
            } else {
                PathBuf::from(".")
            }
        })
        .clone()
}

/// Directory where persistent runtime state files (e.g. `state.toml`) reside.
///
/// On Linux: `~/.local/state/melody`
/// On macOS: `~/Library/Application Support/moe.curstantine.melody`
/// On Windows: `AppData/Roaming/curstantine/melody/data`
pub fn state_dir() -> PathBuf {
    STATE_DIR
        .get_or_init(|| {
            if let Some(proj) = project_dirs() {
                let dir = proj
                    .state_dir()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| proj.data_dir().to_path_buf());
                let _ = std::fs::create_dir_all(&dir);
                dir
            } else {
                PathBuf::from(".")
            }
        })
        .clone()
}

/// Directory where ephemeral caches (e.g. downscaled cover art thumbnails) reside.
///
/// On Linux: `~/.cache/melody`
/// On macOS: `~/Library/Caches/moe.curstantine.melody`
/// On Windows: `AppData/Local/curstantine/melody/cache`
pub fn cache_dir() -> PathBuf {
    CACHE_DIR
        .get_or_init(|| {
            if let Some(proj) = project_dirs() {
                let dir = proj.cache_dir().to_path_buf();
                let _ = std::fs::create_dir_all(&dir);
                dir
            } else {
                PathBuf::from(".cache")
            }
        })
        .clone()
}

/// Path to the main application configuration file (`config.toml`).
pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Path to the application runtime state file (`state.toml`).
pub fn state_file() -> PathBuf {
    state_dir().join("state.toml")
}

/// Path to the SQLite / libSQL library database file (`library.db`).
pub fn database_file() -> PathBuf {
    data_dir().join("library.db")
}

/// Directory where cached album artwork thumbnails are stored.
pub fn thumbnail_cache_dir() -> PathBuf {
    THUMBNAIL_CACHE_DIR
        .get_or_init(|| {
            let dir = cache_dir().join("thumbnails");
            let _ = std::fs::create_dir_all(&dir);
            dir
        })
        .clone()
}

/// User directory where custom runtime TOML themes are placed.
pub fn user_themes_dir() -> PathBuf {
    USER_THEMES_DIR
        .get_or_init(|| {
            let dir = config_dir().join("themes");
            let _ = std::fs::create_dir_all(&dir);
            dir
        })
        .clone()
}

/// User directory where custom runtime KDL layouts are placed.
pub fn user_layouts_dir() -> PathBuf {
    USER_LAYOUTS_DIR
        .get_or_init(|| {
            let dir = config_dir().join("layouts");
            let _ = std::fs::create_dir_all(&dir);
            dir
        })
        .clone()
}

/// Directory where Melody-managed `.m3u8` playlist files are stored.
///
/// External playlists may be referenced from anywhere on disk; this directory
/// holds only the playlists the application itself creates or saves.
pub fn playlists_dir() -> PathBuf {
    PLAYLISTS_DIR
        .get_or_init(|| {
            let dir = data_dir().join("playlists");
            let _ = std::fs::create_dir_all(&dir);
            dir
        })
        .clone()
}

/// Atomically writes content to the target path by writing to a temporary file
/// in the same directory and renaming it over the destination.
pub fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp_path, content)?;

    #[cfg(windows)]
    {
        if std::fs::rename(&tmp_path, path).is_err() {
            let _ = std::fs::remove_file(path);
            std::fs::rename(&tmp_path, path)?;
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(&tmp_path, path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_name_and_debug_isolation() {
        if cfg!(debug_assertions) {
            assert_eq!(app_name(), "melody.debug");
            assert!(is_debug());
        } else {
            assert_eq!(app_name(), "melody");
            assert!(!is_debug());
        }

        if let Some(proj) = project_dirs() {
            let path_str = proj.config_dir().to_string_lossy();
            assert!(
                path_str.contains(app_name()),
                "config dir {path_str} should contain {}",
                app_name()
            );
        }
    }
}
