use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use gpui::{App, AssetSource, Global, SharedString};

use crate::schema::{Theme, ThemeFamily};

pub type LoadedThemes = Box<dyn AssetSource>;

#[derive(Default)]
struct GlobalThemeRegistry(Arc<ThemeRegistry>);

impl std::ops::DerefMut for GlobalThemeRegistry {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl std::ops::Deref for GlobalThemeRegistry {
	type Target = Arc<ThemeRegistry>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Global for GlobalThemeRegistry {}

struct State {
	themes: HashMap<SharedString, Arc<Theme>>,
	loaded: bool,
}

pub struct ThemeRegistry {
	state: RwLock<State>,
	assets: LoadedThemes,
}

impl Default for ThemeRegistry {
	fn default() -> Self {
		Self::new(Box::new(()))
	}
}

impl ThemeRegistry {
	pub fn global(cx: &App) -> Arc<Self> {
		cx.global::<GlobalThemeRegistry>().0.clone()
	}

	pub fn default_global(cx: &mut App) -> Arc<Self> {
		cx.default_global::<GlobalThemeRegistry>().0.clone()
	}

	pub fn try_global(cx: &mut App) -> Option<Arc<Self>> {
		cx.try_global::<GlobalThemeRegistry>().map(|t| t.0.clone())
	}

	pub(crate) fn set_global(assets: Box<dyn AssetSource>, cx: &mut App) {
		cx.set_global(GlobalThemeRegistry(Arc::new(ThemeRegistry::new(assets))));
	}

	pub fn assets(&self) -> &dyn AssetSource {
		self.assets.as_ref()
	}

	pub fn new(assets: LoadedThemes) -> Self {
		let registry = Self {
			state: RwLock::new(State {
				themes: HashMap::default(),
				loaded: false,
			}),
			assets,
		};

		// We're loading the Zed default theme, as we need a theme to be loaded
		// for tests.
		// registry.insert_families([crate::fallback_themes::zed_default_themes()]);

		registry
	}

	pub fn insert_themes(&self, themes: impl IntoIterator<Item = Theme>) {
		let mut state = self.state.write().unwrap();

		for theme in themes.into_iter() {
			state.themes.insert(theme.name.clone(), Arc::new(theme));
		}
	}

	pub fn insert_families(&self, families: impl IntoIterator<Item = ThemeFamily>) {
		for family in families.into_iter() {
			self.insert_themes(family.themes);
		}
	}
}
