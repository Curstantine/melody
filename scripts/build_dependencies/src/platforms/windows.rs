use std::{io::Result, process::Command};

use crate::{
	constants::FFMPEG_BUILD_FEATURES,
	utils::{cd, fetch_and_checkout_origin, initialize_ffmpeg, make_and_install, mkdir, pwd},
};

pub fn run() -> Result<()> {
	let _ = mkdir("tmp");

	cd("tmp")?;

	let tmp_path = pwd()?.to_string_lossy().to_string();
	let build_path = format!("{}/ffmpeg_build", tmp_path);
	let branch = std::env::args().nth(1).unwrap_or_else(|| "release/6.1".to_string());
	let num_job = std::thread::available_parallelism().unwrap().get();

	initialize_ffmpeg(&branch)?;
	cd("ffmpeg")?;
	fetch_and_checkout_origin(&branch)?;

	Command::new("./configure")
		.arg(format!("--prefix={}", build_path))
		.arg("--enable-gpl")
		.args(FFMPEG_BUILD_FEATURES)
		// To workaround `https://github.com/larksuite/rsmpeg/pull/98#issuecomment-1467511193`
		.arg("--disable-decoder=exr,phm")
		.arg("--disable-programs")
		.arg("--enable-nonfree")
		.arg("--arch=x86")
		.arg("--target-os=mingw32")
		.arg("--cross-prefix=i686-w64-mingw32-")
		.arg("--pkg-config=pkg-config")
		.status()?;

	make_and_install(num_job)?;

	cd("..")?;

	Ok(())
}
