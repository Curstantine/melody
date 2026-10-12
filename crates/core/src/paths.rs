use directories::ProjectDirs;
use std::path::PathBuf;

/// Returns the primary `ProjectDirs` object for Vibecore, if resolvable by the platform.
pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("moe", "curstantine", "vibecore")
}

/// Directory where application configuration files (e.g. `config.toml`, themes) reside.
///
/// On Linux: `~/.config/vibecore`
/// On macOS: `~/Library/Application Support/moe.curstantine.vibecore`
/// On Windows: `AppData/Roaming/curstantine/vibecore/config`
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
/// On Linux: `~/.local/share/vibecore`
/// On macOS: `~/Library/Application Support/moe.curstantine.vibecore`
/// On Windows: `AppData/Roaming/curstantine/vibecore/data`
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
/// On Linux: `~/.cache/vibecore`
/// On macOS: `~/Library/Caches/moe.curstantine.vibecore`
/// On Windows: `AppData/Local/curstantine/vibecore/cache`
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
