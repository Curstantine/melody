use std::{env::set_current_dir, fs, io::Result, path::Path, process::Command};

use crate::constants::FFMPEG_BUILD_FEATURES;

pub fn initialize_ffmpeg(branch: &str) -> Result<()> {
	if fs::metadata("ffmpeg").is_err() {
		println!("Cloning ffmpeg repository...");

		Command::new("git")
			.arg("clone")
			.arg("--single-branch")
			.arg("--branch")
			.arg(branch)
			.arg("--depth")
			.arg("1")
			.arg("https://github.com/ffmpeg/ffmpeg")
			.status()?;
	} else {
		println!("Found ffmpeg repository");
	}

	set_current_dir("ffmpeg")?;

	Ok(())
}

pub fn fetch_and_checkout_origin(branch: &str) -> Result<()> {
	Command::new("git")
		.arg("fetch")
		.arg("origin")
		.arg(branch)
		.arg("--depth")
		.arg("1")
		.status()?;

	Command::new("git").arg("checkout").arg("FETCH_HEAD").status()?;

	Ok(())
}

pub fn configure(path: &Path, build_path: &Path) -> Result<()> {
	println!("Configuring ffmpeg with platform options...");

	#[cfg(unix)]
	Command::new(path)
		.arg(format!("--prefix={:?}", build_path))
		.arg("--enable-gpl")
		.args(FFMPEG_BUILD_FEATURES)
		// To workaround `https://github.com/larksuite/rsmpeg/pull/98#issuecomment-1467511193`
		.arg("--disable-decoder=exr,phm")
		.arg("--disable-programs")
		.arg("--enable-nonfree")
		.status()?;

	#[cfg(windows)]
	Command::new(path)
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

	println!("Finished configuring");

	Ok(())
}

pub fn make_and_install(num_job: usize) -> Result<()> {
	println!("Building project with make...");
	Command::new("make").arg("-j").arg(num_job.to_string()).status()?;

	println!("Installing...");
	Command::new("make").arg("install").status()?;
	println!("Done");

	Ok(())
}
