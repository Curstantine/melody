# Melody Extensions Guide

Melody supports sandboxed **WebAssembly** extensions that users can install, enable, configure, reload, and uninstall at runtime from **Settings -> Extensions**.

---

## 1. Package Layout & Discovery Locations

Each extension lives in its own directory containing a manifest (`extension.toml`) and a compiled WebAssembly module (`extension.wasm` by default):

```text
my-extension/
├── extension.toml
└── extension.wasm
```

Melody discovers extensions from two locations at startup and whenever you click **Reload from disk**:

1. **Bundled extensions**: `./extensions/` beside the executable (or `../share/melody/extensions/` on Linux, `../Resources/extensions/` on macOS).
2. **User-installed extensions**:
   - **Windows**: `%APPDATA%\curstantine\melody\data\extensions` (or `melody.debug\data\extensions` in debug builds)
   - **Linux**: `~/.local/share/melody/extensions`
   - **macOS**: `~/Library/Application Support/moe.curstantine.melody/extensions`

If a user extension has the same `id` as a bundled extension, the user extension takes precedence.

---

## 2. Manifest Schema (`extension.toml`)

```toml
id = "com.example.my-extension"
name = "My Extension"
version = "0.1.0"
description = "Short description displayed under the extension name in Settings."
authors = ["Your Name"]
repository = "https://github.com/example/my-extension"
entry = "extension.wasm"
api_version = 1
permissions = ["playback-events", "network"]
allowed_hosts = ["api.example.com", "*.example.org"]

[[settings]]
key = "api_token"
label = "API token"
description = "Token used to authenticate outbound requests"
type = "text"
default = ""
placeholder = "Enter API token..."
```

### Manifest Fields

| Field | Required | Description |
| :--- | :--- | :--- |
| `id` | Yes | Unique identifier (`1..=64` ASCII chars: `A-Z`, `a-z`, `0-9`, `.`, `_`, `-`, no `..`). |
| `name` | Yes | Display name shown as the section card header under **Settings -> Extensions**. |
| `version` | Yes | Semantic version string (e.g. `"0.1.0"`). |
| `description` | No | Summary displayed inside the extension card. |
| `authors` | No | List of author names. |
| `repository` | No | Optional repository URL. |
| `entry` | No | Relative path to the `.wasm` module inside the extension folder (default `"extension.wasm"`). |
| `api_version` | No | Host ABI version (currently `1`). |
| `permissions` | No | Explicit list of capabilities requested by this extension. |
| `allowed_hosts` | No | Optional hostname allowlist (exact or `*.suffix`) enforced on `network` HTTP requests. |
| `allowed_ipc` | No | Named pipe / Unix socket allowlist (single-segment `*` wildcard); required when `"local-ipc"` is declared. |
| `settings` | No | Array of `[[settings]]` option tables rendered in **Settings -> Extensions**. |

---

## 3. Permissions & Sandbox Enforcement

Extensions run inside an isolated [`wasmi`](https://github.com/wasmi-labs/wasmi) sandbox with fuel metering (`5,000,000` fuel per call) and a `16 MB` linear memory cap. Every permission must be declared in `extension.toml`:

| Permission | What it grants | Enforcement |
| :--- | :--- | :--- |
| `"playback-events"` | Receive `on_track_change`, `on_playback_end`, `on_playback_status_change`, `on_position_change` (throttled to 1 Hz), `on_seeked`, `on_volume_change`, `on_repeat_change`, and `on_shuffle_change`. | Events are only dispatched if declared. |
| `"lyrics"` | Receive `fetch_lyrics` requests for tracks lacking local sidecar or embedded lyrics. | Queried only if declared. |
| `"network"` | Make outbound HTTP/HTTPS requests via `melody_extension_sdk::host::http_request`. | Host linker refuses to link `host_http_request` and installer rejects modules importing it unless declared. Optional `allowed_hosts` restricts target domains. |
| `"player-control"` | Send `ExtensionPlayerAction` commands (`play`, `pause`, `next`, `previous`, `seek_offset_ms`, `set_position_ms`, `set_volume`, `set_shuffle`, `set_repeat`) via `host::dispatch_player_action`. | Host linker refuses to link `host_dispatch_player_action` and installer rejects modules importing it unless declared. |
| `"local-ipc"` | Connect to local named pipes or Unix domain sockets via `melody_extension_sdk::host::ipc_connect` / `ipc_write` / `ipc_read` / `ipc_close` and read `host::pid()`. | Host linker refuses to link `host_ipc_*` and `host_pid`, and installer rejects modules importing them unless declared. Requires `allowed_ipc` patterns in `extension.toml`. |

Newly installed extensions always start **disabled** so you can inspect their declared permissions before toggling them on.

---

## 4. Settings Schema Options

Options can be defined statically in `extension.toml` (`[[settings]]`) and/or returned dynamically at startup from `Extension::settings_schema(&self)`.

Supported option `type` values:

- **`toggle`**: Boolean switch (`default = true | false`).
- **`text`**: Single-line text input (`default`, optional `placeholder`).
- **`number`**: Slider with numeric readout (`default`, `min`, `max`, optional `step` defaulting to `1.0`, optional `unit`).
- **`select`**: Dropdown menu (`default`, `options = [{ value = "...", label = "..." }]`).
- **`action`**: Button (`button_label`) that calls `Extension::on_action(action_id, settings)` and displays the returned `Ok(msg)` / `Err(msg)` banner inside the extension's card.

Persisted settings values are stored in `config.toml` under `[extensions."<extension-id>".settings]`.

---

## 5. Authoring an Extension in Rust

1. Create a library crate with `crate-type = ["cdylib", "rlib"]` depending on `melody-extension-sdk`.
2. Implement `melody_extension_sdk::Extension` and register your struct with `melody_extension_sdk::register_extension!(MyExtension);`:

```rust
use melody_extension_sdk::{host, register_extension, Extension, SettingsMap, TrackInfo};

#[derive(Default)]
struct MyExtension {
    greeting: String,
}

impl Extension for MyExtension {
    fn init(&mut self, settings: SettingsMap) -> Result<(), String> {
        self.on_settings_change(settings)
    }

    fn on_settings_change(&mut self, settings: SettingsMap) -> Result<(), String> {
        if let Some(msg) = settings.get_str("greeting") {
            self.greeting = msg.to_string();
        }
        Ok(())
    }

    fn on_track_change(&mut self, track: &TrackInfo) -> Result<(), String> {
        host::log_info(format!("{}: {} - {}", self.greeting, track.artist, track.title));
        Ok(())
    }
}

register_extension!(MyExtension);
```

3. Compile to WebAssembly:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/my_extension.wasm extension.wasm
```

4. To distribute or install:
   - **Archive**: Zip `extension.toml` and `extension.wasm` into `my-extension.zip` or `my-extension.mext` (either at the archive root or inside a single top-level folder) and install via **Settings -> Extensions -> Install archive (.zip / .mext)**.
   - **Local folder**: Click **Settings -> Extensions -> Install from folder** and pick the folder containing `extension.toml` and `extension.wasm`.
