use std::fmt::Write;

use md5::{Digest, Md5};
use melody_extension_sdk::{
    host, register_extension, Extension, HttpMethod, HttpRequest, SettingsMap, TrackInfo,
};

pub const DEFAULT_LASTFM_API_URL: &str = "https://ws.audioscrobbler.com/2.0/";

fn calculate_lastfm_signature(params: &[(&str, &str)], secret: &str) -> String {
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
    let mut hex = String::with_capacity(result.len() * 2);
    for b in result {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

pub struct LastFmExtension {
    api_key: String,
    api_secret: String,
    session_key: String,
    api_url: String,
    submit_now_playing: bool,
    scrobble_percent: u32,
    current_track: Option<TrackInfo>,
    track_start_time: Option<u64>,
    scrobbled: bool,
    scrobble_count: u64,
}

impl Default for LastFmExtension {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_secret: String::new(),
            session_key: String::new(),
            api_url: DEFAULT_LASTFM_API_URL.to_string(),
            submit_now_playing: true,
            scrobble_percent: 50,
            current_track: None,
            track_start_time: None,
            scrobbled: false,
            scrobble_count: 0,
        }
    }
}

impl LastFmExtension {
    fn apply_settings(&mut self, settings: &SettingsMap) {
        if let Some(key) = settings.get_str("api_key") {
            self.api_key = key.to_string();
        }
        if let Some(secret) = settings.get_str("api_secret") {
            self.api_secret = secret.to_string();
        }
        if let Some(sk) = settings.get_str("session_key") {
            self.session_key = sk.to_string();
        }
        if let Some(url) = settings.get_str("api_url") {
            let trimmed = url.trim();
            self.api_url = if trimmed.is_empty() {
                DEFAULT_LASTFM_API_URL.to_string()
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

    fn send_form(&self, mut params: Vec<(String, String)>, secret: &str) -> Result<String, String> {
        let borrowed: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let sig = calculate_lastfm_signature(&borrowed, secret);
        params.push(("api_sig".to_string(), sig));
        params.push(("format".to_string(), "json".to_string()));

        let req = HttpRequest {
            method: HttpMethod::Post,
            url: self.api_url.clone(),
            headers: Vec::new(),
            body: None,
            form: Some(params),
            timeout_ms: Some(10_000),
        };

        let resp = host::http_request(&req)?;
        if resp.status >= 400 {
            return Err(format!(
                "Last.fm returned HTTP {}: {}",
                resp.status, resp.body
            ));
        }
        Ok(resp.body)
    }

    fn submit_now_playing(&self, track: &TrackInfo) {
        let api_key = self.api_key.trim();
        let session_key = self.session_key.trim();
        if api_key.is_empty() || session_key.is_empty() {
            return;
        }

        let mut params = vec![
            ("method".to_string(), "track.updateNowPlaying".to_string()),
            ("artist".to_string(), track.artist.clone()),
            ("track".to_string(), track.title.clone()),
            ("api_key".to_string(), api_key.to_string()),
            ("sk".to_string(), session_key.to_string()),
        ];
        if !track.album.is_empty() {
            params.push(("album".to_string(), track.album.clone()));
        }

        match self.send_form(params, self.api_secret.trim()) {
            Ok(_) => host::log_info(format!(
                "[Last.fm] Now playing submitted: {} - {}",
                track.artist, track.title
            )),
            Err(err) => host::log_warn(format!("[Last.fm] Failed to submit playing now: {err}")),
        }
    }

    fn submit_scrobble(&mut self, track: &TrackInfo, timestamp: u64) {
        let api_key = self.api_key.trim();
        let session_key = self.session_key.trim();
        if api_key.is_empty() || session_key.is_empty() {
            return;
        }

        let mut params = vec![
            ("method".to_string(), "track.scrobble".to_string()),
            ("artist".to_string(), track.artist.clone()),
            ("track".to_string(), track.title.clone()),
            ("timestamp".to_string(), timestamp.to_string()),
            ("api_key".to_string(), api_key.to_string()),
            ("sk".to_string(), session_key.to_string()),
        ];
        if !track.album.is_empty() {
            params.push(("album".to_string(), track.album.clone()));
        }

        match self.send_form(params, self.api_secret.trim()) {
            Ok(_) => {
                self.scrobble_count += 1;
                host::log_info(format!(
                    "[Last.fm] Scrobbled track: {} - {}",
                    track.artist, track.title
                ));
            }
            Err(err) => host::log_error(format!("[Last.fm] Failed to scrobble: {err}")),
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
        let api_key = self.api_key.trim();
        let api_secret = self.api_secret.trim();
        let session_key = self.session_key.trim();

        if api_key.is_empty() {
            return Err("Last.fm API Key is missing. Please enter your API Key.".to_string());
        }
        if session_key.is_empty() {
            return Err(
                "Last.fm Session Key (sk) is missing. Please enter your Session Key.".to_string(),
            );
        }

        host::log_info(format!(
            "Testing Last.fm authentication against {}",
            self.api_url
        ));

        let params = vec![
            ("method".to_string(), "user.getInfo".to_string()),
            ("api_key".to_string(), api_key.to_string()),
            ("sk".to_string(), session_key.to_string()),
        ];

        let body_str = self
            .send_form(params, api_secret)
            .map_err(|e| format!("Last.fm request failed: {e}"))?;

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body_str) {
            if let Some(user) = val.get("user") {
                let username = user.get("name").and_then(|n| n.as_str()).unwrap_or("User");
                let playcount = user
                    .get("playcount")
                    .and_then(|p| p.as_str())
                    .unwrap_or("0");
                return Ok(format!(
                    "Authenticated as '{username}' (Total scrobbles: {playcount})"
                ));
            } else if let Some(err_msg) = val.get("message").and_then(|m| m.as_str()) {
                return Err(format!("Last.fm returned error: {err_msg}"));
            }
        }

        Ok("Last.fm authentication verified".to_string())
    }
}

impl Extension for LastFmExtension {
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

register_extension!(LastFmExtension);

#[cfg(test)]
mod tests {
    use super::*;
    use melody_extension_sdk::{HttpResponse, SettingValue};

    #[test]
    fn test_lastfm_signature_and_lifecycle() {
        host::clear_mocks();
        host::set_mock_now_unix_ms(Some(1_700_000_000_000));

        let params = vec![
            ("method", "track.scrobble"),
            ("artist", "Pink Floyd"),
            ("track", "Time"),
            ("api_key", "mock_key"),
            ("sk", "mock_session"),
            ("timestamp", "1600000000"),
        ];
        let sig = calculate_lastfm_signature(&params, "mock_secret");
        assert_eq!(sig.len(), 32);

        host::set_mock_http_handler(|_req| {
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: "{}".to_string(),
            })
        });

        let mut settings = SettingsMap::new();
        settings.insert("api_key", SettingValue::String("api_key_test".to_string()));
        settings.insert(
            "api_secret",
            SettingValue::String("api_secret_test".to_string()),
        );
        settings.insert(
            "session_key",
            SettingValue::String("session_key_test".to_string()),
        );
        settings.insert(
            "api_url",
            SettingValue::String("http://127.0.0.1:9/2.0/".to_string()),
        );
        settings.insert("submit_now_playing", SettingValue::Bool(true));
        settings.insert("scrobble_percent", SettingValue::Integer(50));

        let mut ext = LastFmExtension::default();
        ext.init(settings).unwrap();

        let track = TrackInfo {
            title: "Comfortably Numb".to_string(),
            artist: "Pink Floyd".to_string(),
            album: "The Wall".to_string(),
            duration_ms: 380_000,
        };
        ext.on_track_change(&track).unwrap();
        ext.on_position_change(100_000).unwrap();
        assert_eq!(ext.scrobble_count, 0);

        ext.on_position_change(195_000).unwrap();
        assert_eq!(ext.scrobble_count, 1);

        host::clear_mocks();
    }

    #[test]
    fn test_lastfm_test_connection_action() {
        host::clear_mocks();
        let mut ext = LastFmExtension::default();

        let err = ext
            .on_action("test_connection", SettingsMap::new())
            .unwrap_err();
        assert!(err.contains("API Key"));

        host::set_mock_http_handler(|req| {
            let form = req.form.as_ref().expect("form body");
            assert!(form
                .iter()
                .any(|(k, v)| k == "method" && v == "user.getInfo"));
            assert!(form.iter().any(|(k, _)| k == "api_sig"));
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: r#"{"user":{"name":"bob","playcount":"1234"}}"#.to_string(),
            })
        });

        let mut settings = SettingsMap::new();
        settings.insert("api_key", SettingValue::String("k".to_string()));
        settings.insert("api_secret", SettingValue::String("s".to_string()));
        settings.insert("session_key", SettingValue::String("sk".to_string()));

        let msg = ext.on_action("test_connection", settings).unwrap();
        assert!(msg.contains("bob"));
        assert!(msg.contains("1234"));
        host::clear_mocks();
    }
}
