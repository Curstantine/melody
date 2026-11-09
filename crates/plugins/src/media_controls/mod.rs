#[cfg(target_os = "linux")]
pub mod mpris;
#[cfg(target_os = "windows")]
pub mod windows_media;

#[cfg(target_os = "linux")]
pub use mpris::MprisPlugin;
#[cfg(target_os = "windows")]
pub use windows_media::WindowsMediaPlugin;
