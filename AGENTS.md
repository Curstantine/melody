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
├── layouts/                     # Declarative KDL layout templates & view formatting rules
├── docs/                        # User guides and functional documentation
└── crates/
    ├── core/                    # Centralized paths, Turso database, FFmpeg metadata extraction, thumbnail caching, template formatting
    ├── audio/                   # Dedicated audio worker thread, CPAL playback, FFmpeg decoder & resampler, lock-free ring buffer
    ├── plugins/                 # Extensibility framework, host contracts, lyrics, discord presence hooks
    └── ui/                      # Iced GUI entry point, Elm state machine, views, persistent config, theming, KDL layout runtime
```

### Dependency Graph & Responsibilities

| Crate | Path | Responsibilities | Key Dependencies |
| :--- | :--- | :--- | :--- |
| **`core`** | [`crates/core`](file:///home/curstantine/Code/Personal/melody/crates/core/src/lib.rs) | Centralized storage paths, async Turso local database schema & queries, viewport windowing, FFmpeg metadata parsing, cover art extraction & disk thumbnail caching, template string formatter engine. | `turso`, `ffmpeg-next`, `image`, `directories`, `serde`, `thiserror`, `walkdir` |
| **`audio`** | [`crates/audio`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/lib.rs) | Lock-free audio worker thread, CPAL output stream, SPSC ring buffer, FFmpeg packet demuxing/decoding/resampling, atomic status reporting. | `cpal`, `ffmpeg-next`, `ringbuf`, `crossbeam-channel`, `thiserror` |
| **`plugins`** | [`crates/plugins`](file:///home/curstantine/Code/Personal/melody/crates/plugins/src/lib.rs) | Plugin contracts, event dispatch (`on_track_change`, `on_playback_end`), lyrics fetcher, RPC stubs. | `serde`, `serde_json`, `tracing` |
| **`ui`** | [`crates/ui`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/main.rs) | Application state, Model-View-Update cycle, system font picker, SVG icons, runtime theme manager, KDL layout definitions, views, and persistent configuration. | `iced`, `rfd`, `directories`, `toml`, `kdl`, `core`, `audio`, `plugins` |

---

## 3. Development Environment & Toolchain

### Prerequisites (Linux / NixOS)

The repository contains a Nix flake [`flake.nix`](file:///home/curstantine/Code/Personal/melody/flake.nix) providing `alsa-lib`, `fontconfig`, `freetype`, `wayland`, `libxkbcommon`, `vulkan-loader`, and `noto-fonts-cjk-sans`.

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

# 3. Lint every crate and target (warnings are treated as errors)
cargo clippy --workspace --all-targets -- -D warnings

# 4. Verify formatting
cargo fmt --all -- --check

# 5. Build release binaries
cargo build --workspace --release

# 6. Run the music player
cargo run
# or: cargo run -p ui --bin melody
```

### Pre-Commit Checks

Run these before every commit; all three must pass. Clippy is configured with
`-D warnings`, so any new lint fails the build and the commit should not be made
until it is fixed.

```bash
# 1. Auto-format the tree
cargo fmt --all

# 2. Lint (deny warnings)
cargo clippy --workspace --all-targets -- -D warnings

# 3. Tests
cargo test --workspace
```

---

## 4. Key Architectural Patterns & Invariants

### 4.1 Audio Engine: Lock-Free Isolation

- **No Blocking on UI**: The audio thread (`AudioWorkerThread`) defined in [`crates/audio/src/engine.rs`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/engine.rs) runs independently of GUI frame rendering.
- **UI → Audio (Commands)**: Handled via unbounded [`crossbeam_channel::Sender<AudioCommand>`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs). Commands include [`Play`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs#L6), [`Pause`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs#L7), [`Seek(Duration)`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs#L9), [`Load(PathBuf, bool)`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs#L5), [`SetVolume(f32)`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs#L10).
- **Audio → UI (Telemetry)**: Zero-cost atomic telemetry block [`AudioStatus`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/status.rs). Uses `Arc<AtomicU64>` (position in ms), `Arc<AtomicBool>` (playing flag), `Arc<AtomicU32>` (volume), and `Arc<AtomicBool>` (track ended flag). The UI polls this via [`iced::time::every`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs#L182-L184) at 50ms intervals.
- **SPSC Ring Buffer**: Audio samples are decoded from Symphonia into an interleaved `f32` [`HeapRb<f32>`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/engine.rs#L27). The CPAL output callback reads directly from the consumer without heap allocations or locks.
- **Sample Rate & Channel Converter**: [`AudioConverter`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/resampler.rs) dynamically maps mono/stereo/surround to target device channels and performs linear interpolation resampling.

### 4.2 SQLite Database & Viewport Virtualization

- **Database Engine**: [`LibraryDatabase`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs) manages the `tracks` table with WAL mode and indices on `title`, `artist`, `album`, `genre`, and `file_path`.
- **Viewport Windowing**: Never execute `SELECT * FROM tracks` into a `Vec<Track>` for rendering. Viewports query only visible slices using [`get_track_rows_window`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs) with `LIMIT ? OFFSET ?`.
- **Lightweight Index Headers**: Use [`TrackIdHeader { id: i64, duration_ms: u32 }`](file:///home/curstantine/Code/Personal/melody/crates/core/src/model.rs) via [`get_all_track_headers`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs) if global track count, random shuffle, or total duration calculations are required.
- **Scoped Filters**: Queries accept a parsed [`TrackFilter`](file:///home/curstantine/Code/Personal/melody/crates/core/src/search.rs) rather than a raw string. [`filter_clause`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs) turns it into a dynamically-parameterised `WHERE` clause (bound via `turso::Value` + `params_from_iter`).

### 4.3 Disk Thumbnail Caching

- Implemented in [`crates/core/src/thumbnail.rs`](file:///home/curstantine/Code/Personal/melody/crates/core/src/thumbnail.rs).
- Extracted cover images are downscaled to compact JPEG thumbnails (`DEFAULT_THUMBNAIL_SIZE = 200px`) and cached on disk in `thumbnail_cache_dir()` (`~/.cache/melody/thumbnails/`).
- Employs deterministic hash keys factoring in path, modification time, file size, and target dimensions. Missing covers write negative hit marker files (`.none`) to avoid redundant filesystem extractions.
- Cache statistics and cache clearing are integrated in the Settings view via [`get_thumbnail_cache_stats()`](file:///home/curstantine/Code/Personal/melody/crates/core/src/thumbnail.rs#L125) and [`clear_thumbnail_cache()`](file:///home/curstantine/Code/Personal/melody/crates/core/src/thumbnail.rs#L148).

### 4.4 Vector SVG Icons

- Unicode emojis (e.g. `▶`, `⏮`, `🔊`) fail on platforms lacking emoji font fallbacks and render as question marks.
- All icons in [`crates/ui/src/icons.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/icons.rs) are inline vector SVGs rendered using [`iced::widget::svg`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/icons.rs#L26-L33).
- The [`icon(svg_str, size, color)`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/icons.rs#L26-L33) function dynamically tints SVGs using the theme's active palette.

### 4.5 Data-Driven Theming & Font Configuration

- Themes are external TOML files stored in [`themes/`](file:///home/curstantine/Code/Personal/melody/themes) and `~/.config/melody/themes/`.
- [`ThemeConfig`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/theme/mod.rs#L68) deserializes hex color strings (`primary`, `background`, `surface`, `border`, `text`, `text_muted`, etc.).
- [`ThemeManager`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/theme/mod.rs#L240) enables hot-swapping themes at runtime without recompiling.
- Available system fonts are enumerated using [`list_font_families()`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/font.rs#L5) without loading full font files into RAM. User configuration and settings are persisted via [`AppConfig`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/config.rs#L43), while runtime session state (active tab, volume, shuffle, repeat, view modes) is decoupled and persisted via [`AppState`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/state.rs#L19).

### 4.6 Playback Queues & Scoped Library Filtering

- **Queues live in the database, not on disk**: [`QueueStore`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/queue.rs) is an in-memory view over the persisted `playback_queues`, `playback_queue_tracks` (ordered track ids) and `playback_state` tables. Playlists remain the only concept backed by the M3U file format.
- **Only the active queue is materialized**: `QueueStore` keeps a cheap [`QueueSummary`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/queue.rs) for every queue and the full `Vec<TrackRow>` for the active queue only. Queues are loaded on activation via [`load_queue_tracks`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs).
- **Incremental persistence**: cursor changes write a single `UPDATE` ([`set_queue_current`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs)); structural edits replace the queue's membership ([`replace_queue_tracks`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs)). Never re-serialize every queue on each change.
- **Every playback context becomes a queue**: playing from the library rebuilds a queue from the active scope ([`start_library_playback`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs)); the queue is named after the album, artist, search query, or `All Songs`. Same-named queues are reused and refreshed rather than duplicated.
- **Scoped library search**: [`TrackFilter::parse`](file:///home/curstantine/Code/Personal/melody/crates/core/src/search.rs) supports `album:"…"`, `artist:…`, `album_artist:…`, `genre:…`, `year:…`, `title:…` plus free text. Categorical fields match exactly (so clicking an album shows only that album); free text is substring across title/artist/album.
- **`All Songs` memory caveat**: [`get_all_track_rows_filtered`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs) intentionally bypasses windowing to materialize a queue. This is the one sanctioned exception to the "never load the whole library" rule; keep it out of render paths.

### 4.7 Declarative KDL Layouts & View Formatting

- **Data-Driven UI Hierarchy**: UI layouts are defined using KDL schema files in [`layouts/`](file:///home/curstantine/Code/Personal/melody/layouts) and `~/.config/melody/layouts/`, parsed into [`LayoutDefinition`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/layout/schema.rs).
- **Customizable String Templates**: Layouts define view item presentations under a `view-formats` node. String substitutions and conditional brackets (`[...]`) are powered by [`format_template`](file:///home/curstantine/Code/Personal/melody/crates/core/src/template.rs). See [`docs/string-formatting.md`](file:///home/curstantine/Code/Personal/melody/docs/string-formatting.md) for full syntax details.
- **Dynamic Table Columns & Size Constraints**: Library table columns can be defined, reordered, sized (`fill-portion` or fixed `width`), and bound to sorting fields directly in the KDL template.

---

## 5. Key Entry Points & Modules

### Application Entry & GUI Lifecycle

- [`crates/ui/src/main.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/main.rs): Application entry point; logging init, window configuration, and default font configuration.
- [`crates/ui/src/app.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs): The central [`App`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs#L22-L83) struct, [`update`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs#L187) state machine, and [`view`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs#L1012) layout composition.
- [`crates/ui/src/config.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/config.rs): Persistent configuration state (`theme_name`, `primary_font`, `tab_order`, `page_size`, tile dimensions, `gapless`, scrobbling credentials).
- [`crates/ui/src/state.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/state.rs): Persistent runtime application and player state (`active_tab`, `settings_tab`, `volume`, `is_shuffle`, `is_repeat`, view modes, sort fields).
- [`crates/ui/src/message.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/message.rs): [`Message`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/message.rs#L28-L77) enum representing all UI events and user interactions.

### Views Directory ([`crates/ui/src/views/`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views))

- [`library.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/library.rs): Virtualized track list table and grid views, search bar, sortable column headers, pagination controls.
- [`albums.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/albums.rs): Album catalog aggregated by artist and album name with cover art grid/list display, sorting, and per-album Play / Shuffle All actions.
- [`artists.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/artists.rs): Artist catalog with track count and duration summaries plus Play / Shuffle All actions.
- [`playlists.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/playlists.rs): Playlist catalog/detail (M3U) and the Queues section rendered from [`QueueStore`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/queue.rs).
- [`now_playing.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/now_playing.rs): Now Playing view with large cover art, track metadata, and scrollable lyrics.
- [`settings.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/settings.rs): Dedicated settings tabs for Appearance, Playback, Library management, and Scrobbling.
- [`player_bar.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/player_bar.rs): Persistent bottom player bar with cover thumbnail, track info, seek slider, playback buttons, shuffle, repeat, favorite toggle, and volume control.
- [`status_bar.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/views/status_bar.rs): Bottom status bar displaying track and catalog counts, active filter indicators, and library scanning status.

---

## 6. How-To Extension Recipes

### Recipe 1: Adding a New Vector Icon

1. Open [`crates/ui/src/icons.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/icons.rs).
2. Add a `pub const MY_ICON_SVG: &str = r#"<svg xmlns=... viewBox="0 0 24 24"><path ... fill="currentColor"/></svg>"#;`.
3. Render in any view:
    ```rust
    use crate::icons;
    let my_icon = icons::icon(icons::MY_ICON_SVG, 16.0, theme_cfg.primary_color());
    ```

### Recipe 2: Adding a New Audio Command

1. In [`crates/audio/src/commands.rs`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs), add the variant to [`AudioCommand`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/commands.rs#L4):
    ```rust
    pub enum AudioCommand {
        // ... existing variants
        SetPlaybackSpeed(f32),
    }
    ```
2. In [`crates/audio/src/engine.rs`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/engine.rs#L42), add a helper method to [`AudioController`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/engine.rs#L42):
    ```rust
    pub fn set_playback_speed(&self, speed: f32) {
        self.send(AudioCommand::SetPlaybackSpeed(speed));
    }
    ```
3. Handle the command in the worker loop (`handle_command` in [`crates/audio/src/engine.rs`](file:///home/curstantine/Code/Personal/melody/crates/audio/src/engine.rs)).

### Recipe 3: Adding a New Database Query or Index

1. In [`crates/core/src/db.rs`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs), update the schema creation in `init_schema()` if adding indices or tables.
2. Add query methods to [`LibraryDatabase`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs). Always paginate or return lightweight summary types rather than loading all full track rows.
3. If the query filters tracks, extend [`filter_clause`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs) and the [`TrackFilter`](file:///home/curstantine/Code/Personal/melody/crates/core/src/search.rs) field set instead of appending raw SQL at the call site.
4. Add a unit test in [`crates/core/tests/db_tests.rs`](file:///home/curstantine/Code/Personal/melody/crates/core/tests/db_tests.rs).

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

    [border_radius]
    xs = 2.0
    sm = 4.0
    md = 6.0
    lg = 8.0
    xl = 16.0
    ```
2. The theme will be automatically discovered at launch from `./themes/` or `~/.config/melody/themes/`.

### Recipe 5: Adding a New Scoped Search Field

1. Add the field to [`TrackFilter`](file:///home/curstantine/Code/Personal/melody/crates/core/src/search.rs)
   and parse it in `TrackFilter::parse` (add the key to the `known` list and the
   match arm).
2. Emit the field in `to_query_string()` (for the search box round-trip) and, if
   it can name a queue, in `queue_name()`.
3. Bind it in [`filter_clause`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs)
   so `get_total_track_count` / `get_track_rows_window` /
   `get_all_track_rows_filtered` all honour it.
4. Add parser unit tests in `crates/core/src/search.rs` and a DB assertion in
   [`crates/core/tests/db_tests.rs`](file:///home/curstantine/Code/Personal/melody/crates/core/tests/db_tests.rs).

### Recipe 6: Adding a New Playback Queue Source

1. Add the variant to [`QueueSource`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/queue.rs)
   and its `as_str` / `from_str` mappings.
2. Build the queue with [`create_queue_from_tracks`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs)
   (it reuses a same-named queue) and persist via `persist_active_queue_task`.
3. For library-derived scopes, extend [`queue_source_for_filter`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/app.rs)
   and the `TrackFilter::queue_name` rules.

### Recipe 7: Adding a Custom KDL Layout

1. Create `layouts/my-layout.kdl` or `~/.config/melody/layouts/my-layout.kdl`:
    ```kdl
    layout name="My Layout" description="Custom player layout" {
        view-formats {
            library {
                column "Title" format="{title}" fill-portion=4 sort="title"
                column "Artist" format="{artist}" fill-portion=3 sort="artist"
                column "Album" format="{album}[ ({year})]" fill-portion=3 sort="album"
                column "Duration" format="{duration_label}" width=80 sort="duration"
            }
        }

        column {
            navbar height=32
            main-view fill=true
            player-bar height=72
            status-bar height=24
        }
    }
    ```
2. The layout will be automatically discovered at launch from `./layouts/` or `~/.config/melody/layouts/`.
3. In **Settings → Appearance**, select the new layout or click **Reload Layouts from Disk**.
4. See [`docs/string-formatting.md`](file:///home/curstantine/Code/Personal/melody/docs/string-formatting.md) for available placeholder keys and conditional brackets syntax.

---

## 7. Strict Rules & Anti-Patterns to Avoid

- ❌ **NEVER block the audio thread**: Do not acquire UI mutexes, perform disk scans, or make network calls in CPAL callbacks or the audio worker loop.
- ❌ **NEVER pull entire track tables into memory for rendering**: Never run `SELECT *` without `LIMIT`/`OFFSET` in render paths. Memory footprint must remain within ~30–50 MB. The single sanctioned exception is materializing a playback queue via [`get_all_track_rows_filtered`](file:///home/curstantine/Code/Personal/melody/crates/core/src/db.rs).
- ❌ **NEVER use Unicode emojis for UI controls**: Always use vector SVGs via [`icons.rs`](file:///home/curstantine/Code/Personal/melody/crates/ui/src/icons.rs).
- ❌ **NEVER bypass disk thumbnail caching**: Do not load raw uncompressed audio visuals or high-res artwork directly into UI textures for list/grid items; use [`get_or_create_thumbnail`](file:///home/curstantine/Code/Personal/melody/crates/core/src/thumbnail.rs#L44).
- ❌ **NEVER break cross-platform support**: Use platform-agnostic abstractions or `#[cfg(target_os = "...")]` guards. Ensure path handling uses `PathBuf` and `directories`.

---

## 8. Verification Checklist for Agents

Before completing any task, run the pre-commit checks and ensure they all pass:

1. `cargo fmt --all` (formatting; verify clean with `cargo fmt --all -- --check`).
2. `cargo clippy --workspace --all-targets -- -D warnings` (lints; warnings are errors).
3. `cargo test --workspace` passes all unit and integration tests.
4. `cargo check --workspace` returns 0 errors.
5. Code formatting and comments preserve existing documentation integrity.
6. Any new files or modified functions are linked in your responses using GitHub markdown format `[symbol](file:///path/to/file#Lline)`.
