use melody_extension_sdk::{
    host, register_extension, Extension, HttpMethod, HttpRequest, SettingsMap, TrackInfo,
};
use serde_json::json;

pub const DEFAULT_LISTENBRAINZ_API_URL: &str = "https://api.listenbrainz.org/1/submit-listens";

pub struct ListenBrainzExtension {
    user_token: String,
    api_url: String,
    submit_now_playing: bool,
    scrobble_percent: u32,
    current_track: Option<TrackInfo>,
    track_start_time: Option<u64>,
    scrobbled: bool,
    scrobble_count: u64,
}

impl Default for ListenBrainzExtension {
    fn default() -> Self {
        Self {
            user_token: String::new(),
            api_url: DEFAULT_LISTENBRAINZ_API_URL.to_string(),
            submit_now_playing: true,
            scrobble_percent: 50,
            current_track: None,
            track_start_time: None,
            scrobbled: false,
            scrobble_count: 0,
        }
    }
}

impl ListenBrainzExtension {
    fn apply_settings(&mut self, settings: &SettingsMap) {
        if let Some(token) = settings.get_str("user_token") {
            self.user_token = token.to_string();
        }
        if let Some(url) = settings.get_str("api_url") {
            let trimmed = url.trim();
            self.api_url = if trimmed.is_empty() {
                DEFAULT_LISTENBRAINZ_API_URL.to_string()
            } else {
                trimmed.to_string()
            };
        }
        if let Some(submit) = settings.get_bool("submit_now_playing") {
            self.submit_now_playing = submit;
        }
        if let Some(pct) = settings.get_u32("scrobble_percent") {
            self.scrobble_percent = pct.clamp(10, 100);
        }
    }

    fn submit_now_playing(&self, track: &TrackInfo) {
        let token = self.user_token.trim();
        if token.is_empty() {
            return;
        }

        let payload = json!({
            "listen_type": "playing_now",
            "payload": [
                {
                    "track_metadata": {
                        "artist_name": track.artist,
                        "track_name": track.title,
                        "release_name": track.album,
                        "additional_info": {
                            "media_player": "Melody",
                            "submission_client": "Melody"
                        }
                    }
                }
            ]
        });

        let req = HttpRequest {
            method: HttpMethod::Post,
            url: self.api_url.clone(),
            headers: vec![
                ("Authorization".to_string(), format!("Token {token}")),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(payload.to_string()),
            form: None,
            timeout_ms: Some(10_000),
        };

        match host::http_request(&req) {
            Ok(resp) if resp.status < 400 => {
                host::log_info(format!(
                    "[ListenBrainz] Now playing submitted: {} - {}",
                    track.artist, track.title
                ));
            }
            Ok(resp) => {
                host::log_warn(format!(
                    "[ListenBrainz] Now playing returned HTTP {}: {}",
                    resp.status, resp.body
                ));
            }
            Err(err) => {
                host::log_warn(format!(
                    "[ListenBrainz] Failed to submit playing now: {err}"
                ));
            }
        }
    }

    fn submit_scrobble(&mut self, track: &TrackInfo, timestamp: u64) {
        let token = self.user_token.trim();
        if token.is_empty() {
            return;
        }

        let payload = json!({
            "listen_type": "single",
            "payload": [
                {
                    "listened_at": timestamp,
                    "track_metadata": {
                        "artist_name": track.artist,
                        "track_name": track.title,
                        "release_name": track.album,
                        "additional_info": {
                            "media_player": "Melody",
                            "submission_client": "Melody",
                            "duration_ms": track.duration_ms
                        }
                    }
                }
            ]
        });

        let req = HttpRequest {
            method: HttpMethod::Post,
            url: self.api_url.clone(),
            headers: vec![
                ("Authorization".to_string(), format!("Token {token}")),
                ("Content-Type".to_string(), "application/json".to_string()),
            ],
            body: Some(payload.to_string()),
            form: None,
            timeout_ms: Some(10_000),
        };

        match host::http_request(&req) {
            Ok(resp) if resp.status < 400 => {
                self.scrobble_count += 1;
                host::log_info(format!(
                    "[ListenBrainz] Scrobbled track: {} - {}",
                    track.artist, track.title
                ));
            }
            Ok(resp) => {
                host::log_error(format!(
                    "[ListenBrainz] Scrobble returned HTTP {}: {}",
                    resp.status, resp.body
                ));
            }
            Err(err) => {
                host::log_error(format!("[ListenBrainz] Failed to scrobble: {err}"));
            }
        }
    }

    fn maybe_scrobble(&mut self) {
        if self.scrobbled {
            return;
        }
        let Some(track) = self.current_track.clone() else {
            return;
        };
        if track.duration_ms < 30_000 {
            return;
        }
        self.scrobbled = true;
        let timestamp = self.track_start_time.unwrap_or_else(host::now_unix_secs);
        self.submit_scrobble(&track, timestamp);
    }

    fn test_connection(&self) -> Result<String, String> {
        let token = self.user_token.trim();
        if token.is_empty() {
            return Err("User token is empty. Please enter your ListenBrainz token.".to_string());
        }

        let base_url = if let Some(idx) = self.api_url.find("/1/") {
            &self.api_url[..idx]
        } else {
            "https://api.listenbrainz.org"
        };
        let validate_url = format!("{base_url}/1/validate-token?token={token}");

        host::log_info(format!(
            "Validating ListenBrainz token against {validate_url}"
        ));

        let req = HttpRequest {
            method: HttpMethod::Get,
            url: validate_url,
            headers: Vec::new(),
            body: None,
            form: None,
            timeout_ms: Some(10_000),
        };

        let resp = host::http_request(&req).map_err(|e| format!("Network request failed: {e}"))?;
        if resp.status >= 400 {
            return Err(format!("ListenBrainz returned HTTP {}", resp.status));
        }

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp.body) {
            if val.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
                let username = val
                    .get("user_name")
                    .and_then(|u| u.as_str())
                    .unwrap_or("User");
                return Ok(format!("Token is valid for user '{username}'"));
            } else {
                let msg = val
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Token is invalid");
                return Err(format!("Validation failed: {msg}"));
            }
        }

        Ok("Token validation completed successfully".to_string())
    }
}

impl Extension for ListenBrainzExtension {
    fn init(&mut self, settings: SettingsMap) -> Result<(), String> {
        self.apply_settings(&settings);
        Ok(())
    }

    fn on_settings_change(&mut self, settings: SettingsMap) -> Result<(), String> {
        self.apply_settings(&settings);
        Ok(())
    }

    fn on_action(&mut self, action_id: &str, settings: SettingsMap) -> Result<String, String> {
        self.apply_settings(&settings);
        match action_id {
            "test_connection" => self.test_connection(),
            _ => Err(format!("Unknown action '{action_id}'")),
        }
    }

    fn on_track_change(&mut self, track: &TrackInfo) -> Result<(), String> {
        self.current_track = Some(track.clone());
        self.track_start_time = Some(host::now_unix_secs());
        self.scrobbled = false;

        if self.submit_now_playing {
            self.submit_now_playing(track);
        }
        Ok(())
    }

    fn on_position_change(&mut self, position_ms: u64) -> Result<(), String> {
        if self.scrobbled {
            return Ok(());
        }
        let Some(ref track) = self.current_track else {
            return Ok(());
        };
        if track.duration_ms < 30_000 {
            return Ok(());
        }

        let pct = u64::from(self.scrobble_percent.clamp(10, 100));
        let threshold = (track.duration_ms * pct / 100).min(240_000);
        if position_ms >= threshold {
            self.maybe_scrobble();
        }
        Ok(())
    }

    fn on_playback_end(&mut self) -> Result<(), String> {
        self.maybe_scrobble();
        Ok(())
    }
}

register_extension!(ListenBrainzExtension);

#[cfg(test)]
mod tests {
    use super::*;
    use melody_extension_sdk::{HttpResponse, SettingValue};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_listenbrainz_lifecycle_and_thresholds() {
        host::clear_mocks();
        host::set_mock_now_unix_ms(Some(1_700_000_000_000));

        let requests: Rc<RefCell<Vec<HttpRequest>>> = Rc::new(RefCell::new(Vec::new()));
        let req_log = requests.clone();
        host::set_mock_http_handler(move |req| {
            req_log.borrow_mut().push(req.clone());
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: r#"{"status":"ok"}"#.to_string(),
            })
        });

        let mut settings = SettingsMap::new();
        settings.insert(
            "user_token",
            SettingValue::String("test_token_123".to_string()),
        );
        settings.insert(
            "api_url",
            SettingValue::String("http://127.0.0.1:9/1/submit-listens".to_string()),
        );
        settings.insert("submit_now_playing", SettingValue::Bool(true));
        settings.insert("scrobble_percent", SettingValue::Integer(50));

        let mut ext = ListenBrainzExtension::default();
        ext.init(settings).expect("init");

        let short_track = TrackInfo {
            title: "Short Intro".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 20_000,
        };
        ext.on_track_change(&short_track).unwrap();
        ext.on_position_change(15_000).unwrap();
        ext.on_playback_end().unwrap();
        assert_eq!(ext.scrobble_count, 0);

        let long_track = TrackInfo {
            title: "Standard Song".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 200_000,
        };
        ext.on_track_change(&long_track).unwrap();
        ext.on_position_change(50_000).unwrap();
        assert_eq!(ext.scrobble_count, 0);

        ext.on_position_change(105_000).unwrap();
        assert_eq!(ext.scrobble_count, 1);

        // Two playing_now + one single scrobble
        assert_eq!(requests.borrow().len(), 3);
        host::clear_mocks();
    }

    #[test]
    fn test_listenbrainz_test_connection_action() {
        host::clear_mocks();
        let mut ext = ListenBrainzExtension::default();

        // Empty token returns validation error without HTTP call
        let err = ext
            .on_action("test_connection", SettingsMap::new())
            .unwrap_err();
        assert!(err.contains("empty"));

        host::set_mock_http_handler(|req| {
            assert!(req.url.contains("/1/validate-token?token=secret_tok"));
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: r#"{"valid":true,"user_name":"alice"}"#.to_string(),
            })
        });

        let mut settings = SettingsMap::new();
        settings.insert("user_token", SettingValue::String("secret_tok".to_string()));
        let msg = ext
            .on_action("test_connection", settings)
            .expect("valid token");
        assert!(msg.contains("alice"));
        host::clear_mocks();
    }
}
