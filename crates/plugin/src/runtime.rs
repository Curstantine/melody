use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::Sender;
use melody_extension_sdk::{
    ExtensionPermission, ExtensionPlayerAction, ExtensionRequest, ExtensionResponse, HttpMethod,
    HttpRequest, HttpResponse, LogLevel, LyricsRequest, RepeatMode, SettingOption, SettingsMap,
    ABI_VERSION,
};
use tracing::{debug, error, info, trace, warn};
use wasmi::{
    Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

use crate::manifest::ExtensionManifest;
use crate::types::PlayerAction;

/// Maximum linear memory a single WASM extension may allocate (16 MB).
pub const MAX_MEMORY_BYTES: usize = 16 * 1024 * 1024;
/// Maximum fuel allowed per host-to-WASM invocation to prevent infinite loops.
pub const CALL_FUEL_LIMIT: u64 = 5_000_000;
/// Maximum HTTP request JSON payload accepted from guest memory.
pub const MAX_HTTP_REQ_BYTES: usize = 2 * 1024 * 1024;
/// Maximum HTTP response body read from the network (4 MB).
pub const MAX_HTTP_RESP_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum ABI response payload returned from `melody_call`.
pub const MAX_ABI_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
/// Default and maximum outbound HTTP timeouts.
pub const DEFAULT_HTTP_TIMEOUT_MS: u64 = 10_000;
pub const MAX_HTTP_TIMEOUT_MS: u64 = 15_000;
/// Minimum interval between `OnPositionChange` dispatches per extension.
pub const POSITION_THROTTLE_MS: u64 = 1_000;

/// Creates the shared `wasmi` engine configured with fuel metering.
pub fn create_engine() -> Engine {
    let mut config = Config::default();
    config.consume_fuel(true);
    Engine::new(&config)
}

/// Verifies that every import requested by `module` is an allowed Melody host function
/// backed by the extension's declared manifest permissions.
pub fn audit_module_imports(
    module: &Module,
    permissions: &[ExtensionPermission],
) -> Result<(), String> {
    let granted: HashSet<ExtensionPermission> = permissions.iter().copied().collect();
    for import in module.imports() {
        let mod_name = import.module();
        let field_name = import.name();
        if mod_name != "melody" {
            return Err(format!(
                "Disallowed WASM import '{mod_name}::{field_name}' (only 'melody' host imports are permitted)"
            ));
        }

        match field_name {
            "host_log" | "host_now_unix_ms" | "host_take_response" => {}
            "host_http_request" => {
                if !granted.contains(&ExtensionPermission::Network) {
                    return Err(
                        "Module imports 'melody::host_http_request' without declaring the 'network' permission"
                            .to_string(),
                    );
                }
            }
            "host_dispatch_player_action" => {
                if !granted.contains(&ExtensionPermission::PlayerControl) {
                    return Err(
                        "Module imports 'melody::host_dispatch_player_action' without declaring the 'player-control' permission"
                            .to_string(),
                    );
                }
            }
            other => {
                return Err(format!("Unknown host import 'melody::{other}'"));
            }
        }
    }
    Ok(())
}

struct HostState {
    extension_id: String,
    permissions: HashSet<ExtensionPermission>,
    allowed_hosts: Vec<String>,
    action_tx: Sender<PlayerAction>,
    response_buf: Vec<u8>,
    limiter: StoreLimits,
}

impl HostState {
    fn new(manifest: &ExtensionManifest, action_tx: Sender<PlayerAction>) -> Self {
        let limiter = StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_BYTES)
            .memories(1)
            .tables(4)
            .instances(1)
            .build();
        Self {
            extension_id: manifest.id.clone(),
            permissions: manifest.permissions.iter().copied().collect(),
            allowed_hosts: manifest.allowed_hosts.clone(),
            action_tx,
            response_buf: Vec::new(),
            limiter,
        }
    }
}

fn read_guest_bytes(
    caller: &Caller<'_, HostState>,
    ptr: u32,
    len: u32,
    max_len: usize,
) -> Result<Vec<u8>, String> {
    let len_usize = len as usize;
    if len_usize > max_len {
        return Err(format!(
            "Guest buffer length {len_usize} exceeds limit {max_len}"
        ));
    }
    let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
        return Err("WASM module does not export 'memory'".to_string());
    };
    let mut buf = vec![0u8; len_usize];
    memory
        .read(caller, ptr as usize, &mut buf)
        .map_err(|e| format!("Guest memory read out of bounds: {e}"))?;
    Ok(buf)
}

fn host_matches_pattern(host: &str, pattern: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let pattern = pattern.trim().to_ascii_lowercase();
    if pattern.is_empty() {
        return false;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{suffix}"))
    } else {
        host == pattern
    }
}

fn perform_http_request(
    req: &HttpRequest,
    allowed_hosts: &[String],
) -> Result<HttpResponse, String> {
    let parsed =
        url::Url::parse(&req.url).map_err(|e| format!("Invalid URL '{}': {e}", req.url))?;
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "Disallowed URL scheme '{scheme}' (only http and https are allowed)"
            ));
        }
    }

    if !allowed_hosts.is_empty() {
        let host = parsed
            .host_str()
            .ok_or_else(|| "URL has no host component".to_string())?;
        if !allowed_hosts
            .iter()
            .any(|pattern| host_matches_pattern(host, pattern))
        {
            return Err(format!(
                "Host '{host}' is not permitted by extension allowed_hosts"
            ));
        }
    }

    let timeout_ms = req
        .timeout_ms
        .unwrap_or(DEFAULT_HTTP_TIMEOUT_MS)
        .clamp(500, MAX_HTTP_TIMEOUT_MS);

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_millis(timeout_ms)))
        .build()
        .into();

    let response = match req.method {
        HttpMethod::Get | HttpMethod::Delete => {
            let mut rb = match req.method {
                HttpMethod::Get => agent.get(&req.url),
                HttpMethod::Delete => agent.delete(&req.url),
                _ => unreachable!(),
            };
            for (k, v) in &req.headers {
                rb = rb.header(k, v);
            }
            rb.call().map_err(|e| format!("HTTP request failed: {e}"))?
        }
        HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => {
            let mut rb = match req.method {
                HttpMethod::Post => agent.post(&req.url),
                HttpMethod::Put => agent.put(&req.url),
                HttpMethod::Patch => agent.patch(&req.url),
                _ => unreachable!(),
            };
            for (k, v) in &req.headers {
                rb = rb.header(k, v);
            }
            if let Some(ref form) = req.form {
                let pairs: Vec<(&str, &str)> =
                    form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                rb.send_form(pairs)
                    .map_err(|e| format!("HTTP form request failed: {e}"))?
            } else if let Some(ref body) = req.body {
                rb.send(body.as_str())
                    .map_err(|e| format!("HTTP body request failed: {e}"))?
            } else {
                rb.send_empty()
                    .map_err(|e| format!("HTTP request failed: {e}"))?
            }
        }
    };

    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|val| (k.as_str().to_string(), val.to_string()))
        })
        .collect();
    let body = response
        .into_body()
        .with_config()
        .limit(MAX_HTTP_RESP_BYTES)
        .read_to_string()
        .map_err(|e| format!("Failed to read HTTP response body: {e}"))?;

    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn map_player_action(action: ExtensionPlayerAction) -> PlayerAction {
    match action {
        ExtensionPlayerAction::Play => PlayerAction::Play,
        ExtensionPlayerAction::Pause => PlayerAction::Pause,
        ExtensionPlayerAction::TogglePlayPause => PlayerAction::TogglePlayPause,
        ExtensionPlayerAction::Stop => PlayerAction::Stop,
        ExtensionPlayerAction::Next => PlayerAction::Next,
        ExtensionPlayerAction::Previous => PlayerAction::Previous,
        ExtensionPlayerAction::SeekOffsetMs { offset_ms } => {
            // Host PlayerAction::Seek uses positive Duration offsets cleanly for forward seeks
            PlayerAction::Seek(Duration::from_millis(offset_ms.max(0) as u64))
        }
        ExtensionPlayerAction::SetPositionMs { position_ms } => {
            PlayerAction::SetPosition(Duration::from_millis(position_ms))
        }
        ExtensionPlayerAction::SetVolume { volume } => PlayerAction::SetVolume(volume),
        ExtensionPlayerAction::SetShuffle { enabled } => PlayerAction::SetShuffle(enabled),
        ExtensionPlayerAction::SetRepeat { mode } => {
            let mapped = match mode {
                RepeatMode::Off => model::RepeatMode::Off,
                RepeatMode::All => model::RepeatMode::All,
                RepeatMode::One => model::RepeatMode::One,
            };
            PlayerAction::SetRepeat(mapped)
        }
    }
}

fn build_linker(
    engine: &Engine,
    permissions: &[ExtensionPermission],
) -> Result<Linker<HostState>, String> {
    let mut linker = Linker::<HostState>::new(engine);
    let granted: HashSet<ExtensionPermission> = permissions.iter().copied().collect();

    linker
        .func_wrap(
            "melody",
            "host_log",
            |caller: Caller<'_, HostState>, level: u32, ptr: u32, len: u32| {
                let Ok(bytes) = read_guest_bytes(&caller, ptr, len, 16 * 1024) else {
                    return;
                };
                let msg = String::from_utf8_lossy(&bytes);
                let ext_id = &caller.data().extension_id;
                match LogLevel::from_u32(level) {
                    LogLevel::Error => error!("[ext:{ext_id}] {msg}"),
                    LogLevel::Warn => warn!("[ext:{ext_id}] {msg}"),
                    LogLevel::Info => info!("[ext:{ext_id}] {msg}"),
                    LogLevel::Debug => debug!("[ext:{ext_id}] {msg}"),
                    LogLevel::Trace => trace!("[ext:{ext_id}] {msg}"),
                }
            },
        )
        .map_err(|e| format!("Failed to link host_log: {e}"))?;

    linker
        .func_wrap(
            "melody",
            "host_now_unix_ms",
            |_caller: Caller<'_, HostState>| -> u64 {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0)
            },
        )
        .map_err(|e| format!("Failed to link host_now_unix_ms: {e}"))?;

    linker
        .func_wrap(
            "melody",
            "host_take_response",
            |mut caller: Caller<'_, HostState>, dst_ptr: u32, dst_len: u32| -> i32 {
                let buf = std::mem::take(&mut caller.data_mut().response_buf);
                if buf.len() != dst_len as usize {
                    return -1;
                }
                let Some(Extern::Memory(memory)) = caller.get_export("memory") else {
                    return -1;
                };
                if memory.write(&mut caller, dst_ptr as usize, &buf).is_err() {
                    return -1;
                }
                0
            },
        )
        .map_err(|e| format!("Failed to link host_take_response: {e}"))?;

    if granted.contains(&ExtensionPermission::Network) {
        linker
            .func_wrap(
                "melody",
                "host_http_request",
                |mut caller: Caller<'_, HostState>, req_ptr: u32, req_len: u32| -> u32 {
                    let result: Result<HttpResponse, String> = if !caller
                        .data()
                        .permissions
                        .contains(&ExtensionPermission::Network)
                    {
                        Err("Permission 'network' denied".to_string())
                    } else {
                        match read_guest_bytes(&caller, req_ptr, req_len, MAX_HTTP_REQ_BYTES) {
                            Ok(bytes) => match serde_json::from_slice::<HttpRequest>(&bytes) {
                                Ok(req) => {
                                    let allowed = caller.data().allowed_hosts.clone();
                                    perform_http_request(&req, &allowed)
                                }
                                Err(e) => Err(format!("Invalid HTTP request JSON: {e}")),
                            },
                            Err(e) => Err(e),
                        }
                    };

                    let encoded = serde_json::to_vec(&result).unwrap_or_else(|_| {
                        br#"{"Err":"Failed to serialize HTTP result"}"#.to_vec()
                    });
                    let len = encoded.len() as u32;
                    caller.data_mut().response_buf = encoded;
                    len
                },
            )
            .map_err(|e| format!("Failed to link host_http_request: {e}"))?;
    }

    if granted.contains(&ExtensionPermission::PlayerControl) {
        linker
            .func_wrap(
                "melody",
                "host_dispatch_player_action",
                |caller: Caller<'_, HostState>, ptr: u32, len: u32| -> i32 {
                    if !caller
                        .data()
                        .permissions
                        .contains(&ExtensionPermission::PlayerControl)
                    {
                        return -1;
                    }
                    let Ok(bytes) = read_guest_bytes(&caller, ptr, len, 64 * 1024) else {
                        return -1;
                    };
                    let Ok(action) = serde_json::from_slice::<ExtensionPlayerAction>(&bytes) else {
                        return -1;
                    };
                    let mapped = map_player_action(action);
                    if caller.data().action_tx.send(mapped).is_ok() {
                        0
                    } else {
                        -1
                    }
                },
            )
            .map_err(|e| format!("Failed to link host_dispatch_player_action: {e}"))?;
    }

    Ok(linker)
}

struct InstanceBindings {
    memory: wasmi::Memory,
    alloc: wasmi::TypedFunc<u32, u32>,
    dealloc: wasmi::TypedFunc<(u32, u32), ()>,
    call: wasmi::TypedFunc<(u32, u32), u64>,
}

impl InstanceBindings {
    fn from_instance(
        store: &mut Store<HostState>,
        instance: &wasmi::Instance,
    ) -> Result<Self, String> {
        let memory = instance
            .get_memory(&*store, "memory")
            .ok_or_else(|| "Extension does not export 'memory'".to_string())?;
        let abi_fn = instance
            .get_typed_func::<(), u32>(&*store, "melody_abi_version")
            .map_err(|e| format!("Missing 'melody_abi_version' export: {e}"))?;
        let alloc = instance
            .get_typed_func::<u32, u32>(&*store, "melody_alloc")
            .map_err(|e| format!("Missing 'melody_alloc' export: {e}"))?;
        let dealloc = instance
            .get_typed_func::<(u32, u32), ()>(&*store, "melody_dealloc")
            .map_err(|e| format!("Missing 'melody_dealloc' export: {e}"))?;
        let call = instance
            .get_typed_func::<(u32, u32), u64>(&*store, "melody_call")
            .map_err(|e| format!("Missing 'melody_call' export: {e}"))?;

        store
            .set_fuel(CALL_FUEL_LIMIT)
            .map_err(|e| format!("Failed to set fuel: {e}"))?;
        let abi = abi_fn
            .call(&mut *store, ())
            .map_err(|e| format!("melody_abi_version trapped: {e}"))?;
        if abi != ABI_VERSION {
            return Err(format!(
                "Extension ABI version mismatch: guest reported {abi}, host requires {ABI_VERSION}"
            ));
        }

        Ok(Self {
            memory,
            alloc,
            dealloc,
            call,
        })
    }

    fn invoke(
        &self,
        store: &mut Store<HostState>,
        request: &ExtensionRequest,
    ) -> Result<ExtensionResponse, String> {
        let payload = serde_json::to_vec(request)
            .map_err(|e| format!("Failed to serialize extension request: {e}"))?;
        let in_len = u32::try_from(payload.len())
            .map_err(|_| "Extension request payload exceeds u32 length".to_string())?;

        store
            .set_fuel(CALL_FUEL_LIMIT)
            .map_err(|e| format!("Failed to set fuel: {e}"))?;

        let in_ptr = self
            .alloc
            .call(&mut *store, in_len)
            .map_err(|e| format!("Extension melody_alloc failed: {e}"))?;
        if in_ptr == 0 && in_len > 0 {
            return Err("Extension melody_alloc returned null pointer".to_string());
        }

        if let Err(e) = self.memory.write(&mut *store, in_ptr as usize, &payload) {
            let _ = self.dealloc.call(&mut *store, (in_ptr, in_len));
            return Err(format!("Failed to write request into WASM memory: {e}"));
        }

        let packed = self
            .call
            .call(&mut *store, (in_ptr, in_len))
            .map_err(|e| format!("Extension trapped during call: {e}"))?;

        let _ = self.dealloc.call(&mut *store, (in_ptr, in_len));

        let out_ptr = (packed >> 32) as u32;
        let out_len = (packed & 0xFFFF_FFFF) as u32;
        if out_ptr == 0 || out_len == 0 {
            return Err("Extension returned empty ABI response".to_string());
        }
        if out_len as usize > MAX_ABI_RESPONSE_BYTES {
            let _ = self.dealloc.call(&mut *store, (out_ptr, out_len));
            return Err(format!(
                "Extension response length {out_len} exceeds limit {MAX_ABI_RESPONSE_BYTES}"
            ));
        }

        let mut out_buf = vec![0u8; out_len as usize];
        let read_res = self.memory.read(&*store, out_ptr as usize, &mut out_buf);
        let _ = self.dealloc.call(&mut *store, (out_ptr, out_len));
        read_res.map_err(|e| format!("Failed to read response from WASM memory: {e}"))?;

        serde_json::from_slice::<ExtensionResponse>(&out_buf)
            .map_err(|e| format!("Invalid extension response JSON: {e}"))
    }
}

enum WorkerCommand {
    Event(ExtensionRequest),
    PositionUpdate(u64),
    SettingsChanged(SettingsMap),
    Action {
        action_id: String,
        settings: SettingsMap,
        reply: mpsc::Sender<Result<String, String>>,
    },
    FetchLyrics {
        request: LyricsRequest,
        reply: mpsc::Sender<Result<Option<String>, String>>,
    },
    Shutdown,
}

/// Handle to a running, isolated WebAssembly extension instance on its own worker thread.
pub struct WasmExtensionHandle {
    id: String,
    tx: crossbeam_channel::Sender<WorkerCommand>,
    join_handle: std::sync::Mutex<Option<JoinHandle<()>>>,
    last_position_sent_ms: AtomicU64,
}

impl WasmExtensionHandle {
    /// Compiles `wasm_bytes`, audits imports against `manifest.permissions`, spawns a dedicated
    /// worker thread, initializes the extension with `initial_settings`, and queries any dynamic
    /// settings schema if the `settings` permission is granted.
    pub fn spawn(
        engine: &Engine,
        manifest: ExtensionManifest,
        wasm_bytes: &[u8],
        initial_settings: SettingsMap,
        action_tx: Sender<PlayerAction>,
    ) -> Result<(Self, Option<Vec<SettingOption>>), String> {
        let module = Module::new(engine, wasm_bytes)
            .map_err(|e| format!("Failed to compile WASM module: {e}"))?;
        audit_module_imports(&module, &manifest.permissions)?;

        let engine_clone = engine.clone();
        let ext_id = manifest.id.clone();
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerCommand>();
        let (init_tx, init_rx) = mpsc::channel::<Result<Option<Vec<SettingOption>>, String>>();

        let thread_name = format!("ext-{}", manifest.id);
        let join_handle = thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let host_state = HostState::new(&manifest, action_tx);
                let mut store = Store::new(&engine_clone, host_state);
                store.limiter(|state| &mut state.limiter);

                let linker = match build_linker(&engine_clone, &manifest.permissions) {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = init_tx.send(Err(e));
                        return;
                    }
                };

                if let Err(e) = store.set_fuel(CALL_FUEL_LIMIT) {
                    let _ = init_tx.send(Err(format!("Failed to set init fuel: {e}")));
                    return;
                }

                let instance = match linker.instantiate_and_start(&mut store, &module) {
                    Ok(inst) => inst,
                    Err(e) => {
                        let _ =
                            init_tx.send(Err(format!("Failed to instantiate WASM module: {e}")));
                        return;
                    }
                };

                let bindings = match InstanceBindings::from_instance(&mut store, &instance) {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = init_tx.send(Err(e));
                        return;
                    }
                };

                match bindings.invoke(
                    &mut store,
                    &ExtensionRequest::Init {
                        settings: initial_settings,
                    },
                ) {
                    Ok(ExtensionResponse::Ok) => {}
                    Ok(ExtensionResponse::Err { message }) => {
                        let _ = init_tx.send(Err(format!("Extension init error: {message}")));
                        return;
                    }
                    Ok(other) => {
                        let _ = init_tx.send(Err(format!(
                            "Unexpected Init response from extension: {other:?}"
                        )));
                        return;
                    }
                    Err(e) => {
                        let _ = init_tx.send(Err(e));
                        return;
                    }
                }

                let schema = if manifest.has_permission(ExtensionPermission::Settings) {
                    match bindings.invoke(&mut store, &ExtensionRequest::GetSettingsSchema) {
                        Ok(ExtensionResponse::SettingsSchema { options })
                            if !options.is_empty() =>
                        {
                            if let Err(e) = crate::manifest::validate_settings_schema(&options) {
                                let _ = init_tx
                                    .send(Err(format!("Invalid runtime settings schema: {e}")));
                                return;
                            }
                            Some(options)
                        }
                        Ok(ExtensionResponse::SettingsSchema { .. }) => None,
                        Ok(ExtensionResponse::Err { message }) => {
                            let _ = init_tx
                                .send(Err(format!("Extension GetSettingsSchema error: {message}")));
                            return;
                        }
                        Ok(_) => None,
                        Err(e) => {
                            let _ = init_tx.send(Err(e));
                            return;
                        }
                    }
                } else {
                    None
                };

                let _ = init_tx.send(Ok(schema));

                run_worker_loop(&manifest.id, &mut store, &bindings, rx);
            })
            .map_err(|e| format!("Failed to spawn extension thread: {e}"))?;

        match init_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(schema)) => Ok((
                Self {
                    id: ext_id,
                    tx,
                    join_handle: std::sync::Mutex::new(Some(join_handle)),
                    last_position_sent_ms: AtomicU64::new(0),
                },
                schema,
            )),
            Ok(Err(err)) => {
                let _ = join_handle.join();
                Err(err)
            }
            Err(_) => {
                let _ = tx.send(WorkerCommand::Shutdown);
                Err(format!(
                    "Extension '{ext_id}' timed out during initialization"
                ))
            }
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn send_event(&self, req: ExtensionRequest) {
        let _ = self.tx.send(WorkerCommand::Event(req));
    }

    pub fn send_position(&self, position_ms: u64) {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let prev = self.last_position_sent_ms.load(Ordering::Relaxed);
        if now_ms.saturating_sub(prev) < POSITION_THROTTLE_MS {
            return;
        }
        self.last_position_sent_ms.store(now_ms, Ordering::Relaxed);
        let _ = self.tx.send(WorkerCommand::PositionUpdate(position_ms));
    }

    pub fn send_settings(&self, settings: SettingsMap) {
        let _ = self.tx.send(WorkerCommand::SettingsChanged(settings));
    }

    pub fn trigger_action(
        &self,
        action_id: String,
        settings: SettingsMap,
    ) -> Result<String, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(WorkerCommand::Action {
                action_id,
                settings,
                reply: reply_tx,
            })
            .map_err(|_| format!("Extension '{}' worker is not running", self.id))?;
        reply_rx
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| format!("Extension '{}' action timed out", self.id))?
    }

    pub fn fetch_lyrics(&self, request: LyricsRequest) -> Result<Option<String>, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(WorkerCommand::FetchLyrics {
                request,
                reply: reply_tx,
            })
            .map_err(|_| format!("Extension '{}' worker is not running", self.id))?;
        reply_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| format!("Extension '{}' lyrics lookup timed out", self.id))?
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(WorkerCommand::Shutdown);
        if let Ok(mut guard) = self.join_handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for WasmExtensionHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_worker_loop(
    ext_id: &str,
    store: &mut Store<HostState>,
    bindings: &InstanceBindings,
    rx: crossbeam_channel::Receiver<WorkerCommand>,
) {
    while let Ok(cmd) = rx.recv() {
        if !handle_worker_command(ext_id, store, bindings, &rx, cmd) {
            break;
        }
    }
}

fn handle_worker_command(
    ext_id: &str,
    store: &mut Store<HostState>,
    bindings: &InstanceBindings,
    rx: &crossbeam_channel::Receiver<WorkerCommand>,
    cmd: WorkerCommand,
) -> bool {
    match cmd {
        WorkerCommand::Shutdown => return false,
        WorkerCommand::Event(req) => {
            handle_simple_invocation(ext_id, store, bindings, &req);
        }
        WorkerCommand::PositionUpdate(mut pos_ms) => {
            // Coalesce backlogged position updates if this extension was busy.
            let mut deferred: Option<WorkerCommand> = None;
            while let Ok(next_cmd) = rx.try_recv() {
                match next_cmd {
                    WorkerCommand::PositionUpdate(newer) => pos_ms = newer,
                    other => {
                        deferred = Some(other);
                        break;
                    }
                }
            }
            handle_simple_invocation(
                ext_id,
                store,
                bindings,
                &ExtensionRequest::OnPositionChange {
                    position_ms: pos_ms,
                },
            );
            if let Some(next_cmd) = deferred {
                return handle_worker_command(ext_id, store, bindings, rx, next_cmd);
            }
        }
        WorkerCommand::SettingsChanged(settings) => {
            handle_simple_invocation(
                ext_id,
                store,
                bindings,
                &ExtensionRequest::OnSettingsChange { settings },
            );
        }
        WorkerCommand::Action {
            action_id,
            settings,
            reply,
        } => {
            let res = match bindings.invoke(
                store,
                &ExtensionRequest::OnAction {
                    action_id,
                    settings,
                },
            ) {
                Ok(ExtensionResponse::ActionResult { ok: true, message }) => Ok(message),
                Ok(ExtensionResponse::ActionResult { ok: false, message }) => Err(message),
                Ok(ExtensionResponse::Ok) => Ok("Action completed".to_string()),
                Ok(ExtensionResponse::Err { message }) => Err(message),
                Ok(other) => Err(format!("Unexpected action response: {other:?}")),
                Err(e) => Err(e),
            };
            let _ = reply.send(res);
        }
        WorkerCommand::FetchLyrics { request, reply } => {
            let res = match bindings.invoke(store, &ExtensionRequest::FetchLyrics { request }) {
                Ok(ExtensionResponse::Lyrics { lyrics }) => Ok(lyrics),
                Ok(ExtensionResponse::Err { message }) => Err(message),
                Ok(other) => Err(format!("Unexpected lyrics response: {other:?}")),
                Err(e) => Err(e),
            };
            let _ = reply.send(res);
        }
    }
    true
}

fn handle_simple_invocation(
    ext_id: &str,
    store: &mut Store<HostState>,
    bindings: &InstanceBindings,
    req: &ExtensionRequest,
) {
    match bindings.invoke(store, req) {
        Ok(ExtensionResponse::Ok) => {}
        Ok(ExtensionResponse::Err { message }) => {
            warn!("Extension '{ext_id}' returned error: {message}");
        }
        Ok(other) => {
            debug!("Extension '{ext_id}' returned unexpected response: {other:?}");
        }
        Err(e) => {
            warn!("Extension '{ext_id}' call failed: {e}");
        }
    }
}
