use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use directories::ProjectDirs;

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
#[inline]
pub fn is_debug() -> bool {
    cfg!(debug_assertions)
}

/// Returns the primary `ProjectDirs` object for Melody, if resolvable by the platform.
pub fn project_dirs() -> Option<&'static ProjectDirs> {
    app_paths().project_dirs.as_ref()
}

/// Resolved and prepared filesystem paths for Melody.
pub struct AppPaths {
    pub project_dirs: Option<ProjectDirs>,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
    pub database_file: PathBuf,
    pub thumbnail_cache_dir: PathBuf,
    pub user_themes_dir: PathBuf,
    pub user_layouts_dir: PathBuf,
    pub playlists_dir: PathBuf,
}

impl AppPaths {
    fn initialize() -> Self {
        let proj = ProjectDirs::from("moe", "curstantine", app_name());

        let (config_dir, data_dir, state_dir, cache_dir) = if let Some(ref p) = proj {
            let cfg = p.config_dir().to_path_buf();
            let data = p.data_dir().to_path_buf();
            let state = p
                .state_dir()
                .map(|s| s.to_path_buf())
                .unwrap_or_else(|| p.data_dir().to_path_buf());
            let cache = p.cache_dir().to_path_buf();
            (cfg, data, state, cache)
        } else {
            (
                PathBuf::from("."),
                PathBuf::from("."),
                PathBuf::from("."),
                PathBuf::from(".cache"),
            )
        };

        let config_file = config_dir.join("config.toml");
        let state_file = state_dir.join("state.toml");
        let database_file = data_dir.join("library.db");
        let thumbnail_cache_dir = cache_dir.join("thumbnails");
        let user_themes_dir = config_dir.join("themes");
        let user_layouts_dir = config_dir.join("layouts");
        let playlists_dir = data_dir.join("playlists");

        // Ensure directories exist
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = std::fs::create_dir_all(&data_dir);
        let _ = std::fs::create_dir_all(&state_dir);
        let _ = std::fs::create_dir_all(&cache_dir);
        let _ = std::fs::create_dir_all(&thumbnail_cache_dir);
        let _ = std::fs::create_dir_all(&user_themes_dir);
        let _ = std::fs::create_dir_all(&user_layouts_dir);
        let _ = std::fs::create_dir_all(&playlists_dir);

        Self {
            project_dirs: proj,
            config_dir,
            data_dir,
            state_dir,
            cache_dir,
            config_file,
            state_file,
            database_file,
            thumbnail_cache_dir,
            user_themes_dir,
            user_layouts_dir,
            playlists_dir,
        }
    }
}

static APP_PATHS: OnceLock<AppPaths> = OnceLock::new();

/// Returns the globally resolved [`AppPaths`] instance, initializing on first access.
pub fn app_paths() -> &'static AppPaths {
    APP_PATHS.get_or_init(AppPaths::initialize)
}

/// Directory where application configuration files (e.g. `config.toml`, themes) reside.
pub fn config_dir() -> &'static Path {
    &app_paths().config_dir
}

/// Directory where persistent application data (e.g. `library.db`) resides.
pub fn data_dir() -> &'static Path {
    &app_paths().data_dir
}

/// Directory where persistent runtime state files (e.g. `state.toml`) reside.
pub fn state_dir() -> &'static Path {
    &app_paths().state_dir
}

/// Directory where ephemeral caches (e.g. downscaled cover art thumbnails) reside.
pub fn cache_dir() -> &'static Path {
    &app_paths().cache_dir
}

/// Path to the main application configuration file (`config.toml`).
pub fn config_file() -> &'static Path {
    &app_paths().config_file
}

/// Path to the application runtime state file (`state.toml`).
pub fn state_file() -> &'static Path {
    &app_paths().state_file
}

/// Path to the SQLite / libSQL library database file (`library.db`).
pub fn database_file() -> &'static Path {
    &app_paths().database_file
}

/// Directory where cached album artwork thumbnails are stored.
pub fn thumbnail_cache_dir() -> &'static Path {
    &app_paths().thumbnail_cache_dir
}

/// User directory where custom runtime TOML themes are placed.
pub fn user_themes_dir() -> &'static Path {
    &app_paths().user_themes_dir
}

/// User directory where custom runtime KDL layouts are placed.
pub fn user_layouts_dir() -> &'static Path {
    &app_paths().user_layouts_dir
}

/// Returns the bundled themes directory if found.
pub fn bundled_themes_dir() -> Option<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(windows)]
            {
                let candidate = exe_dir.join("../themes");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }

            #[cfg(target_os = "macos")]
            {
                let candidate = exe_dir.join("../Resources/themes");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }

            #[cfg(not(any(windows, target_os = "macos")))]
            {
                let candidate = exe_dir.join("../share/melody/themes");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in xdg_dirs.split(':') {
                let path = Path::new(dir).join("melody").join("themes");
                if path.is_dir() {
                    return Some(path);
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        for rel in &[
            Path::new("themes"),
            Path::new("../themes"),
            Path::new("../../themes"),
        ] {
            if rel.is_dir() {
                return Some(rel.to_path_buf());
            }
        }
    }

    None
}

/// Returns the bundled layouts directory if found.
pub fn bundled_layouts_dir() -> Option<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(windows)]
            {
                let candidate = exe_dir.join("../layouts");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }

            #[cfg(target_os = "macos")]
            {
                let candidate = exe_dir.join("../Resources/layouts");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }

            #[cfg(not(any(windows, target_os = "macos")))]
            {
                let candidate = exe_dir.join("../share/melody/layouts");
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let xdg_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
        for dir in xdg_dirs.split(':') {
            let path = Path::new(dir).join("melody").join("layouts");
            if path.is_dir() {
                return Some(path);
            }
        }
    }

    #[cfg(debug_assertions)]
    {
        for rel in &[
            Path::new("layouts"),
            Path::new("../layouts"),
            Path::new("../../layouts"),
        ] {
            if rel.is_dir() {
                return Some(rel.to_path_buf());
            }
        }
    }

    None
}

/// Directory where Melody-managed `.m3u8` playlist files are stored.
pub fn playlists_dir() -> &'static Path {
    &app_paths().playlists_dir
}

/// Atomically writes content to the target path by writing to a temporary file
/// in the same directory and renaming it over the destination.
pub fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let ext = path.extension().map_or("tmp".to_string(), |x| {
        format!("{}.{}.tmp", x.to_string_lossy(), std::process::id())
    });
    let tmp_path = path.with_extension(ext);
    std::fs::write(&tmp_path, content)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

static APP_START_TIME: OnceLock<std::time::Instant> = OnceLock::new();

/// Record the startup instant if not already set, returning the start time.
pub fn init_app_start_time() -> std::time::Instant {
    *APP_START_TIME.get_or_init(std::time::Instant::now)
}

/// Duration elapsed since `init_app_start_time` was first invoked.
pub fn elapsed_since_app_start() -> std::time::Duration {
    APP_START_TIME
        .get()
        .map(|t| t.elapsed())
        .unwrap_or_default()
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

    #[test]
    fn test_bundled_paths_discovery() {
        assert!(bundled_themes_dir().is_some());
        assert!(bundled_layouts_dir().is_some());
    }
}
