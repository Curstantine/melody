use anyhow::Context as _;
use gpui::{AssetSource, Result, SharedString};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../assets"]
#[include = "icons/**/*"]
#[include = "themes/**/*"]
#[include = "sounds/**/*"]
pub struct Assets;

impl AssetSource for Assets {
	fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
		Self::get(path)
			.map(|f| Some(f.data))
			.with_context(|| format!("loading asset at path {path:?}"))
	}

	fn list(&self, path: &str) -> Result<Vec<SharedString>> {
		Ok(Self::iter()
			.filter_map(|p| if p.starts_with(path) { Some(p.into()) } else { None })
			.collect())
	}
}
