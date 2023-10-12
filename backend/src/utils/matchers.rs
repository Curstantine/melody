pub mod reg {
	use once_cell::sync::Lazy;
	use regex::Regex;

	pub fn matches_ymd(source: &str) -> bool {
		static YYYYMMDD: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap());
		YYYYMMDD.is_match(source)
	}

	pub fn matches_year(source: &str) -> bool {
		static YYYY: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d{4}").unwrap());
		YYYY.is_match(source)
	}
}

pub mod path {
	use std::path::Path;

	pub fn audio(path: &Path) -> bool {
		matches!(
			path.extension(),
			Some(extension) if crate::constants::SUPPORTED_AUDIO_EXTENSIONS.contains(&extension.to_str().unwrap())
		)
	}
}
