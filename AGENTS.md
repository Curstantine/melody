# AGENTS.md — Melody Architecture & Contributor Guide

Welcome to **Melody**. This guide is written for autonomous coding agents and human contributors to understand the codebase architecture, design invariants, development workflows, and extension recipes.

---

## 1. Project Overview & Core Mission

**Melody** is a lightweight, cross-platform, native desktop music player written entirely in pure Rust.

### Primary Architectural Goals

- **Low Memory Footprint**: Idles around ~30–50 MB RAM, even with 100,000+ tracks indexed (similar to classic MusicBee / foobar2000).
- **Lock-Free Audio Pipeline**: Audio decoding and output run on a dedicated worker thread, completely decoupled from the UI thread. The audio thread never blocks, allocates in realtime callbacks, or waits on UI locks.
- **Pure Rust GUI Runtime**: Uses [`iced`](https://github.com/iced-rs/iced) (v0.13) following The Elm Architecture (Model-View-Update). Zero C-bindings for UI rendering.
- **Embedded Storage**: Turso local SQLite engine via [`turso`](https://github.com/tursodatabase/turso) in WAL mode, utilizing windowed queries (`LIMIT ? OFFSET ?`) with async Iced tasks for viewport virtualization.
- **On-Disk Thumbnail Caching**: Album artwork is extracted, resized, and cached to disk as compact JPEG textures, preventing memory bloat and stutter during catalog scrolling.
- **System Font Integration & Fallbacks**: Discovers system fonts via memory-mapped `fontdb` with font selection persisted in user configuration.
- **Reliable Vector UI Icons**: Pure SVG icons with runtime palette coloring—no Unicode emojis or font-dependent glyphs.

---

## 2. Workspace Organization & Crate Boundaries

```
melody/
├── Cargo.toml
├── flake.nix
├── themes/                      # Runtime data-driven TOML theme definitions
└── crates/
    ├── core/                    # Centralized paths, Turso database, FFmpeg metadata extraction, thumbnail caching
    ├── audio/                   # Dedicated audio worker thread, CPAL playback, FFmpeg decoder & resampler, lock-free ring buffer
    ├── plugins/                 # Extensibility framework, host contracts, lyrics, discord presence hooks
    └── ui/                      # Iced GUI entry point, Elm state machine, views, persistent config, theming
```

### Dependency Graph & Responsibilities

| Crate | Path | Responsibilities | Key Dependencies |
| :--- | :--- | :--- | :--- |
| **`core`** | [`crates/core`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/lib.rs) | Centralized storage paths, async Turso local database schema & queries, viewport windowing, FFmpeg metadata parsing, cover art extraction & disk thumbnail caching. | `turso`, `ffmpeg-next`, `image`, `directories`, `serde`, `thiserror`, `walkdir` |
| **`audio`** | [`crates/audio`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/lib.rs) | Lock-free audio worker thread, CPAL output stream, SPSC ring buffer, FFmpeg packet demuxing/decoding/resampling, atomic status reporting. | `cpal`, `ffmpeg-next`, `ringbuf`, `crossbeam-channel`, `thiserror` |
| **`plugins`** | [`crates/plugins`](file:///home/curstantine/Code/Experimental/melody/crates/plugins/src/lib.rs) | Plugin contracts, event dispatch (`on_track_change`, `on_playback_end`), lyrics fetcher, RPC stubs. | `serde`, `serde_json`, `tracing` |
| **`ui`** | [`crates/ui`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/main.rs) | Application state, Model-View-Update cycle, system font picker, SVG icons, runtime theme manager, views, and persistent configuration. | `iced`, `rfd`, `directories`, `toml`, `core`, `audio`, `plugins` |

---

## 3. Development Environment & Toolchain

### Prerequisites (Linux / NixOS)

The repository contains a Nix flake [`flake.nix`](file:///home/curstantine/Code/Experimental/melody/flake.nix) providing `alsa-lib`, `fontconfig`, `freetype`, `wayland`, `libxkbcommon`, `vulkan-loader`, and `noto-fonts-cjk-sans`.

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
cargo check --workspace

# 2. Run entire test suite across all crates
cargo test --workspace

# 3. Build release binaries
cargo build --workspace --release

# 4. Run the music player
cargo run
# or: cargo run -p ui --bin melody
```

---

## 4. Key Architectural Patterns & Invariants

### 4.1 Audio Engine: Lock-Free Isolation

- **No Blocking on UI**: The audio thread (`AudioWorkerThread`) defined in [`crates/audio/src/engine.rs`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/engine.rs) runs independently of GUI frame rendering.
- **UI → Audio (Commands)**: Handled via unbounded [`crossbeam_channel::Sender<AudioCommand>`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs). Commands include [`Play`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs#L6), [`Pause`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs#L7), [`Seek(Duration)`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs#L9), [`Load(PathBuf, bool)`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs#L5), [`SetVolume(f32)`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs#L10).
- **Audio → UI (Telemetry)**: Zero-cost atomic telemetry block [`AudioStatus`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/status.rs). Uses `Arc<AtomicU64>` (position in ms), `Arc<AtomicBool>` (playing flag), `Arc<AtomicU32>` (volume), and `Arc<AtomicBool>` (track ended flag). The UI polls this via [`iced::time::every`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/app.rs#L182-L184) at 50ms intervals.
- **SPSC Ring Buffer**: Audio samples are decoded from Symphonia into an interleaved `f32` [`HeapRb<f32>`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/engine.rs#L27). The CPAL output callback reads directly from the consumer without heap allocations or locks.
- **Sample Rate & Channel Converter**: [`AudioConverter`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/resampler.rs) dynamically maps mono/stereo/surround to target device channels and performs linear interpolation resampling.

### 4.2 SQLite Database & Viewport Virtualization

- **Database Engine**: [`LibraryDatabase`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/db.rs) manages `tracks` table with WAL mode and indices on `title`, `artist`, `album`, `genre`, and `file_path`.
- **Viewport Windowing**: Never execute `SELECT * FROM tracks` into a `Vec<Track>`. Viewports query only visible slices using [`get_tracks_window`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/db.rs#L115-L165) with `LIMIT ? OFFSET ?`.
- **Lightweight Index Headers**: Use [`TrackIdHeader { id: i64, duration_ms: u32 }`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/model.rs#L34-L38) via [`get_all_track_headers`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/db.rs#L210-L225) if global track count, random shuffle, or total duration calculations are required.

### 4.3 Disk Thumbnail Caching

- Implemented in [`crates/core/src/thumbnail.rs`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/thumbnail.rs).
- Extracted cover images are downscaled to compact JPEG thumbnails (`DEFAULT_THUMBNAIL_SIZE = 200px`) and cached on disk in `thumbnail_cache_dir()` (`~/.cache/melody/thumbnails/`).
- Employs deterministic hash keys factoring in path, modification time, file size, and target dimensions. Missing covers write negative hit marker files (`.none`) to avoid redundant filesystem extractions.
- Cache statistics and cache clearing are integrated in the Settings view via [`get_thumbnail_cache_stats()`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/thumbnail.rs#L125) and [`clear_thumbnail_cache()`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/thumbnail.rs#L148).

### 4.4 Vector SVG Icons

- Unicode emojis (e.g. `▶`, `⏮`, `🔊`) fail on platforms lacking emoji font fallbacks and render as question marks.
- All icons in [`crates/ui/src/icons.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/icons.rs) are inline vector SVGs rendered using [`iced::widget::svg`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/icons.rs#L26-L33).
- The [`icon(svg_str, size, color)`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/icons.rs#L26-L33) function dynamically tints SVGs using the theme's active palette.

### 4.5 Data-Driven Theming & Font Configuration

- Themes are external TOML files stored in [`themes/`](file:///home/curstantine/Code/Experimental/melody/themes) and `~/.config/melody/themes/`.
- [`ThemeConfig`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/theme/mod.rs#L68) deserializes hex color strings (`primary`, `background`, `surface`, `border`, `text`, `text_muted`, etc.).
- [`ThemeManager`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/theme/mod.rs#L240) enables hot-swapping themes at runtime without recompiling.
- Available system fonts are enumerated using [`list_font_families()`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/font.rs#L5) without loading full font files into RAM. The chosen font and view settings are persisted via [`AppConfig`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/config.rs#L13).

---

## 5. Key Entry Points & Modules

### Application Entry & GUI Lifecycle

- [`crates/ui/src/main.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/main.rs): Application entry point; logging init, window configuration, and default font configuration.
- [`crates/ui/src/app.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/app.rs): The central [`App`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/app.rs#L22-L83) struct, [`update`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/app.rs#L187) state machine, and [`view`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/app.rs#L1012) layout composition.
- [`crates/ui/src/config.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/config.rs): Persistent configuration state (`active_tab`, `theme_name`, `primary_font`, `library_view_mode`, `page_size`, volume, etc.).
- [`crates/ui/src/message.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/message.rs): [`Message`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/message.rs#L28-L77) enum representing all UI events and user interactions.

### Views Directory ([`crates/ui/src/views/`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views))

- [`library.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/library.rs): Virtualized track list table and grid views, search bar, sortable column headers, pagination controls.
- [`albums.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/albums.rs): Album catalog aggregated by artist and album name with cover art grid/list display and sorting.
- [`artists.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/artists.rs): Artist catalog with track count and duration summaries.
- [`now_playing.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/now_playing.rs): Now Playing view with large cover art, track metadata, and scrollable lyrics.
- [`settings.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/settings.rs): Dedicated settings tabs for Appearance, Library management, and Debug telemetry.
- [`player_bar.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/player_bar.rs): Persistent bottom player bar with cover thumbnail, track info, seek slider, playback buttons, shuffle, repeat, favorite toggle, and volume control.
- [`status_bar.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/views/status_bar.rs): Bottom status bar displaying track and catalog counts, active filter indicators, and library scanning status.

---

## 6. How-To Extension Recipes

### Recipe 1: Adding a New Vector Icon

1. Open [`crates/ui/src/icons.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/icons.rs).
2. Add a `pub const MY_ICON_SVG: &str = r#"<svg xmlns=... viewBox="0 0 24 24"><path ... fill="currentColor"/></svg>"#;`.
3. Render in any view:
    ```rust
    use crate::icons;
    let my_icon = icons::icon(icons::MY_ICON_SVG, 16.0, theme_cfg.primary_color());
    ```

### Recipe 2: Adding a New Audio Command

1. In [`crates/audio/src/commands.rs`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs), add the variant to [`AudioCommand`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/commands.rs#L4):
    ```rust
    pub enum AudioCommand {
        // ... existing variants
        SetPlaybackSpeed(f32),
    }
    ```
2. In [`crates/audio/src/engine.rs`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/engine.rs#L42), add a helper method to [`AudioController`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/engine.rs#L42):
    ```rust
    pub fn set_playback_speed(&self, speed: f32) {
        self.send(AudioCommand::SetPlaybackSpeed(speed));
    }
    ```
3. Handle the command in the worker loop (`handle_command` in [`crates/audio/src/engine.rs`](file:///home/curstantine/Code/Experimental/melody/crates/audio/src/engine.rs)).

### Recipe 3: Adding a New Database Query or Index

1. In [`crates/core/src/db.rs`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/db.rs#L40), update the schema creation in `init_schema()` if adding indices.
2. Add query methods to [`LibraryDatabase`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/db.rs#L18). Always paginate or return lightweight summary types rather than loading all full track rows.
3. Add a unit test in [`crates/core/tests/db_tests.rs`](file:///home/curstantine/Code/Experimental/melody/crates/core/tests/db_tests.rs).

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
2. The theme will be automatically discovered at launch from `./themes/` or `~/.config/melody/themes/`.

---

## 7. Strict Rules & Anti-Patterns to Avoid

- ❌ **NEVER block the audio thread**: Do not acquire UI mutexes, perform disk scans, or make network calls in CPAL callbacks or the audio worker loop.
- ❌ **NEVER pull entire track tables into memory**: Never run `SELECT * FROM tracks` without `LIMIT` and `OFFSET`. Memory footprint must remain within ~30–50 MB.
- ❌ **NEVER use Unicode emojis for UI controls**: Always use vector SVGs via [`icons.rs`](file:///home/curstantine/Code/Experimental/melody/crates/ui/src/icons.rs).
- ❌ **NEVER bypass disk thumbnail caching**: Do not load raw uncompressed audio visuals or high-res artwork directly into UI textures for list/grid items; use [`get_or_create_thumbnail`](file:///home/curstantine/Code/Experimental/melody/crates/core/src/thumbnail.rs#L44).
- ❌ **NEVER break cross-platform support**: Use platform-agnostic abstractions or `#[cfg(target_os = "...")]` guards. Ensure path handling uses `PathBuf` and `directories`.

---

## 8. Verification Checklist for Agents

Before completing any task, ensure:

1. `cargo check --workspace` returns 0 errors.
2. `cargo test --workspace` passes all unit and integration tests.
3. Code formatting and comments preserve existing documentation integrity.
4. Any new files or modified functions are linked in your responses using GitHub markdown format `[symbol](file:///path/to/file#Lline)`.
