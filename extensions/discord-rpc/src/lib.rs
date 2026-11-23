use melody_extension_sdk::{host, register_extension, Extension, SettingsMap, TrackInfo};
use serde::Serialize;
use serde_json::{json, Value};

pub const BUNDLED_DISCORD_CLIENT_ID: Option<&str> = option_env!("MELODY_DISCORD_CLIENT_ID");

const OP_HANDSHAKE: u32 = 0;
const OP_FRAME: u32 = 1;
const OP_CLOSE: u32 = 2;
const OP_PING: u32 = 3;
const OP_PONG: u32 = 4;

const MAX_FRAME_LEN: usize = 1024 * 1024;
const MAX_PING_FRAMES: usize = 5;
const MAX_SKIP_FRAMES: usize = 8;
const RETRY_BACKOFF_MS: u64 = 15_000;
const TIMESTAMP_DRIFT_MS: u64 = 4_000;

const IPC_CANDIDATES: [&str; 40] = [
    "discord-ipc-0",
    "discord-ipc-1",
    "discord-ipc-2",
    "discord-ipc-3",
    "discord-ipc-4",
    "discord-ipc-5",
    "discord-ipc-6",
    "discord-ipc-7",
    "discord-ipc-8",
    "discord-ipc-9",
    "app/com.discordapp.Discord/discord-ipc-0",
    "app/com.discordapp.Discord/discord-ipc-1",
    "app/com.discordapp.Discord/discord-ipc-2",
    "app/com.discordapp.Discord/discord-ipc-3",
    "app/com.discordapp.Discord/discord-ipc-4",
    "app/com.discordapp.Discord/discord-ipc-5",
    "app/com.discordapp.Discord/discord-ipc-6",
    "app/com.discordapp.Discord/discord-ipc-7",
    "app/com.discordapp.Discord/discord-ipc-8",
    "app/com.discordapp.Discord/discord-ipc-9",
    "snap.discord/discord-ipc-0",
    "snap.discord/discord-ipc-1",
    "snap.discord/discord-ipc-2",
    "snap.discord/discord-ipc-3",
    "snap.discord/discord-ipc-4",
    "snap.discord/discord-ipc-5",
    "snap.discord/discord-ipc-6",
    "snap.discord/discord-ipc-7",
    "snap.discord/discord-ipc-8",
    "snap.discord/discord-ipc-9",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-0",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-1",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-2",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-3",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-4",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-5",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-6",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-7",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-8",
    ".flatpak/com.discordapp.Discord/xdg-run/discord-ipc-9",
];

const ACTIVITY_TYPE_LISTENING: u8 = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StatusDisplayType {
    #[default]
    Name,
    State,
    Details,
}

impl StatusDisplayType {
    const fn as_u8(self) -> u8 {
        match self {
            Self::Name => 0,
            Self::State => 1,
            Self::Details => 2,
        }
    }
}

#[derive(Serialize)]
struct DiscordTimestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<u64>,
}

#[derive(Serialize)]
struct DiscordAssets {
    #[serde(skip_serializing_if = "Option::is_none")]
    large_text: Option<String>,
}

#[derive(Serialize)]
struct DiscordActivity {
    #[serde(rename = "type")]
    activity_type: u8,
    status_display_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamps: Option<DiscordTimestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assets: Option<DiscordAssets>,
}

fn sanitize_discord_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let char_count = trimmed.chars().count();
    if char_count < 2 {
        return Some(format!("{trimmed}\u{200B}"));
    }
    if char_count > 128 {
        let mut truncated: String = trimmed.chars().take(127).collect();
        truncated.push('…');
        return Some(truncated);
    }
    Some(trimmed.to_string())
}

fn encode_frame(opcode: u32, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&opcode.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload);
    buf
}

fn ipc_read_exact(handle: u32, len: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let remaining = (len - out.len()) as u32;
        let chunk = host::ipc_read(handle, remaining)?;
        if chunk.is_empty() {
            return Err("Discord closed IPC connection".to_string());
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

fn read_frame(handle: u32) -> Result<(u32, Vec<u8>), String> {
    let header = ipc_read_exact(handle, 8)?;
    let opcode = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    if len > MAX_FRAME_LEN {
        return Err(format!(
            "Discord IPC frame length {len} exceeds {MAX_FRAME_LEN} bytes"
        ));
    }
    let payload = ipc_read_exact(handle, len)?;
    Ok((opcode, payload))
}

fn read_frame_handling_pings(handle: u32) -> Result<(u32, Vec<u8>), String> {
    for _ in 0..MAX_PING_FRAMES {
        let (opcode, payload) = read_frame(handle)?;
        if opcode == OP_PING {
            host::ipc_write(handle, &encode_frame(OP_PONG, &payload))?;
            continue;
        }
        return Ok((opcode, payload));
    }
    Err("Discord IPC sent too many consecutive ping frames".to_string())
}

fn parse_error_message(bytes: &[u8]) -> Option<String> {
    let val: Value = serde_json::from_slice(bytes).ok()?;
    val.pointer("/data/message")
        .or_else(|| val.get("message"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

enum IpcError {
    Transport(String),
    Protocol(String),
}

impl IpcError {
    fn into_message(self) -> String {
        match self {
            Self::Transport(msg) | Self::Protocol(msg) => msg,
        }
    }
}

struct IpcConnection {
    handle: u32,
    connected_user: Option<String>,
    nonce: u64,
}

impl IpcConnection {
    fn connect(client_id: &str) -> Result<Self, String> {
        let handle = host::ipc_connect(&IPC_CANDIDATES).map_err(|err| {
            host::log_debug(format!("[Discord RPC] ipc_connect error: {err}"));
            let lower = err.to_ascii_lowercase();
            if lower.contains("permission")
                || lower.contains("allowed_ipc")
                || lower.contains("maximum")
                || lower.contains("not supported")
            {
                err
            } else {
                "Could not find a running Discord desktop client".to_string()
            }
        })?;

        let handshake = json!({ "v": 1, "client_id": client_id });
        let payload = serde_json::to_vec(&handshake)
            .map_err(|e| format!("Failed to serialize Discord handshake: {e}"))?;

        if let Err(err) = host::ipc_write(handle, &encode_frame(OP_HANDSHAKE, &payload)) {
            host::ipc_close(handle);
            return Err(err);
        }

        let (opcode, resp_bytes) = match read_frame_handling_pings(handle) {
            Ok(res) => res,
            Err(err) => {
                host::ipc_close(handle);
                return Err(err);
            }
        };

        if opcode == OP_CLOSE {
            host::ipc_close(handle);
            let reason = parse_error_message(&resp_bytes)
                .unwrap_or_else(|| "Discord closed IPC connection".to_string());
            return Err(reason);
        }
        if opcode != OP_FRAME {
            host::ipc_close(handle);
            return Err(format!(
                "Unexpected Discord handshake opcode {opcode} (expected {OP_FRAME})"
            ));
        }

        let resp_val: Value = match serde_json::from_slice(&resp_bytes) {
            Ok(v) => v,
            Err(e) => {
                host::ipc_close(handle);
                return Err(format!("Invalid Discord handshake response: {e}"));
            }
        };

        if resp_val.get("evt").and_then(|v| v.as_str()) == Some("ERROR") {
            host::ipc_close(handle);
            let msg = resp_val
                .pointer("/data/message")
                .and_then(|v| v.as_str())
                .unwrap_or("Discord rejected handshake");
            return Err(format!("Discord IPC error: {msg}"));
        }

        if resp_val.get("evt").and_then(|v| v.as_str()) != Some("READY") {
            host::ipc_close(handle);
            return Err("Discord handshake did not return READY event".to_string());
        }

        let connected_user = resp_val
            .pointer("/data/user/global_name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                resp_val
                    .pointer("/data/user/username")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
            })
            .map(str::to_string);

        Ok(Self {
            handle,
            connected_user,
            nonce: 0,
        })
    }

    fn set_activity(&mut self, activity: Option<&DiscordActivity>) -> Result<(), IpcError> {
        self.nonce = self.nonce.wrapping_add(1);
        let expected_nonce = format!("melody-{}", self.nonce);
        let frame = json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": host::pid(),
                "activity": activity,
            },
            "nonce": expected_nonce,
        });
        let payload = serde_json::to_vec(&frame).map_err(|e| {
            IpcError::Protocol(format!("Failed to serialize Discord SET_ACTIVITY: {e}"))
        })?;
        host::ipc_write(self.handle, &encode_frame(OP_FRAME, &payload))
            .map_err(IpcError::Transport)?;

        for _ in 0..MAX_SKIP_FRAMES {
            let (opcode, resp_bytes) =
                read_frame_handling_pings(self.handle).map_err(IpcError::Transport)?;
            if opcode == OP_CLOSE {
                let reason = parse_error_message(&resp_bytes)
                    .unwrap_or_else(|| "Discord closed IPC connection".to_string());
                return Err(IpcError::Transport(reason));
            }
            if opcode != OP_FRAME {
                continue;
            }

            let resp_val: Value = serde_json::from_slice(&resp_bytes).map_err(|e| {
                IpcError::Transport(format!("Invalid Discord SET_ACTIVITY response: {e}"))
            })?;

            if resp_val.get("nonce").and_then(|v| v.as_str()) != Some(expected_nonce.as_str()) {
                continue;
            }

            if resp_val.get("evt").and_then(|v| v.as_str()) == Some("ERROR") {
                let msg = resp_val
                    .pointer("/data/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Discord rejected SET_ACTIVITY");
                return Err(IpcError::Protocol(format!("Discord IPC error: {msg}")));
            }

            return Ok(());
        }

        Err(IpcError::Transport(
            "Timed out waiting for Discord SET_ACTIVITY reply".to_string(),
        ))
    }

    fn close(self) {
        let _ = host::ipc_write(self.handle, &encode_frame(OP_CLOSE, b"{}"));
        host::ipc_close(self.handle);
    }
}

pub struct DiscordRpcExtension {
    status_display: StatusDisplayType,
    show_timestamps: bool,
    show_album: bool,
    show_when_paused: bool,
    current_track: Option<TrackInfo>,
    position_ms: u64,
    is_playing: bool,
    presence_synced: bool,
    last_sync_anchor_ms: u64,
    last_retry_ms: u64,
    warned_missing_client_id: bool,
    conn: Option<IpcConnection>,
    #[cfg(test)]
    test_client_id: Option<&'static str>,
}

impl Default for DiscordRpcExtension {
    fn default() -> Self {
        Self {
            status_display: StatusDisplayType::Name,
            show_timestamps: true,
            show_album: true,
            show_when_paused: false,
            current_track: None,
            position_ms: 0,
            is_playing: false,
            presence_synced: false,
            last_sync_anchor_ms: 0,
            last_retry_ms: 0,
            warned_missing_client_id: false,
            conn: None,
            #[cfg(test)]
            test_client_id: None,
        }
    }
}

impl DiscordRpcExtension {
    fn bundled_client_id(&self) -> Result<&'static str, String> {
        #[cfg(test)]
        if let Some(id) = self.test_client_id {
            return Ok(id);
        }
        let id = BUNDLED_DISCORD_CLIENT_ID.unwrap_or("").trim();
        if id.is_empty() {
            return Err(
                "Discord application ID is not bundled in this build (missing MELODY_DISCORD_CLIENT_ID)."
                    .to_string(),
            );
        }
        Ok(id)
    }

    fn apply_settings(&mut self, settings: &SettingsMap) {
        if let Some(display) = settings.get_str("status_display") {
            self.status_display = match display {
                "details" => StatusDisplayType::Details,
                "state" => StatusDisplayType::State,
                _ => StatusDisplayType::Name,
            };
        }
        if let Some(show) = settings.get_bool("show_timestamps") {
            self.show_timestamps = show;
        }
        if let Some(show) = settings.get_bool("show_album") {
            self.show_album = show;
        }
        if let Some(show) = settings.get_bool("show_when_paused") {
            self.show_when_paused = show;
        }
    }

    fn build_activity(&self, track: &TrackInfo, now_ms: u64) -> DiscordActivity {
        let title = track.title.trim();
        let details = sanitize_discord_text(if title.is_empty() {
            "Unknown Track"
        } else {
            title
        });

        let artist = track.artist.trim();
        let state = if self.is_playing {
            sanitize_discord_text(artist)
        } else if artist.is_empty() {
            Some("Paused".to_string())
        } else {
            sanitize_discord_text(&format!("{artist} (Paused)"))
        };

        let timestamps = (self.is_playing && self.show_timestamps).then(|| {
            let start_ms = now_ms.saturating_sub(self.position_ms);
            DiscordTimestamps {
                start: Some(start_ms / 1000),
                end: (track.duration_ms > 0)
                    .then(|| start_ms.saturating_add(track.duration_ms) / 1000),
            }
        });

        let assets = self
            .show_album
            .then(|| sanitize_discord_text(&track.album))
            .flatten()
            .map(|large_text| DiscordAssets {
                large_text: Some(large_text),
            });

        DiscordActivity {
            activity_type: ACTIVITY_TYPE_LISTENING,
            status_display_type: self.status_display.as_u8(),
            details,
            state,
            timestamps,
            assets,
        }
    }

    fn send_activity(&mut self, activity: Option<&DiscordActivity>) -> Result<(), String> {
        let client_id = self.bundled_client_id()?;

        if self.conn.is_none() {
            if activity.is_none() {
                return Ok(());
            }
            self.conn = Some(IpcConnection::connect(client_id)?);
        }

        if let Some(conn) = self.conn.as_mut() {
            match conn.set_activity(activity) {
                Ok(()) => return Ok(()),
                Err(IpcError::Protocol(msg)) => return Err(msg),
                Err(IpcError::Transport(_)) => {
                    if let Some(old) = self.conn.take() {
                        old.close();
                    }
                    if activity.is_none() {
                        return Ok(());
                    }
                }
            }
        }

        // Retry once on a fresh connection in case Discord restarted.
        let mut conn = IpcConnection::connect(client_id)?;
        if let Err(err) = conn.set_activity(activity) {
            conn.close();
            return Err(err.into_message());
        }
        self.conn = Some(conn);
        Ok(())
    }

    fn clear_presence(&mut self) {
        self.presence_synced = false;
        if let Err(err) = self.send_activity(None) {
            host::log_debug(format!("[Discord RPC] Failed to clear activity: {err}"));
        }
    }

    fn sync_presence(&mut self) {
        let Some(track) = self.current_track.clone() else {
            self.clear_presence();
            return;
        };

        if !self.is_playing && !self.show_when_paused {
            self.clear_presence();
            return;
        }

        if let Err(err) = self.bundled_client_id() {
            if !self.warned_missing_client_id {
                self.warned_missing_client_id = true;
                host::log_warn(format!("[Discord RPC] {err}"));
            }
            self.last_retry_ms = host::now_unix_ms();
            self.presence_synced = false;
            return;
        }
        self.warned_missing_client_id = false;

        let now_ms = host::now_unix_ms();
        let activity = self.build_activity(&track, now_ms);
        match self.send_activity(Some(&activity)) {
            Ok(()) => {
                self.presence_synced = true;
                self.last_sync_anchor_ms = now_ms.saturating_sub(self.position_ms);
            }
            Err(err) => {
                self.presence_synced = false;
                self.last_retry_ms = now_ms;
                host::log_debug(format!("[Discord RPC] Failed to set activity: {err}"));
            }
        }
    }

    fn test_connection(&mut self) -> Result<String, String> {
        let client_id = self.bundled_client_id()?;
        if let Some(old) = self.conn.take() {
            old.close();
        }
        let conn = IpcConnection::connect(client_id)?;
        let user = conn.connected_user.clone();
        self.conn = Some(conn);

        if self.current_track.is_some() && (self.is_playing || self.show_when_paused) {
            self.sync_presence();
        }

        match user {
            Some(name) if !name.trim().is_empty() => {
                Ok(format!("Connected to Discord as '{name}'"))
            }
            _ => Ok("Connected to Discord".to_string()),
        }
    }
}

impl Extension for DiscordRpcExtension {
    fn init(&mut self, settings: SettingsMap) -> Result<(), String> {
        self.apply_settings(&settings);
        Ok(())
    }

    fn on_settings_change(&mut self, settings: SettingsMap) -> Result<(), String> {
        self.apply_settings(&settings);
        self.sync_presence();
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
        self.position_ms = 0;
        self.is_playing = true;
        self.presence_synced = false;
        self.sync_presence();
        Ok(())
    }

    fn on_playback_status_change(&mut self, is_playing: bool) -> Result<(), String> {
        if self.is_playing == is_playing && self.presence_synced {
            return Ok(());
        }
        self.is_playing = is_playing;
        self.sync_presence();
        Ok(())
    }

    fn on_seeked(&mut self, position_ms: u64) -> Result<(), String> {
        self.position_ms = position_ms;
        if self.is_playing {
            self.sync_presence();
        }
        Ok(())
    }

    fn on_position_change(&mut self, position_ms: u64) -> Result<(), String> {
        self.position_ms = position_ms;
        if self.current_track.is_none() || !self.is_playing {
            return Ok(());
        }

        let now_ms = host::now_unix_ms();
        if !self.presence_synced {
            if now_ms.saturating_sub(self.last_retry_ms) >= RETRY_BACKOFF_MS {
                self.sync_presence();
            }
            return Ok(());
        }

        if self.show_timestamps {
            let current_anchor_ms = now_ms.saturating_sub(position_ms);
            if current_anchor_ms.abs_diff(self.last_sync_anchor_ms) > TIMESTAMP_DRIFT_MS {
                self.sync_presence();
            }
        }

        Ok(())
    }

    fn on_playback_end(&mut self) -> Result<(), String> {
        self.is_playing = false;
        self.current_track = None;
        self.position_ms = 0;
        self.clear_presence();
        Ok(())
    }
}

register_extension!(DiscordRpcExtension);

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;

    use super::*;
    use melody_extension_sdk::host::MockIpcTransport;
    use melody_extension_sdk::SettingValue;

    struct MockDiscordPipeState {
        fail_connect: bool,
        written_frames: Vec<(u32, Value)>,
        read_queue: VecDeque<u8>,
    }

    impl MockDiscordPipeState {
        fn new() -> Self {
            Self {
                fail_connect: false,
                written_frames: Vec::new(),
                read_queue: VecDeque::new(),
            }
        }

        fn push_frame(&mut self, opcode: u32, payload: Value) {
            let bytes = serde_json::to_vec(&payload).unwrap();
            self.read_queue.extend(encode_frame(opcode, &bytes));
        }
    }

    struct MockDiscordTransport(Rc<RefCell<MockDiscordPipeState>>);

    impl MockIpcTransport for MockDiscordTransport {
        fn connect(&mut self, _candidates: &[String]) -> Result<u32, String> {
            let mut st = self.0.borrow_mut();
            if st.fail_connect {
                return Err("Discord not running".to_string());
            }
            st.push_frame(
                OP_FRAME,
                json!({
                    "cmd": "DISPATCH",
                    "evt": "READY",
                    "data": {
                        "user": {
                            "username": "alice_user",
                            "global_name": "Alice"
                        }
                    }
                }),
            );
            Ok(1)
        }

        fn write(&mut self, _handle: u32, data: &[u8]) -> Result<(), String> {
            let opcode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            let len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
            let val: Value = serde_json::from_slice(&data[8..8 + len]).unwrap();
            let mut st = self.0.borrow_mut();
            let nonce = val.get("nonce").cloned().unwrap_or(Value::Null);
            st.written_frames.push((opcode, val));
            if opcode == OP_FRAME {
                st.push_frame(
                    OP_FRAME,
                    json!({"cmd": "SET_ACTIVITY", "evt": null, "nonce": nonce}),
                );
            }
            Ok(())
        }

        fn read(&mut self, _handle: u32, max_len: u32) -> Result<Vec<u8>, String> {
            let mut st = self.0.borrow_mut();
            let n = (max_len as usize).min(st.read_queue.len());
            Ok(st.read_queue.drain(..n).collect())
        }
    }

    fn sample_track() -> TrackInfo {
        TrackInfo {
            title: "Time".to_string(),
            artist: "Pink Floyd".to_string(),
            album: "The Dark Side of the Moon".to_string(),
            duration_ms: 413_000,
        }
    }

    #[test]
    fn test_sanitize_discord_text() {
        assert_eq!(sanitize_discord_text("   "), None);
        assert_eq!(sanitize_discord_text("A"), Some("A\u{200B}".to_string()));
        assert_eq!(sanitize_discord_text("  Time  "), Some("Time".to_string()));

        let long = "é".repeat(140);
        let sanitized = sanitize_discord_text(&long).unwrap();
        assert_eq!(sanitized.chars().count(), 128);
        assert!(sanitized.ends_with('…'));
    }

    fn test_ext() -> DiscordRpcExtension {
        DiscordRpcExtension {
            test_client_id: Some("123456789012345678"),
            ..Default::default()
        }
    }

    #[test]
    fn test_track_change_pause_resume_and_playback_end() {
        host::clear_mocks();
        host::set_mock_now_unix_ms(Some(1_700_000_000_000));

        let state = Rc::new(RefCell::new(MockDiscordPipeState::new()));
        host::set_mock_ipc_transport(MockDiscordTransport(Rc::clone(&state)));

        let mut ext = test_ext();
        ext.init(SettingsMap::new()).unwrap();

        ext.on_track_change(&sample_track()).unwrap();
        {
            let st = state.borrow();
            assert_eq!(st.written_frames.len(), 2);
            assert_eq!(st.written_frames[0].0, OP_HANDSHAKE);
            assert_eq!(st.written_frames[1].0, OP_FRAME);
            let act = &st.written_frames[1].1["args"]["activity"];
            assert_eq!(act["type"], 2);
            assert_eq!(act["details"], "Time");
            assert_eq!(act["state"], "Pink Floyd");
            assert_eq!(act["timestamps"]["start"], 1_700_000_000);
            assert_eq!(act["timestamps"]["end"], 1_700_000_413);
            assert_eq!(act["assets"]["large_text"], "The Dark Side of the Moon");
        }

        // Default show_when_paused = false clears presence on pause.
        ext.on_playback_status_change(false).unwrap();
        assert!(state.borrow().written_frames.last().unwrap().1["args"]["activity"].is_null());

        // Resuming restores presence.
        host::set_mock_now_unix_ms(Some(1_700_000_010_000));
        ext.on_position_change(10_000).unwrap();
        ext.on_playback_status_change(true).unwrap();
        assert_eq!(
            state.borrow().written_frames.last().unwrap().1["args"]["activity"]["details"],
            "Time"
        );

        // Playback end clears presence.
        ext.on_playback_end().unwrap();
        assert!(state.borrow().written_frames.last().unwrap().1["args"]["activity"].is_null());

        host::clear_mocks();
    }

    #[test]
    fn test_show_when_paused_and_settings_change() {
        host::clear_mocks();
        host::set_mock_now_unix_ms(Some(1_700_000_000_000));

        let state = Rc::new(RefCell::new(MockDiscordPipeState::new()));
        host::set_mock_ipc_transport(MockDiscordTransport(Rc::clone(&state)));

        let mut settings = SettingsMap::new();
        settings.insert("show_when_paused", SettingValue::Bool(true));
        settings.insert("show_album", SettingValue::Bool(false));
        settings.insert(
            "status_display",
            SettingValue::String("details".to_string()),
        );

        let mut ext = test_ext();
        ext.init(settings).unwrap();
        ext.on_track_change(&sample_track()).unwrap();

        ext.on_playback_status_change(false).unwrap();
        {
            let st = state.borrow();
            let act = &st.written_frames.last().unwrap().1["args"]["activity"];
            assert_eq!(act["type"], 2);
            assert_eq!(act["status_display_type"], 2);
            assert_eq!(act["state"], "Pink Floyd (Paused)");
            assert!(act.get("timestamps").is_none());
            assert!(act.get("assets").is_none());
        }

        // Changing settings while paused re-syncs immediately.
        let mut updated = SettingsMap::new();
        updated.insert("show_when_paused", SettingValue::Bool(false));
        ext.on_settings_change(updated).unwrap();
        assert!(state.borrow().written_frames.last().unwrap().1["args"]["activity"].is_null());

        host::clear_mocks();
    }

    #[test]
    fn test_seek_steady_position_and_retry_backoff() {
        host::clear_mocks();
        host::set_mock_now_unix_ms(Some(1_700_000_000_000));

        let state = Rc::new(RefCell::new(MockDiscordPipeState::new()));
        host::set_mock_ipc_transport(MockDiscordTransport(Rc::clone(&state)));

        let mut ext = test_ext();
        ext.init(SettingsMap::new()).unwrap();
        ext.on_track_change(&sample_track()).unwrap();
        assert_eq!(state.borrow().written_frames.len(), 2); // handshake + set_activity

        // Steady playback does not spam SetActivity.
        host::set_mock_now_unix_ms(Some(1_700_000_001_000));
        ext.on_position_change(1_000).unwrap();
        assert_eq!(state.borrow().written_frames.len(), 2);

        // Explicit seek updates timestamps immediately.
        host::set_mock_now_unix_ms(Some(1_700_000_002_000));
        ext.on_seeked(60_000).unwrap();
        {
            let st = state.borrow();
            assert_eq!(st.written_frames.len(), 3);
            let ts = &st.written_frames[2].1["args"]["activity"]["timestamps"];
            assert_eq!(ts["start"], 1_699_999_942);
            assert_eq!(ts["end"], 1_700_000_355);
        }

        // Failed sync backs off for RETRY_BACKOFF_MS before retrying.
        if let Some(c) = ext.conn.take() {
            c.close();
        }
        state.borrow_mut().fail_connect = true;
        host::set_mock_now_unix_ms(Some(1_700_000_010_000));
        ext.on_track_change(&sample_track()).unwrap();
        assert!(!ext.presence_synced);

        let frames_before_backoff = state.borrow().written_frames.len();
        host::set_mock_now_unix_ms(Some(1_700_000_015_000));
        ext.on_position_change(5_000).unwrap();
        assert_eq!(state.borrow().written_frames.len(), frames_before_backoff);

        state.borrow_mut().fail_connect = false;
        host::set_mock_now_unix_ms(Some(1_700_000_026_000));
        ext.on_position_change(16_000).unwrap();
        assert!(ext.presence_synced);

        host::clear_mocks();
    }

    #[test]
    fn test_connection_action() {
        host::clear_mocks();
        let state = Rc::new(RefCell::new(MockDiscordPipeState::new()));
        host::set_mock_ipc_transport(MockDiscordTransport(Rc::clone(&state)));

        let mut ext = test_ext();
        let msg = ext
            .on_action("test_connection", SettingsMap::new())
            .unwrap();
        assert_eq!(msg, "Connected to Discord as 'Alice'");

        host::clear_mocks();
    }

    #[test]
    fn test_unsolicited_dispatch_skipped_and_protocol_error_does_not_reconnect() {
        host::clear_mocks();
        host::set_mock_now_unix_ms(Some(1_700_000_000_000));

        struct CustomTransport {
            state: Rc<RefCell<MockDiscordPipeState>>,
            connect_count: Rc<RefCell<usize>>,
            inject_unsolicited_once: bool,
            inject_protocol_error_next: bool,
        }

        impl MockIpcTransport for CustomTransport {
            fn connect(&mut self, _candidates: &[String]) -> Result<u32, String> {
                *self.connect_count.borrow_mut() += 1;
                let mut st = self.state.borrow_mut();
                st.push_frame(
                    OP_FRAME,
                    json!({
                        "cmd": "DISPATCH",
                        "evt": "READY",
                        "data": { "user": { "username": "alice_user" } }
                    }),
                );
                Ok(1)
            }

            fn write(&mut self, _handle: u32, data: &[u8]) -> Result<(), String> {
                let opcode = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let len = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
                let val: Value = serde_json::from_slice(&data[8..8 + len]).unwrap();
                let nonce = val.get("nonce").cloned().unwrap_or(Value::Null);
                let mut st = self.state.borrow_mut();
                st.written_frames.push((opcode, val));

                if opcode == OP_FRAME {
                    if self.inject_unsolicited_once {
                        self.inject_unsolicited_once = false;
                        st.push_frame(
                            OP_FRAME,
                            json!({"cmd": "DISPATCH", "evt": "ACTIVITY_JOIN", "nonce": null}),
                        );
                    }
                    if self.inject_protocol_error_next {
                        self.inject_protocol_error_next = false;
                        st.push_frame(
                            OP_FRAME,
                            json!({
                                "cmd": "SET_ACTIVITY",
                                "evt": "ERROR",
                                "nonce": nonce,
                                "data": { "message": "Invalid activity payload" }
                            }),
                        );
                    } else {
                        st.push_frame(
                            OP_FRAME,
                            json!({"cmd": "SET_ACTIVITY", "evt": null, "nonce": nonce}),
                        );
                    }
                }
                Ok(())
            }

            fn read(&mut self, _handle: u32, max_len: u32) -> Result<Vec<u8>, String> {
                let mut st = self.state.borrow_mut();
                let n = (max_len as usize).min(st.read_queue.len());
                Ok(st.read_queue.drain(..n).collect())
            }
        }

        let state = Rc::new(RefCell::new(MockDiscordPipeState::new()));
        let connect_count = Rc::new(RefCell::new(0usize));
        host::set_mock_ipc_transport(CustomTransport {
            state: Rc::clone(&state),
            connect_count: Rc::clone(&connect_count),
            inject_unsolicited_once: true,
            inject_protocol_error_next: false,
        });

        let mut ext = test_ext();
        ext.init(SettingsMap::new()).unwrap();
        ext.on_track_change(&sample_track()).unwrap();
        assert!(ext.presence_synced);
        assert_eq!(*connect_count.borrow(), 1);

        // Next call receives an evt == "ERROR" with matching nonce: must NOT reconnect/resend.
        host::set_mock_ipc_transport(CustomTransport {
            state: Rc::clone(&state),
            connect_count: Rc::clone(&connect_count),
            inject_unsolicited_once: false,
            inject_protocol_error_next: true,
        });
        let frames_before = state.borrow().written_frames.len();
        ext.on_playback_status_change(false).unwrap();
        assert!(!ext.presence_synced);
        assert_eq!(*connect_count.borrow(), 1);
        assert_eq!(state.borrow().written_frames.len(), frames_before + 1);

        host::clear_mocks();
    }
}
