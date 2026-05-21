use std::path::PathBuf;
use std::sync::OnceLock;

/// The application name, used to derive platform-specific data, config, cache,
/// and state directory paths.
pub const APP_NAME: &str = "Melody";

/// Lowercased form of [`APP_NAME`], for use in XDG-style paths on
/// Linux/FreeBSD and the macOS `~/.config` fallback.
pub const APP_NAME_LOWERCASE: &str = {
	assert!(!APP_NAME.is_empty(), "APP_NAME must not be empty");
	assert!(APP_NAME.as_bytes().is_ascii(), "APP_NAME must be ASCII");
	const BYTES: [u8; APP_NAME.len()] = {
		let mut bytes = [0u8; APP_NAME.len()];
		let mut i = 0;
		while i < APP_NAME.len() {
			assert!(
				APP_NAME.as_bytes()[i] != b'/' && APP_NAME.as_bytes()[i] != b'\\',
				"APP_NAME must not contain path separators",
			);
			assert!(
				APP_NAME.as_bytes()[i] >= 0x20,
				"APP_NAME must not contain control characters"
			);
			bytes[i] = APP_NAME.as_bytes()[i];
			i += 1;
		}
		bytes.make_ascii_lowercase();
		bytes
	};
	match std::str::from_utf8(&BYTES) {
		Ok(s) => s,
		Err(_) => unreachable!(),
	}
};

/// The resolved data directory, combining custom override or platform defaults.
/// This is set once and cached for subsequent calls.
/// On macOS, this is `~/Library/Application Support/Melody`.
/// On Linux/FreeBSD, this is `$XDG_DATA_HOME/melody`.
/// On Windows, this is `%LOCALAPPDATA%\Melody``.
static CURRENT_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// The resolved config directory, combining custom override or platform defaults.
/// This is set once and cached for subsequent calls.
/// On macOS, this is `~/.config/melody`.
/// On Linux/FreeBSD, this is `$XDG_CONFIG_HOME/melody`.
/// On Windows, this is `%APPDATA%\Melody`.
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn home_dir() -> &'static PathBuf {
	static HOME_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
	HOME_DIR.get_or_init(|| {
		if cfg!(test) {
			if cfg!(target_os = "macos") {
				PathBuf::from("/Users/melody")
			} else if cfg!(target_os = "windows") {
				PathBuf::from("C:\\Users\\melody")
			} else {
				PathBuf::from("/home/melody")
			}
		} else {
			dirs::home_dir().expect("failed to determine home directory")
		}
	})
}

pub fn config_dir() -> &'static PathBuf {
	CONFIG_DIR.get_or_init(|| {
		if cfg!(target_os = "windows") {
			dirs::config_dir()
				.expect("failed to determine RoamingAppData directory")
				.join(APP_NAME)
		} else if cfg!(any(target_os = "linux", target_os = "freebsd")) {
			if let Ok(flatpak_xdg_config) = std::env::var("FLATPAK_XDG_CONFIG_HOME") {
				flatpak_xdg_config.into()
			} else {
				dirs::config_dir().expect("failed to determine XDG_CONFIG_HOME directory")
			}
			.join(APP_NAME_LOWERCASE)
		} else {
			home_dir().join(".config").join(APP_NAME_LOWERCASE)
		}
	})
}

pub fn data_dir() -> &'static PathBuf {
	CURRENT_DATA_DIR.get_or_init(|| {
		if cfg!(target_os = "macos") {
			return home_dir().join("Library/Application Support").join(APP_NAME);
		}

		if cfg!(any(target_os = "linux", target_os = "freebsd")) {
			if let Ok(flatpak_xdg_data) = std::env::var("FLATPAK_XDG_DATA_HOME") {
				flatpak_xdg_data.into()
			} else {
				dirs::data_local_dir().expect("failed to determine XDG_DATA_HOME directory")
			}
			.join(APP_NAME_LOWERCASE)
		} else if cfg!(target_os = "windows") {
			dirs::data_local_dir()
				.expect("failed to determine LocalAppData directory")
				.join(APP_NAME)
		} else {
			config_dir().clone()
		}
	})
}

pub fn state_dir() -> &'static PathBuf {
	static STATE_DIR: OnceLock<PathBuf> = OnceLock::new();
	STATE_DIR.get_or_init(|| {
		if cfg!(target_os = "macos") {
			return home_dir().join(".local").join("state").join(APP_NAME);
		}

		if cfg!(any(target_os = "linux", target_os = "freebsd")) {
			if let Ok(flatpak_xdg_state) = std::env::var("FLATPAK_XDG_STATE_HOME") {
				flatpak_xdg_state.into()
			} else {
				dirs::state_dir().expect("failed to determine XDG_STATE_HOME directory")
			}
			.join(APP_NAME_LOWERCASE)
		} else {
			dirs::data_local_dir()
				.expect("failed to determine LocalAppData directory")
				.join(APP_NAME)
		}
	})
}

pub fn temp_dir() -> &'static PathBuf {
	static TEMP_DIR: OnceLock<PathBuf> = OnceLock::new();
	TEMP_DIR.get_or_init(|| {
		if cfg!(target_os = "macos") {
			return dirs::cache_dir()
				.expect("failed to determine cachesDirectory directory")
				.join(APP_NAME);
		}

		if cfg!(target_os = "windows") {
			return dirs::cache_dir()
				.expect("failed to determine LocalAppData directory")
				.join(APP_NAME);
		}

		if cfg!(any(target_os = "linux", target_os = "freebsd")) {
			return if let Ok(flatpak_xdg_cache) = std::env::var("FLATPAK_XDG_CACHE_HOME") {
				flatpak_xdg_cache.into()
			} else {
				dirs::cache_dir().expect("failed to determine XDG_CACHE_HOME directory")
			}
			.join(APP_NAME_LOWERCASE);
		}

		home_dir().join(".cache").join(APP_NAME_LOWERCASE)
	})
}

pub fn logs_dir() -> &'static PathBuf {
	static LOGS_DIR: OnceLock<PathBuf> = OnceLock::new();
	LOGS_DIR.get_or_init(|| {
		if cfg!(target_os = "macos") {
			home_dir().join("Library/Logs").join(APP_NAME)
		} else {
			data_dir().join("logs")
		}
	})
}

pub fn log_file() -> &'static PathBuf {
	static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();
	LOG_FILE.get_or_init(|| logs_dir().join(format!("{}.log", APP_NAME)))
}

pub fn database_dir() -> &'static PathBuf {
	static DATABASE_DIR: OnceLock<PathBuf> = OnceLock::new();
	DATABASE_DIR.get_or_init(|| data_dir().join("db"))
}

pub fn settings_file() -> &'static PathBuf {
	static SETTINGS_FILE: OnceLock<PathBuf> = OnceLock::new();
	SETTINGS_FILE.get_or_init(|| config_dir().join("settings.json"))
}

pub fn extensions_dir() -> &'static PathBuf {
	static EXTENSIONS_DIR: OnceLock<PathBuf> = OnceLock::new();
	EXTENSIONS_DIR.get_or_init(|| data_dir().join("extensions"))
}

pub fn themes_dir() -> &'static PathBuf {
	static THEMES_DIR: OnceLock<PathBuf> = OnceLock::new();
	THEMES_DIR.get_or_init(|| config_dir().join("themes"))
}
