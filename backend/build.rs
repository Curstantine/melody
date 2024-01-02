use std::{env, path::Path};
#[cfg(all(feature = "ffmpeg", feature = "symphonia"))]
compile_error!("features `ffmpeg` and `symphonia` are mutually exclusive");

fn setup_ffmpeg(project_root: &Path) {
	let ffmpeg_build = project_root
		.join("./ffmpeg_build/lib/pkgconfig")
		.canonicalize()
		.unwrap();

	if !ffmpeg_build.exists() {
		panic!(
			"ffmpeg build does not exist at {:#?}, try running the build_dependencies script!",
			ffmpeg_build,
		)
	}

	let path_str = ffmpeg_build.to_string_lossy();
	env::set_var("FFMPEG_PKG_CONFIG_PATH", path_str.to_string());
}

fn main() {
	let project_root = env::current_dir().expect("No CWD set");

	#[cfg(feature = "ffmpeg")]
	setup_ffmpeg(&project_root);

	tauri_build::build();
}
