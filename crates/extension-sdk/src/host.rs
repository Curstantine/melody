use crate::settings::SettingValue;
use crate::types::{
    BrowserAuthRequest, BrowserAuthResponse, ExtensionPlayerAction, HttpRequest, HttpResponse,
    LogLevel, SettingUpdate,
};

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
            pub(super) fn host_browser_auth(req_ptr: u32, req_len: u32) -> u32;
        }
    }

    mod ffi_settings {
        #[link(wasm_import_module = "melody")]
        extern "C" {
            pub(super) fn host_set_setting(ptr: u32, len: u32) -> i32;
        }
    }

    mod ffi_player_control {
        #[link(wasm_import_module = "melody")]
        extern "C" {
            pub(super) fn host_dispatch_player_action(ptr: u32, len: u32) -> i32;
        }
    }

    mod ffi_local_ipc {
        #[link(wasm_import_module = "melody")]
        extern "C" {
            pub(super) fn host_pid() -> u32;
            pub(super) fn host_ipc_connect(names_ptr: u32, names_len: u32) -> u32;
            pub(super) fn host_ipc_write(handle: u32, data_ptr: u32, data_len: u32) -> u32;
            pub(super) fn host_ipc_read(handle: u32, max_len: u32) -> u32;
            pub(super) fn host_ipc_close(handle: u32) -> i32;
        }
    }

    fn take_raw_response_bytes(resp_len: u32) -> Result<Vec<u8>, String> {
        if resp_len == 0 {
            return Err("Host response buffer was empty".to_string());
        }
        let mut buf = vec![0u8; resp_len as usize];
        let rc = unsafe { ffi_core::host_take_response(buf.as_mut_ptr() as u32, resp_len) };
        if rc != 0 {
            return Err(format!("Failed to take host response (code {rc})"));
        }
        Ok(buf)
    }

    fn take_json_response<T: serde::de::DeserializeOwned>(resp_len: u32) -> Result<T, String> {
        let buf = take_raw_response_bytes(resp_len)?;
        serde_json::from_slice::<Result<T, String>>(&buf)
            .map_err(|e| format!("deserialize host response: {e}"))?
    }

    fn take_raw_response(resp_len: u32) -> Result<Vec<u8>, String> {
        let buf = take_raw_response_bytes(resp_len)?;
        match buf.split_first() {
            Some((0, payload)) => Ok(payload.to_vec()),
            Some((1, err_bytes)) => Err(String::from_utf8_lossy(err_bytes).into_owned()),
            _ => Err("Invalid raw host response framing".to_string()),
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

    pub fn pid() -> u32 {
        unsafe { ffi_local_ipc::host_pid() }
    }

    pub fn http_request(req: &HttpRequest) -> Result<HttpResponse, String> {
        let payload = serde_json::to_vec(req).map_err(|e| format!("serialize request: {e}"))?;
        let resp_len = unsafe {
            ffi_network::host_http_request(payload.as_ptr() as u32, payload.len() as u32)
        };
        take_json_response(resp_len)
    }

    pub fn browser_auth(req: &BrowserAuthRequest) -> Result<BrowserAuthResponse, String> {
        let payload =
            serde_json::to_vec(req).map_err(|e| format!("serialize auth request: {e}"))?;
        let resp_len = unsafe {
            ffi_network::host_browser_auth(payload.as_ptr() as u32, payload.len() as u32)
        };
        take_json_response(resp_len)
    }

    pub fn set_setting(key: String, value: SettingValue) -> Result<(), String> {
        let payload = serde_json::to_vec(&SettingUpdate { key, value })
            .map_err(|e| format!("serialize setting update: {e}"))?;
        let rc = unsafe {
            ffi_settings::host_set_setting(payload.as_ptr() as u32, payload.len() as u32)
        };
        if rc == 0 {
            Ok(())
        } else {
            Err("Host rejected setting update".to_string())
        }
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

    pub fn ipc_connect(candidates: &[&str]) -> Result<u32, String> {
        let payload =
            serde_json::to_vec(candidates).map_err(|e| format!("serialize ipc names: {e}"))?;
        let resp_len = unsafe {
            ffi_local_ipc::host_ipc_connect(payload.as_ptr() as u32, payload.len() as u32)
        };
        take_json_response(resp_len)
    }

    pub fn ipc_write(handle: u32, data: &[u8]) -> Result<(), String> {
        let resp_len = unsafe {
            ffi_local_ipc::host_ipc_write(handle, data.as_ptr() as u32, data.len() as u32)
        };
        take_json_response(resp_len)
    }

    pub fn ipc_read(handle: u32, max_len: u32) -> Result<Vec<u8>, String> {
        let resp_len = unsafe { ffi_local_ipc::host_ipc_read(handle, max_len) };
        take_raw_response(resp_len)
    }

    pub fn ipc_close(handle: u32) {
        unsafe {
            ffi_local_ipc::host_ipc_close(handle);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::cell::{Cell, RefCell};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    /// Mockable local IPC transport trait for native unit tests.
    pub trait MockIpcTransport {
        fn connect(&mut self, candidates: &[String]) -> Result<u32, String>;
        fn write(&mut self, handle: u32, data: &[u8]) -> Result<(), String>;
        fn read(&mut self, handle: u32, max_len: u32) -> Result<Vec<u8>, String>;
        fn close(&mut self, _handle: u32) {}
    }

    type MockHttpFn = Box<dyn FnMut(&HttpRequest) -> Result<HttpResponse, String>>;
    type MockBrowserAuthFn =
        Box<dyn FnMut(&BrowserAuthRequest) -> Result<BrowserAuthResponse, String>>;

    std::thread_local! {
        static MOCK_HTTP: RefCell<Option<MockHttpFn>> = RefCell::new(None);
        static MOCK_BROWSER_AUTH: RefCell<Option<MockBrowserAuthFn>> = RefCell::new(None);
        static MOCK_IPC: RefCell<Option<Box<dyn MockIpcTransport>>> = RefCell::new(None);
        static MOCK_NOW_MS: Cell<Option<u64>> = const { Cell::new(None) };
        static MOCK_ACTIONS: RefCell<Vec<ExtensionPlayerAction>> = const { RefCell::new(Vec::new()) };
        static MOCK_SETTING_UPDATES: RefCell<Vec<SettingUpdate>> = const { RefCell::new(Vec::new()) };
    }

    /// Registers a thread-local HTTP mock handler for native unit testing.
    pub fn set_mock_http_handler(
        handler: impl FnMut(&HttpRequest) -> Result<HttpResponse, String> + 'static,
    ) {
        MOCK_HTTP.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(handler));
        });
    }

    /// Registers a thread-local browser auth mock handler for native unit testing.
    pub fn set_mock_browser_auth_handler(
        handler: impl FnMut(&BrowserAuthRequest) -> Result<BrowserAuthResponse, String> + 'static,
    ) {
        MOCK_BROWSER_AUTH.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(handler));
        });
    }

    /// Registers a thread-local local IPC mock transport for native unit testing.
    pub fn set_mock_ipc_transport(transport: impl MockIpcTransport + 'static) {
        MOCK_IPC.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(transport));
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

    /// Drains recorded [`SettingUpdate`] calls on the current thread.
    pub fn take_mock_setting_updates() -> Vec<SettingUpdate> {
        MOCK_SETTING_UPDATES.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
    }

    /// Clears all thread-local test mocks.
    pub fn clear_mocks() {
        MOCK_HTTP.with(|cell| *cell.borrow_mut() = None);
        MOCK_BROWSER_AUTH.with(|cell| *cell.borrow_mut() = None);
        MOCK_IPC.with(|cell| *cell.borrow_mut() = None);
        MOCK_NOW_MS.with(|cell| cell.set(None));
        MOCK_ACTIONS.with(|cell| cell.borrow_mut().clear());
        MOCK_SETTING_UPDATES.with(|cell| cell.borrow_mut().clear());
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

    pub fn pid() -> u32 {
        std::process::id()
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

    pub fn browser_auth(req: &BrowserAuthRequest) -> Result<BrowserAuthResponse, String> {
        MOCK_BROWSER_AUTH.with(|cell| {
            if let Some(handler) = cell.borrow_mut().as_mut() {
                handler(req)
            } else {
                Err("No mock browser auth handler registered on host target".to_string())
            }
        })
    }

    pub fn set_setting(key: String, value: SettingValue) -> Result<(), String> {
        MOCK_SETTING_UPDATES.with(|cell| cell.borrow_mut().push(SettingUpdate { key, value }));
        Ok(())
    }

    pub fn dispatch_player_action(action: &ExtensionPlayerAction) -> Result<(), String> {
        MOCK_ACTIONS.with(|cell| cell.borrow_mut().push(action.clone()));
        Ok(())
    }

    pub fn ipc_connect(candidates: &[&str]) -> Result<u32, String> {
        let owned: Vec<String> = candidates.iter().map(|s| (*s).to_string()).collect();
        MOCK_IPC.with(|cell| {
            if let Some(transport) = cell.borrow_mut().as_mut() {
                transport.connect(&owned)
            } else {
                Err("No mock IPC transport registered on host target".to_string())
            }
        })
    }

    pub fn ipc_write(handle: u32, data: &[u8]) -> Result<(), String> {
        MOCK_IPC.with(|cell| {
            if let Some(transport) = cell.borrow_mut().as_mut() {
                transport.write(handle, data)
            } else {
                Err("No mock IPC transport registered on host target".to_string())
            }
        })
    }

    pub fn ipc_read(handle: u32, max_len: u32) -> Result<Vec<u8>, String> {
        MOCK_IPC.with(|cell| {
            if let Some(transport) = cell.borrow_mut().as_mut() {
                transport.read(handle, max_len)
            } else {
                Err("No mock IPC transport registered on host target".to_string())
            }
        })
    }

    pub fn ipc_close(handle: u32) {
        MOCK_IPC.with(|cell| {
            if let Some(transport) = cell.borrow_mut().as_mut() {
                transport.close(handle);
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::{
    clear_mocks, set_mock_browser_auth_handler, set_mock_http_handler, set_mock_ipc_transport,
    set_mock_now_unix_ms, take_mock_player_actions, take_mock_setting_updates, MockIpcTransport,
};

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

/// Host process ID (requires `local-ipc` permission; useful for local IPC protocols).
pub fn pid() -> u32 {
    imp::pid()
}

/// Sends an outbound HTTP request through the host (requires `network` permission).
pub fn http_request(req: &HttpRequest) -> Result<HttpResponse, String> {
    imp::http_request(req)
}

/// Opens a browser authorization URL with a local callback listener and returns query parameters
/// from the redirect (requires `network` permission).
pub fn browser_auth(req: &BrowserAuthRequest) -> Result<BrowserAuthResponse, String> {
    imp::browser_auth(req)
}

/// Persists a setting update back to the host configuration (requires `settings` permission).
pub fn set_setting(key: impl Into<String>, value: SettingValue) -> Result<(), String> {
    imp::set_setting(key.into(), value)
}

/// Dispatches a player control command to Melody (requires `player-control` permission).
pub fn dispatch_player_action(action: &ExtensionPlayerAction) -> Result<(), String> {
    imp::dispatch_player_action(action)
}

/// Connects to the first available local named pipe or Unix domain socket matching `candidates`
/// (requires `local-ipc` permission and `allowed_ipc` manifest allowlist).
pub fn ipc_connect(candidates: &[&str]) -> Result<u32, String> {
    imp::ipc_connect(candidates)
}

/// Writes raw bytes to an open local IPC handle (requires `local-ipc` permission).
pub fn ipc_write(handle: u32, data: &[u8]) -> Result<(), String> {
    imp::ipc_write(handle, data)
}

/// Reads up to `max_len` bytes from an open local IPC handle (requires `local-ipc` permission).
pub fn ipc_read(handle: u32, max_len: u32) -> Result<Vec<u8>, String> {
    imp::ipc_read(handle, max_len)
}

/// Closes an open local IPC handle (requires `local-ipc` permission).
pub fn ipc_close(handle: u32) {
    imp::ipc_close(handle);
}
