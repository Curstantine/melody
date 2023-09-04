use std::fs::File;

use symphonia::core::{formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint};

use crate::errors::Result;

pub fn read_track_meta(source: Box<File>, extension: Option<&str>) -> Result<()> {
	let mss = MediaSourceStream::new(source, Default::default());
	let meta_opts: MetadataOptions = Default::default();
	let fmt_opts: FormatOptions = Default::default();
	let mut hint = Hint::new();

	if let Some(ext) = extension {
		hint.with_extension(ext);
	}

	let mut probed = symphonia::default::get_probe()
		.format(&hint, mss, &fmt_opts, &meta_opts)
		.expect("unsupported format");

	if let Some(metadata_rev) = probed.format.metadata().current() {
		println!("{:#?}", metadata_rev.tags());
	}

	Ok(())
}
