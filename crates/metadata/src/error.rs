use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "ffmpeg")]
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] ffmpeg_next::Error),
    #[cfg(feature = "symphonia")]
    #[error("Symphonia error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("Unsupported audio format or no tracks found")]
    UnsupportedFormat,
}
