#[cfg(all(feature = "ffmpeg", feature = "symphonia"))]
compile_error!("features `ffmpeg` and `symphonia` are mutually exclusive");

fn main() {
	tauri_build::build();
}
