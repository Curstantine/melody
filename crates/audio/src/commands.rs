use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum AudioCommand {
    /// Load and optionally start playing a track
    Load { path: PathBuf, autoplay: bool },
    /// Resume playback
    Play,
    /// Pause playback
    Pause,
    /// Toggle play / pause state
    TogglePlayPause,
    /// Stop playback and reset position to 0
    Stop,
    /// Seek to a duration within the currently loaded track
    Seek(Duration),
    /// Set master volume (0.0 to 1.5)
    SetVolume(f32),
    /// Preload the next track for gapless audio playback
    SetNextTrack(Option<PathBuf>),
    /// Update audio resampling configuration parameters
    SetResampling(crate::resample::ResamplingConfig),
}
