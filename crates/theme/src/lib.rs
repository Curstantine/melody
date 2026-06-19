use gpui::{App, AssetSource};

use crate::registry::{LoadedThemes, ThemeRegistry};

pub mod registry;
pub mod schema;

pub const DEFAULT_THEME: &str = "Dark";

pub fn init(cx: &mut App, to_load: Option<LoadedThemes>) {
	let themes = to_load.unwrap_or(Box::new(()) as Box<dyn AssetSource>);
	let _theme = ThemeRegistry::new(themes);
}
