use crate::settings::{SettingOption, SettingsMap};
use crate::types::{ExtensionRequest, ExtensionResponse, LyricsRequest, RepeatMode, TrackInfo};

/// Trait implemented by Melody WebAssembly extensions.
///
/// Register an implementation with [`crate::register_extension!`].
pub trait Extension: Default + Send + 'static {
    /// Called once right after instantiation with persisted user settings merged with defaults.
    fn init(&mut self, _settings: SettingsMap) -> Result<(), String> {
        Ok(())
    }

    /// Returns dynamic settings schema items if the extension builds options at runtime.
    /// Returning an empty list falls back to `[[settings]]` declared in `extension.toml`.
    fn settings_schema(&self) -> Vec<SettingOption> {
        Vec::new()
    }

    /// Called whenever the user changes one or more settings in Settings -> Extensions.
    fn on_settings_change(&mut self, _settings: SettingsMap) -> Result<(), String> {
        Ok(())
    }

    /// Invoked when the user clicks a `type = "action"` button in Settings -> Extensions.
    fn on_action(&mut self, action_id: &str, _settings: SettingsMap) -> Result<String, String> {
        Err(format!("Unknown action '{action_id}'"))
    }

    fn on_track_change(&mut self, _track: &TrackInfo) -> Result<(), String> {
        Ok(())
    }

    fn on_playback_end(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn on_playback_status_change(&mut self, _is_playing: bool) -> Result<(), String> {
        Ok(())
    }

    fn on_position_change(&mut self, _position_ms: u64) -> Result<(), String> {
        Ok(())
    }

    fn on_seeked(&mut self, _position_ms: u64) -> Result<(), String> {
        Ok(())
    }

    fn on_volume_change(&mut self, _volume: f32) -> Result<(), String> {
        Ok(())
    }

    fn on_repeat_change(&mut self, _mode: RepeatMode) -> Result<(), String> {
        Ok(())
    }

    fn on_shuffle_change(&mut self, _is_shuffle: bool) -> Result<(), String> {
        Ok(())
    }

    fn fetch_lyrics(&mut self, _request: &LyricsRequest) -> Result<Option<String>, String> {
        Ok(None)
    }
}

/// Dispatches an [`ExtensionRequest`] to an [`Extension`] implementation and wraps the outcome.
pub fn dispatch_request<E: Extension>(ext: &mut E, req: ExtensionRequest) -> ExtensionResponse {
    match req {
        ExtensionRequest::Init { settings } => match ext.init(settings) {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::GetSettingsSchema => ExtensionResponse::SettingsSchema {
            options: ext.settings_schema(),
        },
        ExtensionRequest::OnSettingsChange { settings } => match ext.on_settings_change(settings) {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::OnAction {
            action_id,
            settings,
        } => match ext.on_action(&action_id, settings) {
            Ok(message) => ExtensionResponse::ActionResult { ok: true, message },
            Err(message) => ExtensionResponse::ActionResult { ok: false, message },
        },
        ExtensionRequest::OnTrackChange { track } => match ext.on_track_change(&track) {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::OnPlaybackEnd => match ext.on_playback_end() {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::OnPlaybackStatusChange { is_playing } => {
            match ext.on_playback_status_change(is_playing) {
                Ok(()) => ExtensionResponse::Ok,
                Err(message) => ExtensionResponse::Err { message },
            }
        }
        ExtensionRequest::OnPositionChange { position_ms } => {
            match ext.on_position_change(position_ms) {
                Ok(()) => ExtensionResponse::Ok,
                Err(message) => ExtensionResponse::Err { message },
            }
        }
        ExtensionRequest::OnSeeked { position_ms } => match ext.on_seeked(position_ms) {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::OnVolumeChange { volume } => match ext.on_volume_change(volume) {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::OnRepeatChange { mode } => match ext.on_repeat_change(mode) {
            Ok(()) => ExtensionResponse::Ok,
            Err(message) => ExtensionResponse::Err { message },
        },
        ExtensionRequest::OnShuffleChange { is_shuffle } => {
            match ext.on_shuffle_change(is_shuffle) {
                Ok(()) => ExtensionResponse::Ok,
                Err(message) => ExtensionResponse::Err { message },
            }
        }
        ExtensionRequest::FetchLyrics { request } => match ext.fetch_lyrics(&request) {
            Ok(lyrics) => ExtensionResponse::Lyrics { lyrics },
            Err(message) => ExtensionResponse::Err { message },
        },
    }
}

/// Exports the Melody WASM ABI symbols (`melody_abi_version`, `melody_alloc`,
/// `melody_dealloc`, `melody_call`) for an [`Extension`] type when compiled to `wasm32`.
#[macro_export]
macro_rules! register_extension {
    ($ext_ty:ty) => {
        #[cfg(target_arch = "wasm32")]
        mod __melody_extension_exports {
            use super::*;
            use std::sync::Mutex;

            static INSTANCE: Mutex<Option<$ext_ty>> = Mutex::new(None);

            #[no_mangle]
            pub extern "C" fn melody_abi_version() -> u32 {
                $crate::ABI_VERSION
            }

            #[no_mangle]
            pub extern "C" fn melody_alloc(len: u32) -> u32 {
                if len == 0 {
                    return 0;
                }
                let Ok(layout) = ::std::alloc::Layout::from_size_align(len as usize, 1) else {
                    return 0;
                };
                let ptr = unsafe { ::std::alloc::alloc(layout) };
                ptr as u32
            }

            #[no_mangle]
            pub extern "C" fn melody_dealloc(ptr: u32, len: u32) {
                if ptr == 0 || len == 0 {
                    return;
                }
                if let Ok(layout) = ::std::alloc::Layout::from_size_align(len as usize, 1) {
                    unsafe { ::std::alloc::dealloc(ptr as *mut u8, layout) };
                }
            }

            #[no_mangle]
            pub extern "C" fn melody_call(ptr: u32, len: u32) -> u64 {
                let response = if ptr == 0 || len == 0 {
                    $crate::ExtensionResponse::Err {
                        message: "empty ABI request".to_string(),
                    }
                } else {
                    let slice =
                        unsafe { ::std::slice::from_raw_parts(ptr as *const u8, len as usize) };
                    match $crate::__private::serde_json::from_slice::<$crate::ExtensionRequest>(
                        slice,
                    ) {
                        Ok(req) => match INSTANCE.lock() {
                            Ok(mut guard) => {
                                let ext = guard.get_or_insert_with(<$ext_ty as Default>::default);
                                $crate::dispatch_request(ext, req)
                            }
                            Err(_) => $crate::ExtensionResponse::Err {
                                message: "extension instance lock poisoned".to_string(),
                            },
                        },
                        Err(err) => $crate::ExtensionResponse::Err {
                            message: format!("invalid extension request: {err}"),
                        },
                    }
                };

                let bytes = $crate::__private::serde_json::to_vec(&response).unwrap_or_else(|_| {
                    br#"{"type":"err","message":"response serialization failed"}"#.to_vec()
                });
                let out_len = bytes.len() as u32;
                let out_ptr = melody_alloc(out_len);
                if out_ptr == 0 {
                    return 0;
                }
                unsafe {
                    ::std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        out_ptr as *mut u8,
                        bytes.len(),
                    );
                }
                (u64::from(out_ptr) << 32) | u64::from(out_len)
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        const _: fn() = || {
            fn assert_ext<T: $crate::Extension>() {}
            assert_ext::<$ext_ty>();
        };
    };
}
