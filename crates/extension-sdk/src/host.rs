use crate::types::{ExtensionPlayerAction, HttpRequest, HttpResponse, LogLevel};

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;

    mod ffi_core {
        #[link(wasm_import_module = "melody")]
        extern "C" {
            pub(super) fn host_log(level: u32, ptr: u32, len: u32);
            pub(super) fn host_now_unix_ms() -> u64;
            pub(super) fn host_take_response(dst_ptr: u32, dst_len: u32) -> i32;
        }
    }

    mod ffi_network {
        #[link(wasm_import_module = "melody")]
        extern "C" {
            pub(super) fn host_http_request(req_ptr: u32, req_len: u32) -> u32;
        }
    }

    mod ffi_player_control {
        #[link(wasm_import_module = "melody")]
        extern "C" {
            pub(super) fn host_dispatch_player_action(ptr: u32, len: u32) -> i32;
        }
    }

    pub fn log(level: LogLevel, message: &str) {
        let bytes = message.as_bytes();
        unsafe {
            ffi_core::host_log(level as u32, bytes.as_ptr() as u32, bytes.len() as u32);
        }
    }

    pub fn now_unix_ms() -> u64 {
        unsafe { ffi_core::host_now_unix_ms() }
    }

    pub fn http_request(req: &HttpRequest) -> Result<HttpResponse, String> {
        let payload = serde_json::to_vec(req).map_err(|e| format!("serialize request: {e}"))?;
        let resp_len = unsafe {
            ffi_network::host_http_request(payload.as_ptr() as u32, payload.len() as u32)
        };
        if resp_len == 0 {
            return Err("Host HTTP response buffer was empty".to_string());
        }
        let mut buf = vec![0u8; resp_len as usize];
        let rc = unsafe { ffi_core::host_take_response(buf.as_mut_ptr() as u32, resp_len) };
        if rc != 0 {
            return Err(format!("Failed to take host response (code {rc})"));
        }
        serde_json::from_slice::<Result<HttpResponse, String>>(&buf)
            .map_err(|e| format!("deserialize host response: {e}"))?
    }

    pub fn dispatch_player_action(action: &ExtensionPlayerAction) -> Result<(), String> {
        let payload = serde_json::to_vec(action).map_err(|e| format!("serialize action: {e}"))?;
        let rc = unsafe {
            ffi_player_control::host_dispatch_player_action(
                payload.as_ptr() as u32,
                payload.len() as u32,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err("Host rejected player action".to_string())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::cell::{Cell, RefCell};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    type MockHttpFn = Box<dyn FnMut(&HttpRequest) -> Result<HttpResponse, String>>;

    std::thread_local! {
        static MOCK_HTTP: RefCell<Option<MockHttpFn>> = RefCell::new(None);
        static MOCK_NOW_MS: Cell<Option<u64>> = const { Cell::new(None) };
        static MOCK_ACTIONS: RefCell<Vec<ExtensionPlayerAction>> = const { RefCell::new(Vec::new()) };
    }

    /// Registers a thread-local HTTP mock handler for native unit testing.
    pub fn set_mock_http_handler(
        handler: impl FnMut(&HttpRequest) -> Result<HttpResponse, String> + 'static,
    ) {
        MOCK_HTTP.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(handler));
        });
    }

    /// Overrides [`now_unix_ms`] on the current thread for deterministic unit tests.
    pub fn set_mock_now_unix_ms(ms: Option<u64>) {
        MOCK_NOW_MS.with(|cell| cell.set(ms));
    }

    /// Drains recorded [`ExtensionPlayerAction`] calls on the current thread.
    pub fn take_mock_player_actions() -> Vec<ExtensionPlayerAction> {
        MOCK_ACTIONS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
    }

    /// Clears all thread-local test mocks.
    pub fn clear_mocks() {
        MOCK_HTTP.with(|cell| *cell.borrow_mut() = None);
        MOCK_NOW_MS.with(|cell| cell.set(None));
        MOCK_ACTIONS.with(|cell| cell.borrow_mut().clear());
    }

    pub fn log(_level: LogLevel, _message: &str) {}

    pub fn now_unix_ms() -> u64 {
        if let Some(mock) = MOCK_NOW_MS.with(Cell::get) {
            return mock;
        }
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn http_request(req: &HttpRequest) -> Result<HttpResponse, String> {
        MOCK_HTTP.with(|cell| {
            if let Some(handler) = cell.borrow_mut().as_mut() {
                handler(req)
            } else {
                Err("No mock HTTP handler registered on host target".to_string())
            }
        })
    }

    pub fn dispatch_player_action(action: &ExtensionPlayerAction) -> Result<(), String> {
        MOCK_ACTIONS.with(|cell| cell.borrow_mut().push(action.clone()));
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::{clear_mocks, set_mock_http_handler, set_mock_now_unix_ms, take_mock_player_actions};

/// Emits a structured log message to Melody's host logger.
pub fn log(level: LogLevel, message: impl AsRef<str>) {
    imp::log(level, message.as_ref());
}

pub fn log_error(message: impl AsRef<str>) {
    log(LogLevel::Error, message);
}

pub fn log_warn(message: impl AsRef<str>) {
    log(LogLevel::Warn, message);
}

pub fn log_info(message: impl AsRef<str>) {
    log(LogLevel::Info, message);
}

pub fn log_debug(message: impl AsRef<str>) {
    log(LogLevel::Debug, message);
}

/// Current UNIX epoch timestamp in milliseconds provided by the host.
pub fn now_unix_ms() -> u64 {
    imp::now_unix_ms()
}

/// Current UNIX epoch timestamp in seconds provided by the host.
pub fn now_unix_secs() -> u64 {
    imp::now_unix_ms() / 1000
}

/// Sends an outbound HTTP request through the host (requires `network` permission).
pub fn http_request(req: &HttpRequest) -> Result<HttpResponse, String> {
    imp::http_request(req)
}

/// Dispatches a player control command to Melody (requires `player-control` permission).
pub fn dispatch_player_action(action: &ExtensionPlayerAction) -> Result<(), String> {
    imp::dispatch_player_action(action)
}
