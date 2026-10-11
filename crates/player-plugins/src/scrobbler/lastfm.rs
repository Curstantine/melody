use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::manager::Plugin;
use crate::types::{PluginMetadata, TrackMetadata};

pub const DEFAULT_LASTFM_API_URL: &str = "https://ws.audioscrobbler.com/2.0/";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastFmConfig {
    pub enabled: bool,
    pub api_key: String,
    pub api_secret: String,
    pub session_key: String,
    pub api_url: String,
    pub submit_now_playing: bool,
    pub scrobble_percent: u32,
}

impl Default for LastFmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            api_secret: String::new(),
            session_key: String::new(),
            api_url: DEFAULT_LASTFM_API_URL.to_string(),
            submit_now_playing: true,
            scrobble_percent: 50,
        }
    }
}

pub fn calculate_lastfm_signature(params: &[(&str, &str)], secret: &str) -> String {
    let mut sorted = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut sig_base = String::new();
    for (key, val) in sorted {
        if key != "format" && key != "api_sig" {
            sig_base.push_str(key);
            sig_base.push_str(val);
        }
    }
    sig_base.push_str(secret);

    let mut hasher = Md5::new();
    hasher.update(sig_base.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

pub struct LastFmPlugin {
    meta: PluginMetadata,
    config: Arc<RwLock<LastFmConfig>>,
    current_track: Arc<RwLock<Option<TrackMetadata>>>,
    track_start_time: Arc<RwLock<Option<u64>>>,
    scrobbled: Arc<AtomicBool>,
    scrobble_count: Arc<AtomicU64>,
    last_status: Arc<RwLock<String>>,
}

impl LastFmPlugin {
    pub fn new(config: LastFmConfig) -> Self {
        Self {
            meta: PluginMetadata {
                id: "builtin.lastfm".to_string(),
                name: "Last.fm Scrobbler".to_string(),
                version: "1.0.0".to_string(),
                author: "vibecore".to_string(),
                description: "Submits playback history and now playing status to Last.fm"
                    .to_string(),
            },
            config: Arc::new(RwLock::new(config)),
            current_track: Arc::new(RwLock::new(None)),
            track_start_time: Arc::new(RwLock::new(None)),
            scrobbled: Arc::new(AtomicBool::new(false)),
            scrobble_count: Arc::new(AtomicU64::new(0)),
            last_status: Arc::new(RwLock::new("Ready".to_string())),
        }
    }

    pub fn update_config(&self, config: LastFmConfig) {
        if let Ok(mut cfg) = self.config.write() {
            *cfg = config;
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.enabled = enabled;
        }
    }

    pub fn set_api_key(&self, key: String) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.api_key = key;
        }
    }

    pub fn set_api_secret(&self, secret: String) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.api_secret = secret;
        }
    }

    pub fn set_session_key(&self, session_key: String) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.session_key = session_key;
        }
    }

    pub fn set_api_url(&self, url: String) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.api_url = url;
        }
    }

    pub fn set_submit_now_playing(&self, submit: bool) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.submit_now_playing = submit;
        }
    }

    pub fn set_scrobble_percent(&self, percent: u32) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.scrobble_percent = percent;
        }
    }

    pub fn config(&self) -> LastFmConfig {
        self.config
            .read()
            .map(|c| c.clone())
            .unwrap_or_default()
    }

    pub fn status(&self) -> String {
        self.last_status
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    pub fn scrobble_count(&self) -> u64 {
        self.scrobble_count.load(Ordering::Relaxed)
    }

    fn set_status(&self, status: String) {
        if let Ok(mut s) = self.last_status.write() {
            *s = status;
        }
    }

    /// Verifies authentication with Last.fm using user.getInfo
    pub fn test_connection(&self) -> Result<String, String> {
        let (api_key, api_secret, session_key, api_url) = {
            let cfg = self.config.read().map_err(|e| e.to_string())?;
            (
                cfg.api_key.trim().to_string(),
                cfg.api_secret.trim().to_string(),
                cfg.session_key.trim().to_string(),
                cfg.api_url.clone(),
            )
        };

        if api_key.is_empty() {
            return Err("Last.fm API Key is missing. Please enter your API Key.".to_string());
        }
        if session_key.is_empty() {
            return Err("Last.fm Session Key (sk) is missing. Please enter your Session Key.".to_string());
        }

        let mut params = vec![
            ("method", "user.getInfo"),
            ("api_key", api_key.as_str()),
            ("sk", session_key.as_str()),
        ];

        let signature = calculate_lastfm_signature(&params, &api_secret);
        params.push(("api_sig", signature.as_str()));
        params.push(("format", "json"));

        info!("Testing Last.fm authentication against {}", api_url);

        let response = ureq::post(&api_url)
            .send_form(params)
            .map_err(|e| format!("Last.fm request failed: {}", e))?;

        let body_str = response
            .into_body()
            .read_to_string()
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
            if let Some(user) = val.get("user") {
                let username = user
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("User");
                let playcount = user
                    .get("playcount")
                    .and_then(|p| p.as_str())
                    .unwrap_or("0");
                let msg = format!("Authenticated as '{}' (Total scrobbles: {})", username, playcount);
                self.set_status(format!("Connected as {}", username));
                return Ok(msg);
            } else if let Some(err_msg) = val.get("message").and_then(|m| m.as_str()) {
                return Err(format!("Last.fm returned error: {}", err_msg));
            }
        }

        Ok("Last.fm authentication verified".to_string())
    }

    pub fn submit_now_playing(&self, track: &TrackMetadata) {
        let cfg = self.config();
        if !cfg.enabled || cfg.api_key.trim().is_empty() || cfg.session_key.trim().is_empty() {
            return;
        }

        let api_key = cfg.api_key.trim().to_string();
        let api_secret = cfg.api_secret.trim().to_string();
        let session_key = cfg.session_key.trim().to_string();
        let api_url = cfg.api_url.clone();
        let track_clone = track.clone();
        let status_arc = self.last_status.clone();

        std::thread::spawn(move || {
            let mut params = vec![
                ("method", "track.updateNowPlaying"),
                ("artist", track_clone.artist.as_str()),
                ("track", track_clone.title.as_str()),
                ("api_key", api_key.as_str()),
                ("sk", session_key.as_str()),
            ];

            let album_str = track_clone.album.clone();
            if !album_str.is_empty() {
                params.push(("album", album_str.as_str()));
            }

            let sig = calculate_lastfm_signature(&params, &api_secret);
            params.push(("api_sig", sig.as_str()));
            params.push(("format", "json"));

            match ureq::post(&api_url).send_form(params) {
                Ok(_) => {
                    info!(
                        "[Last.fm] Now playing submitted: {} - {}",
                        track_clone.artist, track_clone.title
                    );
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Playing now: {} - {}", track_clone.artist, track_clone.title);
                    }
                }
                Err(err) => {
                    warn!("[Last.fm] Failed to submit playing now: {}", err);
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Now playing error: {}", err);
                    }
                }
            }
        });
    }

    pub fn submit_scrobble(&self, track: &TrackMetadata, timestamp: u64) {
        let cfg = self.config();
        if !cfg.enabled || cfg.api_key.trim().is_empty() || cfg.session_key.trim().is_empty() {
            return;
        }

        let api_key = cfg.api_key.trim().to_string();
        let api_secret = cfg.api_secret.trim().to_string();
        let session_key = cfg.session_key.trim().to_string();
        let api_url = cfg.api_url.clone();
        let track_clone = track.clone();
        let count_arc = self.scrobble_count.clone();
        let status_arc = self.last_status.clone();
        let timestamp_str = timestamp.to_string();

        std::thread::spawn(move || {
            let mut params = vec![
                ("method", "track.scrobble"),
                ("artist", track_clone.artist.as_str()),
                ("track", track_clone.title.as_str()),
                ("timestamp", timestamp_str.as_str()),
                ("api_key", api_key.as_str()),
                ("sk", session_key.as_str()),
            ];

            let album_str = track_clone.album.clone();
            if !album_str.is_empty() {
                params.push(("album", album_str.as_str()));
            }

            let sig = calculate_lastfm_signature(&params, &api_secret);
            params.push(("api_sig", sig.as_str()));
            params.push(("format", "json"));

            match ureq::post(&api_url).send_form(params) {
                Ok(_) => {
                    info!(
                        "[Last.fm] Scrobbled track: {} - {}",
                        track_clone.artist, track_clone.title
                    );
                    count_arc.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Scrobbled: {} - {}", track_clone.artist, track_clone.title);
                    }
                }
                Err(err) => {
                    error!("[Last.fm] Failed to scrobble: {}", err);
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Scrobble error: {}", err);
                    }
                }
            }
        });
    }
}

impl Plugin for LastFmPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.meta
    }

    fn on_track_change(&self, track: &TrackMetadata) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Ok(mut current) = self.current_track.write() {
            *current = Some(track.clone());
        }
        if let Ok(mut start) = self.track_start_time.write() {
            *start = Some(now);
        }
        self.scrobbled.store(false, Ordering::SeqCst);

        let cfg = self.config();
        if cfg.submit_now_playing {
            self.submit_now_playing(track);
        }
        Ok(())
    }

    fn on_position_change(&self, position_ms: u64) -> Result<(), String> {
        let cfg = self.config();
        if !cfg.enabled || self.scrobbled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let track_opt = self.current_track.read().ok().and_then(|t| t.clone());
        if let Some(track) = track_opt {
            if track.duration_ms < 30_000 {
                return Ok(());
            }

            let pct = (cfg.scrobble_percent as u64).clamp(10, 100);
            let threshold = (track.duration_ms * pct / 100).min(240_000);
            if position_ms >= threshold {
                self.scrobbled.store(true, Ordering::SeqCst);
                let timestamp = self
                    .track_start_time
                    .read()
                    .ok()
                    .and_then(|t| *t)
                    .unwrap_or_else(|| {
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    });

                self.submit_scrobble(&track, timestamp);
            }
        }

        Ok(())
    }

    fn on_playback_end(&self) -> Result<(), String> {
        let cfg = self.config();
        if !cfg.enabled || self.scrobbled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let track_opt = self.current_track.read().ok().and_then(|t| t.clone());
        if let Some(track) = track_opt {
            if track.duration_ms >= 30_000 {
                self.scrobbled.store(true, Ordering::SeqCst);
                let timestamp = self
                    .track_start_time
                    .read()
                    .ok()
                    .and_then(|t| *t)
                    .unwrap_or_else(|| {
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    });

                self.submit_scrobble(&track, timestamp);
            }
        }

        Ok(())
    }

    fn scrobble(&self, track: &TrackMetadata, timestamp: u64) -> Result<(), String> {
        self.submit_scrobble(track, timestamp);
        Ok(())
    }

    fn update_now_playing(&self, track: &TrackMetadata) -> Result<(), String> {
        self.submit_now_playing(track);
        Ok(())
    }
}
