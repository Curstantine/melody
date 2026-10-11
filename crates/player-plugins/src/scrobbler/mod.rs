pub mod lastfm;
pub mod listenbrainz;

use serde::{Deserialize, Serialize};

pub use lastfm::{LastFmConfig, LastFmPlugin};
pub use listenbrainz::{ListenBrainzConfig, ListenBrainzPlugin};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScrobbleService {
    ListenBrainz,
    LastFm,
}

impl std::fmt::Display for ScrobbleService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScrobbleService::ListenBrainz => write!(f, "ListenBrainz"),
            ScrobbleService::LastFm => write!(f, "Last.fm"),
        }
    }
}
