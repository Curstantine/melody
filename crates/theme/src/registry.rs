use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use gpui::{AssetSource, SharedString};

use crate::schema::Theme;

pub type LoadedThemes = Box<dyn AssetSource>;

struct State {
	themes: HashMap<SharedString, Arc<Theme>>,
	loaded: bool,
}

pub struct ThemeRegistry {
	state: RwLock<State>,
	assets: LoadedThemes,
}
