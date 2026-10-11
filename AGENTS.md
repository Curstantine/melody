# AGENTS.md — Vibecore Architecture & Contributor Guide

Welcome to **Vibecore**. This guide is written for autonomous coding agents and human contributors to understand the codebase architecture, design invariants, development workflows, and extension recipes.

---

## 1. Project Overview & Core Mission

**Vibecore** is a lightweight, cross-platform, native desktop music player written entirely in pure Rust.

### Primary Architectural Goals
- **Low Memory Footprint**: Idles around ~30–50 MB RAM, even with 100,000+ tracks indexed (similar to MusicBee / foobar2000).
- **Lock-Free Audio Pipeline**: Audio processing and decoding run on a dedicated worker thread, completely decoupled from the UI thread. The audio thread never blocks, allocates in realtime callbacks, or waits on UI locks.
- **Pure Rust GUI Runtime**: Uses [`iced`](https://github.com/iced-rs/iced) (v0.13) following The Elm Architecture (Model-View-Update). Zero C-bindings for UI rendering.
- **Embedded Storage**: SQLite via [`rusqlite`](https://github.com/rusqlite/rusqlite) in WAL mode, utilizing windowed queries (`LIMIT ? OFFSET ?`) for viewport virtualization.
- **Corrupt-Free Internationalization (CJK)**: Dynamic cross-platform discovery and registration of Chinese, Japanese, and Korean fonts.
- **Reliable Vector UI Icons**: Pure SVG icons with runtime palette coloring—no Unicode emojis or font-dependent glyphs.

---

## 2. Workspace Organization & Crate Boundaries

```
vibecore/
├── Cargo.toml
├── flake.nix
├── themes/                      # Runtime data-driven TOML theme definitions
└── crates/
    ├── player-core/             # Database schema, pagination queries, metadata extraction
    ├── player-audio/            # Dedicated audio worker thread, CPAL playback, Symphonia decoder, lock-free ring buffer
    ├── player-plugins/          # Extensibility framework, host contracts, lyrics, discord presence
    └── player-ui/               # Iced GUI entry point, Elm state machine, views, theming, font discovery
```

### Dependency Graph & Responsibilities

| Crate | Path | Responsibilities | Key Dependencies |
| :--- | :--- | :--- | :--- |
| **`player-core`** | [`crates/player-core`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/lib.rs) | SQLite schema, viewport windowing queries, lightweight headers, Symphonia metadata parsing, cover art extraction. | `rusqlite`, `symphonia`, `serde`, `thiserror`, `walkdir` |
| **`player-audio`** | [`crates/player-audio`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/lib.rs) | Lock-free audio worker thread, CPAL output stream, SPSC ring buffer, sample format conversion, atomic status reporting. | `cpal`, `symphonia`, `ringbuf`, `crossbeam-channel`, `thiserror` |
| **`player-plugins`** | [`crates/player-plugins`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-plugins/src/lib.rs) | Plugin contracts, event dispatch (`on_track_change`, `on_playback_end`), lyrics fetcher, RPC stubs. | `serde`, `serde_json`, `tracing` |
| **`player-ui`** | [`crates/player-ui`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/main.rs) | Application state, Model-View-Update cycle, CJK font loader, SVG icons, runtime theme manager, views. | `iced`, `rfd`, `directories`, `toml`, `player-core`, `player-audio`, `player-plugins` |

---

## 3. Development Environment & Toolchain

### Prerequisites (Linux / NixOS)
The repository contains a Nix flake [`flake.nix`](file:///home/curstantine/Code/Experimental/vibecore/flake.nix) providing `alsa-lib`, `fontconfig`, `freetype`, `wayland`, `libxkbcommon`, `vulkan-loader`, and `noto-fonts-cjk-sans`.

When executing commands in the shell:
```bash
# Load direnv flake environment (if not already active)
eval "$(direnv export bash)"

# Or enter nix development shell directly
nix develop
```

### Core Build & Verification Commands

```bash
# 1. Type-check all workspace crates
eval "$(direnv export bash)" && cargo check --workspace

# 2. Run entire test suite across all crates
eval "$(direnv export bash)" && cargo test --workspace

# 3. Build release binaries
eval "$(direnv export bash)" && cargo build --workspace --release

# 4. Run the music player
eval "$(direnv export bash)" && cargo run -p player-ui
```

---

## 4. Key Architectural Patterns & Invariants

### 4.1 Audio Engine: Lock-Free Isolation
- **No Blocking on UI**: The audio thread (`AudioWorkerThread`) defined in [`crates/player-audio/src/engine.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/engine.rs) runs independently of GUI frame rendering.
- **UI → Audio (Commands)**: Handled via unbounded [`crossbeam_channel::Sender<AudioCommand>`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs). Commands include [`Play`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs#L6), [`Pause`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs#L7), [`Seek(Duration)`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs#L9), [`Load(PathBuf, bool)`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs#L5), [`SetVolume(f32)`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs#L10).
- **Audio → UI (Telemetry)**: Zero-cost atomic telemetry block [`AudioStatus`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/status.rs). Uses `Arc<AtomicU64>` (position in ms), `Arc<AtomicBool>` (playing flag), `Arc<AtomicU32>` (volume), and `Arc<AtomicBool>` (track ended flag). The UI polls this via [`iced::time::every`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/app.rs#L126-L128) at 50ms intervals.
- **SPSC Ring Buffer**: Audio samples are decoded from Symphonia into an interleaved `f32` [`HeapRb<f32>`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/engine.rs#L27). The CPAL output callback reads directly from the consumer without heap allocations or locks.
- **Sample Rate & Channel Converter**: [`AudioConverter`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/resampler.rs) dynamically maps mono/stereo/surround to target device channels and performs linear interpolation resampling.

### 4.2 SQLite Database & Viewport Virtualization
- **Database Engine**: [`LibraryDatabase`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/db.rs) manages `tracks` table with WAL mode and indices on `title`, `artist`, `album`, `genre`, and `file_path`.
- **Viewport Windowing**: Never execute `SELECT * FROM tracks` into a `Vec<Track>`. Viewports query only visible slices using [`get_tracks_window`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/db.rs#L115-L165) with `LIMIT ? OFFSET ?`.
- **Lightweight Index Headers**: Use [`TrackIdHeader { id: i64, duration_ms: u32 }`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/model.rs#L34-L38) via [`get_all_track_headers`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/db.rs#L210-L225) if global track count, random shuffle, or total duration calculations are required.

### 4.3 CJK Font Discovery
- CJK glyph corruption ("tofu" or square boxes) is prevented by [`crates/player-ui/src/font.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/font.rs).
- At application startup in [`main.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/main.rs#L21-L36), [`load_cjk_font_data()`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/font.rs#L10) inspects:
  - Linux `fontconfig` via `fc-match` (`sans-serif:lang=ja`, `lang=zh`, `lang=ko`), Nix store font paths, `/usr/share/fonts/`.
  - Windows `%WINDIR%\Fonts` (`msyh.ttc`, `malgun.ttf`, `meiryo.ttc`, `msgothic.ttc`).
  - macOS `/System/Library/Fonts` and `/Library/Fonts` (`PingFang.ttc`, `AppleSDGothicNeo.ttc`, `Hiragino Sans GB.ttc`).
  - User config dir `~/.config/player/fonts/` and `./fonts/`.
- Loaded font byte slices are registered with `iced::application(...).font(bytes)`.

### 4.4 Vector SVG Icons
- Unicode emojis (e.g. `▶`, `⏮`, `🔊`) fail on platforms lacking emoji font fallbacks and render as question marks.
- All icons in [`crates/player-ui/src/icons.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/icons.rs) are inline vector SVGs rendered using [`iced::widget::svg`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/icons.rs#L26-L33).
- The [`icon(svg_str, size, color)`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/icons.rs#L26-L33) function dynamically tints SVGs using the theme's active palette.

### 4.5 Cover Art Extraction
- Implemented in [`extract_cover_art(path)`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/metadata.rs#L154-L219).
- Scans container metadata via `symphonia::core::meta::Visual` (supporting ID3v2 APIC, FLAC picture blocks, Vorbis Comments, and MP4 cover atoms).
- If absent, checks directory siblings for standard cover images (`cover.jpg`, `folder.png`, `front.jpg`, `album.jpg`, etc.).
- Handled in UI via [`iced::widget::image(Handle::from_bytes(vec))`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/player_bar.rs#L24-L28).

### 4.6 Data-Driven Theming
- Themes are external TOML files stored in [`themes/`](file:///home/curstantine/Code/Experimental/vibecore/themes) and `~/.config/player/themes/`.
- [`ThemeConfig`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/theme/config.rs) deserializes hex color strings (`primary`, `background`, `surface`, `border`, `text`, `text_muted`, etc.).
- [`ThemeManager`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/theme/manager.rs) enables hot-swapping themes at runtime without recompiling.

---

## 5. Key Entry Points & Modules

### Application Entry & GUI Lifecycle
- [`crates/player-ui/src/main.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/main.rs): Application entry point; font loading and window configuration.
- [`crates/player-ui/src/app.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/app.rs): The central [`App`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/app.rs#L20-L57) struct, [`update`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/app.rs#L131) state machine, and [`view`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/app.rs#L421) layout composition.
- [`crates/player-ui/src/message.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/message.rs): [`Message`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/message.rs#L13-L41) enum representing all UI events and user interactions.

### Views Directory ([`crates/player-ui/src/views/`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views))
- [`sidebar.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/sidebar.rs): Navigation tabs, folder scan button, theme picker, library stats.
- [`player_bar.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/player_bar.rs): Persistent bottom player bar with cover thumbnail, track info, seek slider, playback buttons, and volume control.
- [`library.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/library.rs): Virtualized track list table, search bar, sortable column headers, pagination controls.
- [`albums.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/albums.rs): Album catalog aggregated by artist and album name.
- [`artists.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/artists.rs): Artist catalog with track count and duration summaries.
- [`lyrics.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/views/lyrics.rs): Now Playing view with large cover art, metadata, and scrollable lyrics.

---

## 6. How-To Extension Recipes

### Recipe 1: Adding a New Vector Icon
1. Open [`crates/player-ui/src/icons.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/icons.rs).
2. Add a `pub const MY_ICON_SVG: &str = r#"<svg xmlns=... viewBox="0 0 24 24"><path ... fill="currentColor"/></svg>"#;`.
3. Render in any view:
   ```rust
   use crate::icons;
   let my_icon = icons::icon(icons::MY_ICON_SVG, 16.0, theme_cfg.primary_color());
   ```

### Recipe 2: Adding a New Audio Command
1. In [`crates/player-audio/src/commands.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs), add the variant to [`AudioCommand`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/commands.rs#L4):
   ```rust
   pub enum AudioCommand {
       // ... existing variants
       SetPlaybackSpeed(f32),
   }
   ```
2. In [`crates/player-audio/src/engine.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/engine.rs#L42), add a helper method to [`AudioController`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/engine.rs#L42):
   ```rust
   pub fn set_playback_speed(&self, speed: f32) {
       self.send(AudioCommand::SetPlaybackSpeed(speed));
   }
   ```
3. Handle the command in the worker loop (`handle_command` in [`crates/player-audio/src/engine.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-audio/src/engine.rs)).

### Recipe 3: Adding a New Database Query or Index
1. In [`crates/player-core/src/db.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/db.rs#L40), update the schema creation in `init_schema()` if adding indices.
2. Add query methods to [`LibraryDatabase`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/src/db.rs#L18). Always paginate or return lightweight summary types rather than loading all full track rows.
3. Add a unit test in [`crates/player-core/tests/db_tests.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-core/tests/db_tests.rs).

### Recipe 4: Adding a New Theme
1. Create `themes/my-theme.toml`:
   ```toml
   [theme]
   name = "My Theme"
   primary = "#FF79C6"
   background = "#191A21"
   surface = "#21222C"
   surface_hover = "#282A36"
   border = "#44475A"
   text = "#F8F8F2"
   text_muted = "#6272A4"
   accent = "#BD93F9"
   slider_active = "#FF79C6"
   slider_rail = "#44475A"
   ```
2. The theme will be automatically discovered at launch from `./themes/` or `~/.config/player/themes/`.

---

## 7. Strict Rules & Anti-Patterns to Avoid

- ❌ **NEVER block the audio thread**: Do not acquire UI mutexes, perform disk scans, or make network calls in CPAL callbacks or the audio worker loop.
- ❌ **NEVER pull entire track tables into memory**: Never run `SELECT * FROM tracks` without `LIMIT` and `OFFSET`. Memory footprint must remain within ~30–50 MB.
- ❌ **NEVER use Unicode emojis for UI controls**: Always use vector SVGs via [`icons.rs`](file:///home/curstantine/Code/Experimental/vibecore/crates/player-ui/src/icons.rs).
- ❌ **NEVER discard CJK font loading**: Always preserve `font::load_cjk_font_data()` in `main.rs` so East Asian song titles render without corruption.
- ❌ **NEVER break cross-platform support**: Use platform-agnostic abstractions or `#[cfg(target_os = "...")]` guards. Ensure path handling uses `PathBuf` and `directories`.

---

## 8. Verification Checklist for Agents

Before completing any task, ensure:
1. `eval "$(direnv export bash)" && cargo check --workspace` returns 0 errors.
2. `eval "$(direnv export bash)" && cargo test --workspace` passes all unit and integration tests.
3. Code formatting and comments preserve existing documentation integrity.
4. Any new files or modified functions are linked in your responses using GitHub markdown format `[symbol](file:///path/to/file#Lline)`.
