use std::{env, fs, io::Result, path::Path};

use utils::{configure, make_and_install};

use crate::{
	constants::FFMPEG_BRANCH,
	utils::{fetch_and_checkout_origin, initialize_ffmpeg},
};

mod constants;
mod utils;

fn main() -> Result<()> {
	let cwd = env::current_dir().expect("CWD is not set");
	let root = cwd.join("target/build_dependencies/");

	fs::create_dir_all(&root)?;
	env::set_current_dir(&root)?;

	let build_path = root.join("ffmpeg_build");
	let branch = std::env::args().nth(1).unwrap_or_else(|| FFMPEG_BRANCH.to_string());
	let num_job = std::thread::available_parallelism().unwrap().get();

	initialize_ffmpeg(&branch)?;
	fetch_and_checkout_origin(&branch)?;

	configure(Path::new("./configure"), &build_path)?;
	make_and_install(num_job)?;

	Ok(())
}
