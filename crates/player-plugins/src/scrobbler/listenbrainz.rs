use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, warn};

use crate::manager::Plugin;
use crate::types::{PluginMetadata, TrackMetadata};

pub const DEFAULT_LISTENBRAINZ_API_URL: &str = "https://api.listenbrainz.org/1/submit-listens";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListenBrainzConfig {
    pub enabled: bool,
    pub user_token: String,
    pub api_url: String,
    pub submit_now_playing: bool,
    pub scrobble_percent: u32,
}

impl Default for ListenBrainzConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            user_token: String::new(),
            api_url: DEFAULT_LISTENBRAINZ_API_URL.to_string(),
            submit_now_playing: true,
            scrobble_percent: 50,
        }
    }
}

pub struct ListenBrainzPlugin {
    meta: PluginMetadata,
    config: Arc<RwLock<ListenBrainzConfig>>,
    current_track: Arc<RwLock<Option<TrackMetadata>>>,
    track_start_time: Arc<RwLock<Option<u64>>>,
    scrobbled: Arc<AtomicBool>,
    scrobble_count: Arc<AtomicU64>,
    last_status: Arc<RwLock<String>>,
}

impl ListenBrainzPlugin {
    pub fn new(config: ListenBrainzConfig) -> Self {
        Self {
            meta: PluginMetadata {
                id: "builtin.listenbrainz".to_string(),
                name: "ListenBrainz Scrobbler".to_string(),
                version: "1.0.0".to_string(),
                author: "vibecore".to_string(),
                description: "Submits playback history and now playing status to ListenBrainz"
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

    pub fn update_config(&self, config: ListenBrainzConfig) {
        if let Ok(mut cfg) = self.config.write() {
            *cfg = config;
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.enabled = enabled;
        }
    }

    pub fn set_user_token(&self, token: String) {
        if let Ok(mut cfg) = self.config.write() {
            cfg.user_token = token;
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

    pub fn config(&self) -> ListenBrainzConfig {
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

    /// Validates the user token against the ListenBrainz API endpoint
    pub fn test_connection(&self) -> Result<String, String> {
        let (token, api_url) = {
            let cfg = self.config.read().map_err(|e| e.to_string())?;
            (cfg.user_token.trim().to_string(), cfg.api_url.clone())
        };

        if token.is_empty() {
            return Err("User token is empty. Please enter your ListenBrainz token.".to_string());
        }

        // Derive base URL to call validate-token endpoint
        let base_url = if let Some(idx) = api_url.find("/1/") {
            &api_url[..idx]
        } else {
            "https://api.listenbrainz.org"
        };
        let validate_url = format!("{}/1/validate-token?token={}", base_url, token);

        info!("Validating ListenBrainz token against {}", validate_url);

        let response = ureq::get(&validate_url)
            .call()
            .map_err(|e| format!("Network request failed: {}", e))?;

        let body_str = response
            .into_body()
            .read_to_string()
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
            if val.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
                let username = val
                    .get("user_name")
                    .and_then(|u| u.as_str())
                    .unwrap_or("User");
                let msg = format!("Token is valid for user '{}'", username);
                self.set_status(format!("Connected as {}", username));
                return Ok(msg);
            } else {
                let msg = val
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Token is invalid");
                return Err(format!("Validation failed: {}", msg));
            }
        }

        Ok("Token validation completed successfully".to_string())
    }

    pub fn submit_now_playing(&self, track: &TrackMetadata) {
        let cfg = self.config();
        if !cfg.enabled || cfg.user_token.trim().is_empty() {
            return;
        }

        let token = cfg.user_token.trim().to_string();
        let api_url = cfg.api_url.clone();
        let track_clone = track.clone();
        let status_arc = self.last_status.clone();

        std::thread::spawn(move || {
            let payload = json!({
                "listen_type": "playing_now",
                "payload": [
                    {
                        "track_metadata": {
                            "artist_name": track_clone.artist,
                            "track_name": track_clone.title,
                            "release_name": track_clone.album,
                            "additional_info": {
                                "media_player": "Vibecore",
                                "submission_client": "Vibecore"
                            }
                        }
                    }
                ]
            });

            match ureq::post(&api_url)
                .header("Authorization", &format!("Token {}", token))
                .send_json(&payload)
            {
                Ok(_) => {
                    info!(
                        "[ListenBrainz] Now playing submitted: {} - {}",
                        track_clone.artist, track_clone.title
                    );
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Playing now: {} - {}", track_clone.artist, track_clone.title);
                    }
                }
                Err(err) => {
                    warn!("[ListenBrainz] Failed to submit playing now: {}", err);
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Now playing error: {}", err);
                    }
                }
            }
        });
    }

    pub fn submit_scrobble(&self, track: &TrackMetadata, timestamp: u64) {
        let cfg = self.config();
        if !cfg.enabled || cfg.user_token.trim().is_empty() {
            return;
        }

        let token = cfg.user_token.trim().to_string();
        let api_url = cfg.api_url.clone();
        let track_clone = track.clone();
        let count_arc = self.scrobble_count.clone();
        let status_arc = self.last_status.clone();

        std::thread::spawn(move || {
            let payload = json!({
                "listen_type": "single",
                "payload": [
                    {
                        "listened_at": timestamp,
                        "track_metadata": {
                            "artist_name": track_clone.artist,
                            "track_name": track_clone.title,
                            "release_name": track_clone.album,
                            "additional_info": {
                                "media_player": "Vibecore",
                                "submission_client": "Vibecore",
                                "duration_ms": track_clone.duration_ms
                            }
                        }
                    }
                ]
            });

            match ureq::post(&api_url)
                .header("Authorization", &format!("Token {}", token))
                .send_json(&payload)
            {
                Ok(_) => {
                    info!(
                        "[ListenBrainz] Scrobbled track: {} - {}",
                        track_clone.artist, track_clone.title
                    );
                    count_arc.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Scrobbled: {} - {}", track_clone.artist, track_clone.title);
                    }
                }
                Err(err) => {
                    error!("[ListenBrainz] Failed to scrobble: {}", err);
                    if let Ok(mut s) = status_arc.write() {
                        *s = format!("Scrobble error: {}", err);
                    }
                }
            }
        });
    }
}

impl Plugin for ListenBrainzPlugin {
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
            // Must be at least 30 seconds long
            if track.duration_ms < 30_000 {
                return Ok(());
            }

            // Scrobble condition: percentage (capped at 4 minutes)
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
