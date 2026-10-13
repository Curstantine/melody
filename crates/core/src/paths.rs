use directories::ProjectDirs;
use std::path::PathBuf;

/// Returns the primary `ProjectDirs` object for Melody, if resolvable by the platform.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("moe", "curstantine", "melody")
}

/// Directory where application configuration files (e.g. `config.toml`, themes) reside.
///
/// On Linux: `~/.config/melody`
/// On macOS: `~/Library/Application Support/moe.curstantine.melody`
/// On Windows: `AppData/Roaming/curstantine/melody/config`
pub fn config_dir() -> PathBuf {
    if let Some(proj) = project_dirs() {
        let dir = proj.config_dir().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);
        dir
    } else {
        PathBuf::from(".")
    }
}

/// Directory where persistent application data (e.g. `library.db`) resides.
///
/// On Linux: `~/.local/share/melody`
/// On macOS: `~/Library/Application Support/moe.curstantine.melody`
/// On Windows: `AppData/Roaming/curstantine/melody/data`
pub fn data_dir() -> PathBuf {
    if let Some(proj) = project_dirs() {
        let dir = proj.data_dir().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);
        dir
    } else {
        PathBuf::from(".")
    }
}

/// Directory where ephemeral caches (e.g. downscaled cover art thumbnails) reside.
///
/// On Linux: `~/.cache/melody`
/// On macOS: `~/Library/Caches/moe.curstantine.melody`
/// On Windows: `AppData/Local/curstantine/melody/cache`
pub fn cache_dir() -> PathBuf {
    if let Some(proj) = project_dirs() {
        let dir = proj.cache_dir().to_path_buf();
        let _ = std::fs::create_dir_all(&dir);
        dir
    } else {
        PathBuf::from(".cache")
    }
}

/// Path to the main application configuration file (`config.toml`).
pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// Path to the SQLite / libSQL library database file (`library.db`).
pub fn database_file() -> PathBuf {
    data_dir().join("library.db")
}

/// Directory where cached album artwork thumbnails are stored.
pub fn thumbnail_cache_dir() -> PathBuf {
    let dir = cache_dir().join("thumbnails");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// User directory where custom runtime TOML themes are placed.
pub fn user_themes_dir() -> PathBuf {
    let dir = config_dir().join("themes");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Directory where Melody-managed `.m3u8` playlist files are stored.
///
/// External playlists may be referenced from anywhere on disk; this directory
/// holds only the playlists the application itself creates or saves.
pub fn playlists_dir() -> PathBuf {
    let dir = data_dir().join("playlists");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
