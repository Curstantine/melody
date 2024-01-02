pub const FFMPEG_BRANCH: &str = "release/6.1";
pub const FFMPEG_BUILD_FEATURES: &[&str; 5] = &[
	"--enable-libmp3lame",
	"--enable-libopus",
	"--enable-libvorbis",
	"--disable-programs",
	// To workaround `https://github.com/larksuite/rsmpeg/pull/98#issuecomment-1467511193`
	"--disable-decoder=exr,phm",
];
