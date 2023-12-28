use std::{fs::File, path::Path};

use crate::{
	errors::Result,
	models::temp::{TempTrackMeta, TempTrackResource},
};

pub fn read_track_meta(path: &Path) -> Result<(TempTrackMeta, TempTrackResource)> {
	let extension = path.extension().and_then(|s| s.to_str());
	let source = File::open(path)?;

	let path_str = path.to_str().unwrap();

	unimplemented!()
}
