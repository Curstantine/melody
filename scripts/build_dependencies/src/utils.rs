use std::{fs, io::Result, path::PathBuf, process::Command};

pub fn mkdir(dir_name: &str) -> Result<()> {
	fs::create_dir(dir_name)
}

pub fn pwd() -> Result<PathBuf> {
	std::env::current_dir()
}

pub fn cd(dir_name: &str) -> Result<()> {
	std::env::set_current_dir(dir_name)
}

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

pub fn make_and_install(num_job: usize) -> Result<()> {
	println!("Building project with make...");
	Command::new("make").arg("-j").arg(num_job.to_string()).status()?;

	println!("Installing...");
	Command::new("make").arg("install").status()?;
	println!("Done");

	Ok(())
}
