use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crossbeam_channel::Sender;
use melody_extension_sdk::{
    ExtensionConfigEntry, ExtensionPermission, ExtensionRequest, LyricsRequest, RepeatMode,
    SettingOption, SettingValue, TrackInfo,
};
use tracing::{error, info, warn};
use wasmi::Engine;

use crate::installer::{
    discover_extensions, install_extension_from_path, uninstall_extension as remove_user_extension,
    DiscoveredExtension,
};
use crate::manifest::{
    merge_settings_with_schema, ExtensionManifest, ExtensionOrigin, ExtensionSnapshot,
    ExtensionStatus,
};
use crate::runtime::{create_engine, ExtensionActionOutcome, WasmExtensionHandle};
use crate::types::{PlayerAction, TrackMetadata};

struct ManagedExtension {
    discovered: DiscoveredExtension,
    snapshot: ExtensionSnapshot,
    handle: Option<Arc<WasmExtensionHandle>>,
}

#[derive(Default)]
struct ExtensionRegistry {
    entries: BTreeMap<String, ManagedExtension>,
    order: Vec<String>,
}

impl ExtensionRegistry {
    fn snapshots(&self) -> Vec<ExtensionSnapshot> {
        self.order
            .iter()
            .filter_map(|id| self.entries.get(id).map(|m| m.snapshot.clone()))
            .collect()
    }
}

#[derive(Clone)]
pub struct PluginManager {
    engine: Arc<Engine>,
    action_tx: Sender<PlayerAction>,
    extensions: Arc<RwLock<ExtensionRegistry>>,
    bundled_dir_override: Option<PathBuf>,
    user_dir_override: Option<PathBuf>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        let (tx, _rx) = crossbeam_channel::unbounded();
        Self::new_with_action_sender(tx)
    }

    pub fn new_with_action_sender(action_tx: Sender<PlayerAction>) -> Self {
        Self {
            engine: Arc::new(create_engine()),
            action_tx,
            extensions: Arc::new(RwLock::new(ExtensionRegistry::default())),
            bundled_dir_override: None,
            user_dir_override: None,
        }
    }

    /// Creates a `PluginManager` pointing at custom directories (intended for isolated tests).
    #[cfg(test)]
    pub fn with_dirs(
        action_tx: Sender<PlayerAction>,
        bundled_dir: Option<PathBuf>,
        user_dir: PathBuf,
    ) -> Self {
        Self {
            engine: Arc::new(create_engine()),
            action_tx,
            extensions: Arc::new(RwLock::new(ExtensionRegistry::default())),
            bundled_dir_override: bundled_dir,
            user_dir_override: Some(user_dir),
        }
    }

    fn effective_bundled_dir(&self) -> Option<PathBuf> {
        self.bundled_dir_override
            .clone()
            .or_else(path::bundled_extensions_dir)
    }

    fn effective_user_dir(&self) -> PathBuf {
        self.user_dir_override
            .clone()
            .unwrap_or_else(|| path::user_extensions_dir().to_path_buf())
    }

    /// Discovers bundled and user extensions, stops any previous workers, starts enabled valid
    /// extensions with persisted settings, and returns sorted snapshots.
    pub fn discover_and_sync(
        &self,
        configs: &BTreeMap<String, ExtensionConfigEntry>,
    ) -> Vec<ExtensionSnapshot> {
        let old_handles = self.take_all_handles();
        for handle in old_handles {
            handle.shutdown();
        }

        let bundled = self.effective_bundled_dir();
        let user = self.effective_user_dir();
        let discovered_list = discover_extensions(bundled.as_deref(), &user);

        let mut entries = BTreeMap::new();
        let mut order = Vec::with_capacity(discovered_list.len());

        for disc in discovered_list {
            let id = disc.manifest.id.clone();
            let entry_cfg = configs.get(&id);
            let enabled = entry_cfg.is_some_and(|c| c.enabled);
            let saved_settings = entry_cfg.map(|c| &c.settings);
            let managed = self.start_discovered_extension(disc, enabled, saved_settings);
            order.push(id.clone());
            entries.insert(id, managed);
        }

        let snapshots = {
            let mut reg = self.extensions.write().expect("extension registry lock");
            reg.entries = entries;
            reg.order = order;
            reg.snapshots()
        };

        info!("Synced {} WASM extension(s)", snapshots.len());
        snapshots
    }

    fn start_discovered_extension(
        &self,
        disc: DiscoveredExtension,
        enabled: bool,
        saved_settings: Option<&BTreeMap<String, SettingValue>>,
    ) -> ManagedExtension {
        let empty_map = BTreeMap::new();
        let saved = saved_settings.unwrap_or(&empty_map);
        let fallback_schema = default_schema_for_manifest(&disc.manifest);

        if let Some(ref err) = disc.load_error {
            return ManagedExtension {
                snapshot: ExtensionSnapshot {
                    manifest: disc.manifest.clone(),
                    origin: disc.origin,
                    dir: disc.dir.clone(),
                    enabled,
                    status: ExtensionStatus::Error(err.clone()),
                    settings_schema: fallback_schema,
                },
                discovered: disc,
                handle: None,
            };
        }

        if !enabled {
            return ManagedExtension {
                snapshot: ExtensionSnapshot {
                    manifest: disc.manifest.clone(),
                    origin: disc.origin,
                    dir: disc.dir.clone(),
                    enabled: false,
                    status: ExtensionStatus::Disabled,
                    settings_schema: fallback_schema,
                },
                discovered: disc,
                handle: None,
            };
        }

        let Some(ref wasm_path) = disc.wasm_path else {
            return ManagedExtension {
                snapshot: ExtensionSnapshot {
                    manifest: disc.manifest.clone(),
                    origin: disc.origin,
                    dir: disc.dir.clone(),
                    enabled: true,
                    status: ExtensionStatus::Error("Missing extension module path".to_string()),
                    settings_schema: fallback_schema,
                },
                discovered: disc,
                handle: None,
            };
        };

        let wasm_bytes = match std::fs::read(wasm_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                return ManagedExtension {
                    snapshot: ExtensionSnapshot {
                        manifest: disc.manifest.clone(),
                        origin: disc.origin,
                        dir: disc.dir.clone(),
                        enabled: true,
                        status: ExtensionStatus::Error(format!(
                            "Failed to read '{}': {e}",
                            wasm_path.display()
                        )),
                        settings_schema: fallback_schema,
                    },
                    discovered: disc,
                    handle: None,
                };
            }
        };

        let init_settings = merge_settings_with_schema(&disc.manifest.settings, saved);
        match WasmExtensionHandle::spawn(
            &self.engine,
            disc.manifest.clone(),
            &wasm_bytes,
            init_settings,
            self.action_tx.clone(),
        ) {
            Ok((handle, runtime_schema)) => {
                let effective_schema =
                    if !disc.manifest.has_permission(ExtensionPermission::Settings) {
                        Vec::new()
                    } else if let Some(schema) = runtime_schema {
                        schema
                    } else {
                        disc.manifest.settings.clone()
                    };

                info!(
                    "Started extension '{}' ({})",
                    disc.manifest.name, disc.manifest.id
                );
                ManagedExtension {
                    snapshot: ExtensionSnapshot {
                        manifest: disc.manifest.clone(),
                        origin: disc.origin,
                        dir: disc.dir.clone(),
                        enabled: true,
                        status: ExtensionStatus::Running,
                        settings_schema: effective_schema,
                    },
                    discovered: disc,
                    handle: Some(Arc::new(handle)),
                }
            }
            Err(err) => {
                error!(
                    "Failed to start extension '{}' ({}): {err}",
                    disc.manifest.name, disc.manifest.id
                );
                ManagedExtension {
                    snapshot: ExtensionSnapshot {
                        manifest: disc.manifest.clone(),
                        origin: disc.origin,
                        dir: disc.dir.clone(),
                        enabled: true,
                        status: ExtensionStatus::Error(err),
                        settings_schema: fallback_schema,
                    },
                    discovered: disc,
                    handle: None,
                }
            }
        }
    }

    pub fn snapshots(&self) -> Vec<ExtensionSnapshot> {
        self.extensions
            .read()
            .map(|reg| reg.snapshots())
            .unwrap_or_default()
    }

    /// Enables or disables a single extension without restarting other extensions.
    pub fn set_extension_enabled(
        &self,
        id: &str,
        enabled: bool,
        saved_settings: &BTreeMap<String, SettingValue>,
    ) -> Result<Vec<ExtensionSnapshot>, String> {
        let (discovered, old_handle) = {
            let mut reg = self.extensions.write().map_err(|e| e.to_string())?;
            let entry = reg
                .entries
                .get_mut(id)
                .ok_or_else(|| format!("Extension '{id}' not found"))?;
            (entry.discovered.clone(), entry.handle.take())
        };

        if let Some(handle) = old_handle {
            handle.shutdown();
        }

        let managed = self.start_discovered_extension(discovered, enabled, Some(saved_settings));
        let mut reg = self.extensions.write().map_err(|e| e.to_string())?;
        reg.entries.insert(id.to_string(), managed);
        Ok(reg.snapshots())
    }

    /// Pushes updated persisted settings to a running extension.
    pub fn update_extension_settings(
        &self,
        id: &str,
        saved_settings: &BTreeMap<String, SettingValue>,
    ) {
        let target = {
            let Ok(reg) = self.extensions.read() else {
                return;
            };
            reg.entries.get(id).and_then(|entry| {
                entry
                    .handle
                    .clone()
                    .map(|handle| (handle, entry.snapshot.merged_settings(saved_settings)))
            })
        };

        if let Some((handle, merged)) = target {
            handle.send_settings(merged);
        }
    }

    /// Triggers a declarative `action` option on a running extension and returns its feedback message.
    pub fn trigger_extension_action(
        &self,
        id: &str,
        action_id: &str,
        saved_settings: &BTreeMap<String, SettingValue>,
    ) -> Result<ExtensionActionOutcome, String> {
        let (handle, merged) = {
            let reg = self.extensions.read().map_err(|e| e.to_string())?;
            let entry = reg
                .entries
                .get(id)
                .ok_or_else(|| format!("Extension '{id}' not found"))?;
            if !entry.snapshot.enabled {
                return Err(format!(
                    "Enable '{}' before running actions",
                    entry.snapshot.manifest.name
                ));
            }
            let handle = entry
                .handle
                .clone()
                .ok_or_else(|| match &entry.snapshot.status {
                    ExtensionStatus::Error(e) => e.clone(),
                    _ => format!(
                        "Extension '{}' is not running",
                        entry.snapshot.manifest.name
                    ),
                })?;
            let merged = entry.snapshot.merged_settings(saved_settings);
            (handle, merged)
        };

        handle.trigger_action(action_id.to_string(), merged)
    }

    /// Installs an extension from a local directory or `.zip` / `.mext` archive and re-syncs.
    pub fn install_from_path(
        &self,
        source: &Path,
        configs: &BTreeMap<String, ExtensionConfigEntry>,
    ) -> Result<(ExtensionManifest, Vec<ExtensionSnapshot>), String> {
        let user_dir = self.effective_user_dir();
        let manifest = install_extension_from_path(&self.engine, source, &user_dir)?;
        let snapshots = self.discover_and_sync(configs);
        Ok((manifest, snapshots))
    }

    /// Uninstalls a user-installed extension and removes it from the active registry.
    pub fn uninstall_extension(&self, id: &str) -> Result<Vec<ExtensionSnapshot>, String> {
        let old_handle = {
            let mut reg = self.extensions.write().map_err(|e| e.to_string())?;
            let Some(entry) = reg.entries.get_mut(id) else {
                return Err(format!("Extension '{id}' not found"));
            };
            if entry.snapshot.origin != ExtensionOrigin::User {
                return Err("Bundled extensions cannot be uninstalled".to_string());
            }
            entry.handle.take()
        };

        if let Some(handle) = old_handle {
            handle.shutdown();
        }

        let user_dir = self.effective_user_dir();
        remove_user_extension(id, &user_dir)?;

        // Rebuild snapshots in case a bundled extension of the same id exists underneath.
        let bundled = self.effective_bundled_dir();
        let preserved_configs: HashMap<String, bool> = self
            .extensions
            .read()
            .map(|reg| {
                reg.entries
                    .iter()
                    .filter(|(k, _)| k.as_str() != id)
                    .map(|(k, v)| (k.clone(), v.snapshot.enabled))
                    .collect()
            })
            .unwrap_or_default();

        let discovered_list = discover_extensions(bundled.as_deref(), &user_dir);
        let mut reg = self.extensions.write().map_err(|e| e.to_string())?;
        reg.entries.remove(id);
        let mut new_entries = BTreeMap::new();
        let mut new_order = Vec::with_capacity(discovered_list.len());
        for disc in discovered_list {
            let ext_id = disc.manifest.id.clone();
            if let Some(existing) = reg.entries.remove(&ext_id) {
                new_order.push(ext_id.clone());
                new_entries.insert(ext_id, existing);
            } else {
                let enabled = preserved_configs.get(&ext_id).copied().unwrap_or(false);
                let managed = self.start_discovered_extension(disc, enabled, None);
                new_order.push(ext_id.clone());
                new_entries.insert(ext_id, managed);
            }
        }
        reg.entries = new_entries;
        reg.order = new_order;
        Ok(reg.snapshots())
    }

    /// Gracefully shuts down all running WASM extension worker threads.
    pub fn shutdown_all(&self) {
        let handles = self.take_all_handles();
        for handle in handles {
            handle.shutdown();
        }
    }

    fn take_all_handles(&self) -> Vec<Arc<WasmExtensionHandle>> {
        let Ok(mut reg) = self.extensions.write() else {
            return Vec::new();
        };
        let mut handles = Vec::new();
        for entry in reg.entries.values_mut() {
            if let Some(h) = entry.handle.take() {
                handles.push(h);
            }
        }
        handles
    }

    fn playback_event_handles(&self) -> Vec<Arc<WasmExtensionHandle>> {
        let Ok(reg) = self.extensions.read() else {
            return Vec::new();
        };
        reg.order
            .iter()
            .filter_map(|id| reg.entries.get(id))
            .filter(|m| {
                m.snapshot
                    .manifest
                    .has_permission(ExtensionPermission::PlaybackEvents)
            })
            .filter_map(|m| m.handle.clone())
            .collect()
    }

    fn lyrics_handles(&self) -> Vec<Arc<WasmExtensionHandle>> {
        let Ok(reg) = self.extensions.read() else {
            return Vec::new();
        };
        reg.order
            .iter()
            .filter_map(|id| reg.entries.get(id))
            .filter(|m| {
                m.snapshot
                    .manifest
                    .has_permission(ExtensionPermission::Lyrics)
            })
            .filter_map(|m| m.handle.clone())
            .collect()
    }

    /// Replays the current track, position, and play/pause state to a single running extension
    /// if it declares `playback-events` (used when an extension is enabled mid-playback).
    pub fn sync_playback_state_for_extension(
        &self,
        id: &str,
        track: &TrackMetadata,
        is_playing: bool,
        position_ms: u64,
    ) {
        let handle = {
            let Ok(reg) = self.extensions.read() else {
                return;
            };
            reg.entries.get(id).and_then(|m| {
                if m.snapshot
                    .manifest
                    .has_permission(ExtensionPermission::PlaybackEvents)
                {
                    m.handle.clone()
                } else {
                    None
                }
            })
        };

        if let Some(handle) = handle {
            let info = TrackInfo {
                title: track.title.clone(),
                artist: track.artist.clone(),
                album: track.album.clone(),
                duration_ms: track.duration_ms,
            };
            handle.send_event(ExtensionRequest::OnTrackChange { track: info });
            handle.send_event(ExtensionRequest::OnPositionChange { position_ms });
            handle.send_event(ExtensionRequest::OnPlaybackStatusChange { is_playing });
        }
    }

    pub fn on_track_change(&self, track: &TrackMetadata) {
        let info = TrackInfo {
            title: track.title.clone(),
            artist: track.artist.clone(),
            album: track.album.clone(),
            duration_ms: track.duration_ms,
        };
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnTrackChange {
                track: info.clone(),
            });
        }
    }

    pub fn on_playback_end(&self) {
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnPlaybackEnd);
        }
    }

    pub fn on_playback_status_change(&self, is_playing: bool) {
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnPlaybackStatusChange { is_playing });
        }
    }

    pub fn on_position_change(&self, position_ms: u64) {
        for handle in self.playback_event_handles() {
            handle.send_position(position_ms);
        }
    }

    pub fn on_seeked(&self, position_ms: u64) {
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnSeeked { position_ms });
        }
    }

    pub fn on_volume_change(&self, volume: f32) {
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnVolumeChange { volume });
        }
    }

    pub fn on_repeat_change(&self, mode: model::RepeatMode) {
        let ext_mode = match mode {
            model::RepeatMode::Off => RepeatMode::Off,
            model::RepeatMode::All => RepeatMode::All,
            model::RepeatMode::One => RepeatMode::One,
        };
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnRepeatChange { mode: ext_mode });
        }
    }

    pub fn on_shuffle_change(&self, is_shuffle: bool) {
        for handle in self.playback_event_handles() {
            handle.send_event(ExtensionRequest::OnShuffleChange { is_shuffle });
        }
    }

    pub fn fetch_lyrics(
        &self,
        artist: &str,
        title: &str,
        album: &str,
        duration_ms: u64,
    ) -> Option<String> {
        let handles = self.lyrics_handles();
        if !handles.is_empty() {
            let req = LyricsRequest {
                artist: artist.to_string(),
                title: title.to_string(),
                album: album.to_string(),
                duration_ms,
            };
            for handle in handles {
                match handle.fetch_lyrics(req.clone()) {
                    Ok(Some(lyrics)) if !lyrics.trim().is_empty() => return Some(lyrics),
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Extension '{}' fetch_lyrics error: {e}", handle.id());
                    }
                }
            }
        }

        None
    }
}

fn default_schema_for_manifest(manifest: &ExtensionManifest) -> Vec<SettingOption> {
    if manifest.has_permission(ExtensionPermission::Settings) {
        manifest.settings.clone()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::audit_module_imports;
    use std::io::Write;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("melody_ext_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Minimal valid WAT extension module with configurable extra import and infinite loop toggle.
    fn minimal_wat_module(extra_import: Option<&str>, loop_forever_on_call: bool) -> Vec<u8> {
        let import_block = extra_import.unwrap_or("");
        let body = if loop_forever_on_call {
            r#"
                (loop $inf
                    (br $inf)
                )
                (i64.const 0)
            "#
        } else {
            r#"
                ;; Write {"type":"ok"} (13 bytes) at offset 1024
                (i32.store8 (i32.const 1024) (i32.const 123))
                (i32.store8 (i32.const 1025) (i32.const 34))
                (i32.store8 (i32.const 1026) (i32.const 116))
                (i32.store8 (i32.const 1027) (i32.const 121))
                (i32.store8 (i32.const 1028) (i32.const 112))
                (i32.store8 (i32.const 1029) (i32.const 101))
                (i32.store8 (i32.const 1030) (i32.const 34))
                (i32.store8 (i32.const 1031) (i32.const 58))
                (i32.store8 (i32.const 1032) (i32.const 34))
                (i32.store8 (i32.const 1033) (i32.const 111))
                (i32.store8 (i32.const 1034) (i32.const 107))
                (i32.store8 (i32.const 1035) (i32.const 34))
                (i32.store8 (i32.const 1036) (i32.const 125))
                (i64.or
                    (i64.shl (i64.const 1024) (i64.const 32))
                    (i64.const 13)
                )
            "#
        };

        format!(
            r#"
            (module
                {import_block}
                (memory (export "memory") 1)
                (func (export "melody_abi_version") (result i32)
                    (i32.const 1)
                )
                (func (export "melody_alloc") (param i32) (result i32)
                    (i32.const 2048)
                )
                (func (export "melody_dealloc") (param i32 i32))
                (func (export "melody_call") (param i32 i32) (result i64)
                    {body}
                )
            )
            "#
        )
        .into_bytes()
    }

    #[test]
    fn test_permission_audit_blocks_undeclared_imports() {
        let engine = create_engine();
        let net_wasm = minimal_wat_module(
            Some(r#"(import "melody" "host_http_request" (func (param i32 i32) (result i32)))"#),
            false,
        );
        let module = wasmi::Module::new(&engine, &net_wasm).unwrap();

        let err =
            audit_module_imports(&module, &[ExtensionPermission::PlaybackEvents]).unwrap_err();
        assert!(err.contains("network"));
        assert!(audit_module_imports(&module, &[ExtensionPermission::Network]).is_ok());

        let ctrl_wasm = minimal_wat_module(
            Some(
                r#"(import "melody" "host_dispatch_player_action" (func (param i32 i32) (result i32)))"#,
            ),
            false,
        );
        let ctrl_mod = wasmi::Module::new(&engine, &ctrl_wasm).unwrap();
        let err = audit_module_imports(&ctrl_mod, &[ExtensionPermission::Network]).unwrap_err();
        assert!(err.contains("player-control"));
        assert!(audit_module_imports(&ctrl_mod, &[ExtensionPermission::PlayerControl]).is_ok());

        let ipc_wasm = minimal_wat_module(
            Some(r#"(import "melody" "host_ipc_connect" (func (param i32 i32) (result i32)))"#),
            false,
        );
        let ipc_mod = wasmi::Module::new(&engine, &ipc_wasm).unwrap();
        let err =
            audit_module_imports(&ipc_mod, &[ExtensionPermission::PlaybackEvents]).unwrap_err();
        assert!(err.contains("local-ipc"));
        assert!(audit_module_imports(&ipc_mod, &[ExtensionPermission::LocalIpc]).is_ok());

        let pid_wasm = minimal_wat_module(
            Some(r#"(import "melody" "host_pid" (func (result i32)))"#),
            false,
        );
        let pid_mod = wasmi::Module::new(&engine, &pid_wasm).unwrap();
        let err =
            audit_module_imports(&pid_mod, &[ExtensionPermission::PlaybackEvents]).unwrap_err();
        assert!(err.contains("local-ipc"));
        assert!(audit_module_imports(&pid_mod, &[ExtensionPermission::LocalIpc]).is_ok());
    }

    #[test]
    fn test_fuel_limit_traps_infinite_loop_extension() {
        let engine = create_engine();
        let loop_wasm = minimal_wat_module(None, true);
        let manifest = ExtensionManifest::from_toml_str(
            r#"
id = "test.loop"
name = "Loop"
version = "0.1.0"
"#,
        )
        .unwrap();
        let (tx, _rx) = crossbeam_channel::unbounded();
        let err = WasmExtensionHandle::spawn(
            &engine,
            manifest,
            &loop_wasm,
            melody_extension_sdk::SettingsMap::new(),
            tx,
        )
        .err()
        .expect("infinite loop must trap on fuel exhaustion");
        assert!(err.to_lowercase().contains("fuel") || err.contains("trapped"));
    }

    #[test]
    fn test_memory_limit_blocks_excessive_growth() {
        let engine = create_engine();
        // 300 pages * 64KB = 19.2 MB > 16 MB limit
        let big_mem_wat = r#"
            (module
                (memory (export "memory") 300)
                (func (export "melody_abi_version") (result i32) (i32.const 1))
                (func (export "melody_alloc") (param i32) (result i32) (i32.const 1024))
                (func (export "melody_dealloc") (param i32 i32))
                (func (export "melody_call") (param i32 i32) (result i64) (i64.const 0))
            )
        "#;
        let manifest = ExtensionManifest::from_toml_str(
            r#"
id = "test.bigmem"
name = "BigMem"
version = "0.1.0"
"#,
        )
        .unwrap();
        let (tx, _rx) = crossbeam_channel::unbounded();
        let res = WasmExtensionHandle::spawn(
            &engine,
            manifest,
            big_mem_wat.as_bytes(),
            melody_extension_sdk::SettingsMap::new(),
            tx,
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_player_control_permission_dispatches_action() {
        let engine = create_engine();
        let wat = r#"
            (module
                (import "melody" "host_dispatch_player_action" (func $dispatch (param i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 64) "{\"action\":\"next\"}")
                (data (i32.const 1024) "{\"type\":\"ok\"}")
                (func (export "melody_abi_version") (result i32) (i32.const 1))
                (func (export "melody_alloc") (param i32) (result i32) (i32.const 2048))
                (func (export "melody_dealloc") (param i32 i32))
                (func (export "melody_call") (param i32 i32) (result i64)
                    (drop (call $dispatch (i32.const 64) (i32.const 17)))
                    (i64.or (i64.shl (i64.const 1024) (i64.const 32)) (i64.const 13))
                )
            )
        "#;
        let manifest = ExtensionManifest::from_toml_str(
            r#"
id = "test.ctrl"
name = "Ctrl"
version = "0.1.0"
permissions = ["player-control"]
"#,
        )
        .unwrap();
        let (tx, rx) = crossbeam_channel::unbounded();
        let (handle, _) = WasmExtensionHandle::spawn(
            &engine,
            manifest,
            wat.as_bytes(),
            melody_extension_sdk::SettingsMap::new(),
            tx,
        )
        .expect("spawn player-control module");
        let action = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert_eq!(action, PlayerAction::Next);
        handle.shutdown();
    }

    #[test]
    fn test_installer_dir_zip_zip_slip_and_uninstall() {
        let root = temp_dir("installer");
        let src_dir = root.join("src_ext");
        let user_dir = root.join("user_ext");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&user_dir).unwrap();

        let wasm = minimal_wat_module(None, false);
        std::fs::write(
            src_dir.join("extension.toml"),
            r#"
id = "com.example.local"
name = "Local Ext"
version = "0.1.0"
permissions = ["settings"]

[[settings]]
key = "flag"
label = "Flag"
type = "toggle"
default = false
"#,
        )
        .unwrap();
        std::fs::write(src_dir.join("extension.wasm"), &wasm).unwrap();

        let engine = create_engine();
        let manifest = install_extension_from_path(&engine, &src_dir, &user_dir).unwrap();
        assert_eq!(manifest.id, "com.example.local");
        assert!(user_dir.join("com.example.local/extension.toml").is_file());

        // Zip package with single nested folder
        let zip_path = root.join("nested.mext");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("pkg/extension.toml", opts).unwrap();
            zip.write_all(
                br#"
id = "com.example.zipped"
name = "Zipped Ext"
version = "0.2.0"
"#,
            )
            .unwrap();
            zip.start_file("pkg/extension.wasm", opts).unwrap();
            zip.write_all(&wasm).unwrap();
            zip.finish().unwrap();
        }
        let zipped = install_extension_from_path(&engine, &zip_path, &user_dir).unwrap();
        assert_eq!(zipped.id, "com.example.zipped");

        // Zip Slip archive must be rejected
        let evil_zip = root.join("evil.zip");
        {
            let file = std::fs::File::create(&evil_zip).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("../escape.txt", opts).unwrap();
            zip.write_all(b"evil").unwrap();
            zip.finish().unwrap();
        }
        assert!(install_extension_from_path(&engine, &evil_zip, &user_dir).is_err());

        // Uninstall user extension
        remove_user_extension("com.example.zipped", &user_dir).unwrap();
        assert!(!user_dir.join("com.example.zipped").exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_bundled_real_wasm_extensions_end_to_end() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let extensions_dir = manifest_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("extensions");
        let user_dir = temp_dir("real_wasm_user");

        // Verify compiled bundled WASM modules exist and do NOT import disallowed host functions
        let engine = create_engine();
        for ext_name in ["lastfm", "listenbrainz", "discord-rpc"] {
            let wasm_path = extensions_dir.join(ext_name).join("extension.wasm");
            if !wasm_path.is_file() {
                eprintln!(
                    "Skipping bundled WASM e2e test: {} not built (run scripts/build-extensions)",
                    wasm_path.display()
                );
                let _ = std::fs::remove_dir_all(&user_dir);
                return;
            }
            let bytes = std::fs::read(&wasm_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to read compiled wasm at {}: {e}",
                    wasm_path.display()
                )
            });
            let module = wasmi::Module::new(&engine, &bytes).unwrap();
            let imports: Vec<String> = module
                .imports()
                .map(|i| format!("{}::{}", i.module(), i.name()))
                .collect();
            assert!(
                imports.iter().all(|i| i.starts_with("melody::")),
                "Unexpected non-melody imports in {ext_name}: {imports:?}"
            );
            assert!(
                !imports
                    .iter()
                    .any(|i| i == "melody::host_dispatch_player_action"),
                "Unused player_control import leaked into {ext_name}"
            );
            if ext_name != "discord-rpc" {
                assert!(
                    !imports.iter().any(|i| i == "melody::host_ipc_connect"),
                    "Unused local_ipc import leaked into {ext_name}"
                );
            } else {
                assert!(
                    !imports.iter().any(|i| i == "melody::host_http_request"),
                    "Unused http_request import leaked into {ext_name}"
                );
            }
        }

        let (tx, _rx) = crossbeam_channel::unbounded();
        let manager = PluginManager::with_dirs(tx, Some(extensions_dir), user_dir.clone());

        let mut configs = BTreeMap::new();
        configs.insert(
            "melody.listenbrainz".to_string(),
            ExtensionConfigEntry {
                enabled: true,
                settings: BTreeMap::new(),
            },
        );
        configs.insert(
            "melody.lastfm".to_string(),
            ExtensionConfigEntry {
                enabled: true,
                settings: BTreeMap::new(),
            },
        );
        configs.insert(
            "melody.discord-rpc".to_string(),
            ExtensionConfigEntry {
                enabled: true,
                settings: BTreeMap::new(),
            },
        );

        let snapshots = manager.discover_and_sync(&configs);
        assert!(snapshots.len() >= 3);

        let lb = snapshots
            .iter()
            .find(|s| s.manifest.id == "melody.listenbrainz")
            .expect("listenbrainz snapshot");
        assert_eq!(lb.status, ExtensionStatus::Running);
        assert!(!lb.settings_schema.is_empty());

        let lfm = snapshots
            .iter()
            .find(|s| s.manifest.id == "melody.lastfm")
            .expect("lastfm snapshot");
        assert_eq!(lfm.status, ExtensionStatus::Running);

        let drpc = snapshots
            .iter()
            .find(|s| s.manifest.id == "melody.discord-rpc")
            .expect("discord-rpc snapshot");
        assert_eq!(drpc.status, ExtensionStatus::Running);
        assert!(!drpc.settings_schema.is_empty());

        // Trigger action with empty token/api_key: executes real WASM validation and returns Err
        let lb_err = manager
            .trigger_extension_action("melody.listenbrainz", "test_connection", &BTreeMap::new())
            .unwrap_err();
        assert!(lb_err.contains("empty"));

        let lfm_err = manager
            .trigger_extension_action("melody.lastfm", "test_connection", &BTreeMap::new())
            .unwrap_err();
        assert!(
            lfm_err.contains("not connected") || lfm_err.contains("not bundled"),
            "unexpected lastfm validation error: {lfm_err}"
        );

        match manager.trigger_extension_action(
            "melody.discord-rpc",
            "test_connection",
            &BTreeMap::new(),
        ) {
            Ok(outcome) => {
                assert!(
                    outcome.message.contains("Connected to Discord"),
                    "unexpected discord-rpc success message: {}",
                    outcome.message
                );
            }
            Err(drpc_err) => {
                assert!(
                    drpc_err.contains("not bundled") || drpc_err.contains("Discord"),
                    "unexpected discord-rpc validation error: {drpc_err}"
                );
            }
        }

        // Send playback hooks cleanly
        manager.on_track_change(&TrackMetadata {
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            duration_ms: 200_000,
            file_path: "/tmp/song.mp3".to_string(),
        });
        manager.on_position_change(50_000);
        manager.on_playback_end();

        manager.shutdown_all();
        let _ = std::fs::remove_dir_all(&user_dir);
    }
}
