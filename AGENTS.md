# AGENTS.md — Melody Architecture & Contributor Guide

Welcome to **Melody**. This guide is written for autonomous coding agents and human contributors to understand the codebase architecture, design invariants, development workflows, and extension recipes.

---

## 1. Project Overview & Core Mission

**Melody** is a lightweight, cross-platform, native desktop music player written entirely in pure Rust.

### Primary Architectural Goals

- **Low Memory Footprint**: Idles around ~30–50 MB RAM, even with 100,000+ tracks indexed (similar to classic MusicBee / foobar2000).
- **Lock-Free Audio Pipeline**: Audio decoding and output run on a dedicated worker thread, completely decoupled from the UI thread. The audio thread never blocks, allocates in realtime callbacks, or waits on UI locks.
- **Pure Rust GUI Runtime**: Uses [`iced`](https://github.com/iced-rs/iced) following The Elm Architecture (Model-View-Update). Zero C-bindings for UI rendering.
- **Embedded Storage**: Turso local SQLite engine via [`turso`](https://github.com/tursodatabase/turso) in WAL mode, utilizing windowed queries (`LIMIT ? OFFSET ?`) with async Iced tasks for viewport virtualization.
- **On-Disk Thumbnail Caching**: Album artwork is extracted, resized, and cached to disk as compact JPEG textures (`THUMB_DIM_DEFAULT = 256px`), preventing memory bloat and stutter during catalog scrolling.
- **System Font Integration & Fallbacks**: Discovers system fonts via memory-mapped `fontdb` (`cosmic-text`) without loading full font data into RAM, with font selection persisted in user configuration.
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
    ├── paths/                   # Centralized application paths, directories, atomic writes
    ├── model/                   # Domain data types (Track, TrackRow), search filters, string templates
    ├── thumbnail/               # Cover art thumbnail downscaling and disk caching
    ├── metadata/                # FFmpeg audio tags, tech info, embedded cover extraction, and library watcher
    ├── db/                      # Turso SQLite database schema, windowed queries, queue persistence, M3U playlists
    ├── audio/                   # Dedicated audio worker thread, CPAL playback, FFmpeg decoder & resampler, lock-free ring buffer
    ├── plugins/                 # Extensibility framework, host contracts, lyrics, scrobbling, media controls
    ├── updater/                 # Velopack integration, application lifecycle hooks, GitHub update checking, browser launching
    ├── theme/                   # Data-driven TOML theme definitions, palette config, font resolution, theme management
    ├── layout/                  # Declarative KDL layout templates, schema AST, layout management
    ├── queue/                   # Playback queue state machine, in-memory queue store, virtualization math
    ├── config/                  # Persistent user configuration (AppConfig), session state (AppState), navigation enums
    ├── components/              # Reusable UI widgets, vector SVG icons, dynamic widget styles, chips, layout cards
    └── ui/                      # Iced GUI entry point, Elm state machine, views, KDL layout rendering runtime
```

### Dependency Graph & Responsibilities

| Crate | Path | Responsibilities | Key Dependencies |
| :--- | :--- | :--- | :--- |
| **`paths`** | [`crates/paths`](crates/paths/src/lib.rs) | Centralized filesystem directories (`AppPaths`), application storage paths, atomic file writes (`atomic_write`). | `directories` |
| **`model`** | [`crates/model`](crates/model/src/lib.rs) | Core domain types (`Track`, `TrackRow`, `AlbumSummary`, `ArtistSummary`), catalog sort fields (`AlbumSortField`, `ArtistSortField`, `PlaylistSortField`, `ViewMode`), search filter parser (`TrackFilter`), string template engine (`format_template`), metadata string interning (`intern`). | `serde` |
| **`thumbnail`** | [`crates/thumbnail`](crates/thumbnail/src/lib.rs) | Cover art resizing, blurred background generation, disk thumbnail caching. | `image`, `md-5`, `paths`, `metadata` |
| **`metadata`** | [`crates/metadata`](crates/metadata/src/lib.rs) | FFmpeg audio tag and embedded cover art extraction, technical audio info, supported audio formats, debounced recursive filesystem watcher (`LibraryWatcher`). | `ffmpeg-next`, `model`, `encoding_rs`, `notify`, `crossbeam-channel` |
| **`db`** | [`crates/db`](crates/db/src/lib.rs) | Turso SQLite schema, migrations, windowed queries, queue persistence, library directory scanning, M3U/M3U8 playlist parsing and library resolution. | `turso`, `tokio`, `model`, `paths`, `metadata`, `encoding_rs` |
| **`audio`** | [`crates/audio`](crates/audio/src/lib.rs) | Lock-free audio worker thread (`run_audio_worker`), CPAL output stream, SPSC ring buffer (`HeapRb<f32>`), FFmpeg packet demuxing/decoding/resampling, atomic status reporting (`AudioStatus`). | `cpal`, `ffmpeg-next`, `ringbuf`, `crossbeam-channel`, `thiserror` |
| **`plugins`** | [`crates/plugins`](crates/plugins/src/lib.rs) | Plugin contracts (`Plugin`), event dispatch (`on_track_change`, `on_playback_end`, `scrobble`), lyrics fetcher (`LyricsPlugin`), media controls (Windows SMTC / Linux MPRIS). | `serde`, `serde_json`, `tracing`, `thumbnail`, `paths`, `mpris-server`, `windows` |
| **`updater`** | [`crates/updater`](crates/updater/src/lib.rs) | Velopack runtime hooks (`init`), GitHub release update checking (`check_for_updates`), browser opening helpers (`open_in_browser`). | `velopack`, `tracing`, `tokio` |
| **`theme`** | [`crates/theme`](crates/theme/src/lib.rs) | TOML theme deserialization (`ThemeConfig`), hex color parsing, system font discovery (`list_font_families`), runtime theme management (`ThemeManager`). | `iced`, `serde`, `toml`, `tracing`, `walkdir`, `paths` |
| **`layout`** | [`crates/layout`](crates/layout/src/lib.rs) | Declarative KDL layout schema (`LayoutTemplate`), format configs (`LibraryFormatConfig`, `AlbumsFormatConfig`, etc.), layout discovery and manager (`LayoutManager`). | `kdl`, `model`, `paths`, `tracing`, `iced` |
| **`queue`** | [`crates/queue`](crates/queue/src/lib.rs) | In-memory playback queue management (`QueueStore`, `QueueSummary`, `QueueSource`, `Queue`), cursor navigation, queue virtualization math. | `model`, `serde`, `tracing` |
| **`config`** | [`crates/config`](crates/config/src/lib.rs) | User preferences (`AppConfig`), session/runtime state (`AppState`), scrobbler configuration (`ScrobbleConfig`), navigation enums (`ViewTab`, `SettingsTab`, `PlaylistSection`). | `paths`, `model`, `audio`, `plugins`, `theme`, `layout`, `serde`, `toml`, `tracing` |
| **`components`** | [`crates/components`](crates/components/src/lib.rs) | Reusable UI widgets (`Marquee`, `Spinner`, `CatalogRow`, `CatalogTile`, `TrackRow`), vector SVG icons (`icons`), dynamic widget styles (`styles`), chips, settings cards. | `iced`, `serde`, `theme`, `model` |
| **`ui`** | [`crates/ui`](crates/ui/src/main.rs) | Iced GUI application entry point, Model-View-Update cycle (`App`), context menu runtime, layout view rendering runtime, and views. | `iced`, `rfd`, `directories`, `db`, `model`, `paths`, `thumbnail`, `metadata`, `audio`, `plugins`, `updater`, `theme`, `layout`, `queue`, `config`, `components` |

---

## 3. Development Environment & Toolchain

### Prerequisites (Linux / NixOS)

The repository contains a Nix flake [`flake.nix`](flake.nix) providing `alsa-lib`, `fontconfig`, `freetype`, `wayland`, `libxkbcommon`, `vulkan-loader`, and `noto-fonts-cjk-sans`.

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

- **No Blocking on UI**: The audio thread (`run_audio_worker`) spawned by [`AudioEngine`](crates/audio/src/engine.rs) runs independently of GUI frame rendering.
- **UI → Audio (Commands)**: Handled via unbounded [`crossbeam_channel::Sender<AudioCommand>`](crates/audio/src/commands.rs) encapsulated in [`AudioController`](crates/audio/src/engine.rs). Commands include [`Play`](crates/audio/src/commands.rs), [`Pause`](crates/audio/src/commands.rs), [`Seek(Duration)`](crates/audio/src/commands.rs), [`Load { path, autoplay }`](crates/audio/src/commands.rs), [`SetVolume(f32)`](crates/audio/src/commands.rs), [`SetNextTrack(Option<PathBuf>)`](crates/audio/src/commands.rs), and [`SetResampling(ResamplingConfig)`](crates/audio/src/commands.rs).
- **Audio → UI (Telemetry)**: Zero-cost atomic telemetry block [`AudioStatus`](crates/audio/src/status.rs). Uses `Arc<AtomicU64>` (`position_ms`, `duration_ms`), `Arc<AtomicBool>` (`is_playing`, `is_loaded`, `is_ended`, `gapless_transition`), and `Arc<AtomicU32>` (`volume_raw`). The UI polls this via [`iced::time::every`](crates/ui/src/app.rs) at 50ms intervals (`Message::AudioTick`).
- **SPSC Ring Buffer**: Audio samples are decoded via FFmpeg into an interleaved `f32` [`HeapRb<f32>`](crates/audio/src/engine.rs) (`RING_BUFFER_CAPACITY = 262_144`). The CPAL output callback reads directly from the consumer without heap allocations or locks via `pop_samples_with_boundary`.
- **Sample Rate & Channel Converter**: FFmpeg's [`ResamplerContext`](crates/audio/src/engine.rs) (`libswresample`) dynamically normalizes source format, channels, and sample rate into packed `f32` target channels and device rate via `create_resampler`.

### 4.2 SQLite Database & Viewport Virtualization

- **Database Engine**: [`LibraryDatabase`](crates/db/src/lib.rs) manages the `tracks` table with WAL mode and indices on `title`, `artist`, `album`, `genre`, and `file_path`.
- **Viewport Windowing**: Never execute `SELECT * FROM tracks` into a `Vec<Track>` for rendering. Viewports query only visible slices using [`get_track_rows_window`](crates/db/src/lib.rs) with `LIMIT ? OFFSET ?`.
- **Lightweight Index Headers**: Use [`TrackIdHeader { id: i64, duration_ms: u32 }`](crates/model/src/model.rs) via [`get_all_track_headers`](crates/db/src/lib.rs) if global track count, random shuffle, or total duration calculations are required.
- **Scoped Filters**: Queries accept a parsed [`TrackFilter`](crates/model/src/search.rs) rather than a raw string. [`filter_clause`](crates/db/src/lib.rs) turns it into a dynamically-parameterised `WHERE` clause (bound via `turso::Value` + `params_from_iter`).

### 4.3 Disk Thumbnail Caching

- Implemented in [`crates/thumbnail/src/lib.rs`](crates/thumbnail/src/lib.rs).
- Extracted cover images are downscaled to compact JPEG thumbnails (`THUMB_DIM_DEFAULT = 256px`, `THUMB_DIM_HIGH = 300px`) and cached on disk in `thumbnail_cache_dir()` from [`paths`](crates/paths/src/lib.rs).
- Employs deterministic hash keys factoring in path, modification time, file size, and target dimensions (`compute_thumbnail_cache_key`). Missing covers write negative hit marker files (`.none`) to avoid redundant filesystem extractions.
- Cache statistics and cache clearing are integrated in the Settings view via [`get_thumbnail_cache_stats()`](crates/thumbnail/src/lib.rs) and [`clear_thumbnail_cache()`](crates/thumbnail/src/lib.rs).

### 4.4 Vector SVG Icons

- Unicode emojis (e.g. `▶`, `⏮`, `🔊`) fail on platforms lacking emoji font fallbacks and render as question marks.
- All icons in [`crates/components/src/icons.rs`](crates/components/src/icons.rs) are inline vector SVGs rendered using [`iced::widget::svg`](crates/components/src/icons.rs).
- The [`icon(svg_str, size, color)`](crates/components/src/icons.rs) function dynamically tints SVGs using the theme's active palette.

### 4.5 Data-Driven Theming & Font Configuration

- Themes are external TOML files stored in bundled themes directory (`./themes/`) and user themes directory (`user_themes_dir()` via [`paths`](crates/paths/src/lib.rs)).
- [`ThemeConfig`](crates/theme/src/config.rs) deserializes hex color strings (`primary`, `background`, `surface`, `border`, `text`, `text_muted`, etc.).
- [`ThemeManager`](crates/theme/src/lib.rs) enables hot-swapping themes at runtime without recompiling.
- Available system fonts are enumerated using [`list_font_families()`](crates/theme/src/fonts.rs) without loading full font files into RAM. User configuration and settings are persisted via [`AppConfig`](crates/config/src/config.rs), while runtime session state (active tab, volume, shuffle, repeat, view modes) is decoupled and persisted via [`AppState`](crates/config/src/state.rs).

### 4.6 Playback Queues & Scoped Library Filtering

- **Queues live in the database, not on disk**: [`QueueStore`](crates/queue/src/lib.rs) is an in-memory view over the persisted `playback_queues`, `playback_queue_tracks` (ordered track ids) and `playback_state` tables. Playlists remain the only concept backed by the M3U file format.
- **Only the active queue is materialized**: `QueueStore` keeps a cheap [`QueueSummary`](crates/queue/src/lib.rs) for every queue and the full `Vec<TrackRow>` for the active queue only. Queues are loaded on activation via [`load_queue_tracks`](crates/db/src/lib.rs).
- **Incremental persistence**: cursor changes write a single `UPDATE` ([`set_queue_current`](crates/db/src/lib.rs)); structural edits replace the queue's membership ([`replace_queue_tracks`](crates/db/src/lib.rs)). Never re-serialize every queue on each change.
- **Every playback context becomes a queue**: playing from the library rebuilds a queue from the active scope ([`start_library_playback`](crates/ui/src/app.rs)); the queue is named after the album, artist, search query, or `All Songs`. Same-named queues are reused and refreshed rather than duplicated.
- **Scoped library search**: [`TrackFilter::parse`](crates/model/src/search.rs) supports `album:"…"`, `artist:…`, `album_artist:…`, `genre:…`, `year:…`, `title:…` plus free text. Categorical fields match exactly (so clicking an album shows only that album); free text is substring across title/artist/album.
- **`All Songs` memory caveat**: [`get_all_track_rows_filtered`](crates/db/src/lib.rs) intentionally bypasses windowing to materialize a queue. This is the one sanctioned exception to the "never load the whole library" rule; keep it out of render paths.

### 4.7 Declarative KDL Layouts & View Formatting

- **Data-Driven UI Hierarchy**: UI layouts are defined using KDL schema files in bundled layouts directory (`./layouts/`) and user layouts directory (`user_layouts_dir()` via [`paths`](crates/paths/src/lib.rs)), parsed into [`LayoutTemplate`](crates/layout/src/schema.rs).
- **Customizable String Templates**: Layouts define view item presentations under a `view-formats` node (`ViewFormatsConfig`). String substitutions and conditional brackets (`[...]`) are powered by [`format_template`](crates/model/src/template.rs). See [`docs/string-formatting.md`](docs/string-formatting.md) for full syntax details.
- **Dynamic Table Columns & Size Constraints**: Library table columns can be defined, reordered, sized (`fill-portion` or fixed `width`), and bound to sorting fields directly in the KDL template (`ColumnFormatConfig`).

---

## 5. Key Entry Points & Modules

### Application Entry & GUI Lifecycle

- [`crates/ui/src/main.rs`](crates/ui/src/main.rs): Application entry point; logging init, window configuration, and default font configuration.
- [`crates/ui/src/app.rs`](crates/ui/src/app.rs): The central [`App`](crates/ui/src/app.rs) struct, [`update`](crates/ui/src/app.rs) state machine, and [`view`](crates/ui/src/app.rs) layout composition.
- [`crates/config/src/config.rs`](crates/config/src/config.rs): Persistent configuration state [`AppConfig`](crates/config/src/config.rs) (`theme_name`, `primary_font`, `tab_order`, `page_size`, tile dimensions, `gapless`, `resampling`, `watch_library`, scrobbling credentials).
- [`crates/config/src/state.rs`](crates/config/src/state.rs): Persistent runtime application and player state [`AppState`](crates/config/src/state.rs) (`active_tab`, `settings_tab`, `volume`, `is_shuffle`, `is_repeat`, view modes, sort fields).
- [`crates/ui/src/message.rs`](crates/ui/src/message.rs): [`Message`](crates/ui/src/message.rs) enum representing all UI events and user interactions.

### Views Directory ([`crates/ui/src/views/`](crates/ui/src/views))

- [`library.rs`](crates/ui/src/views/library.rs): Virtualized track list table and grid views (`LibraryView`), search bar, sortable column headers, pagination controls.
- [`albums.rs`](crates/ui/src/views/albums.rs): Album catalog aggregated by artist and album name (`AlbumsView`) with cover art grid/list display, sorting, and per-album Play / Shuffle All actions.
- [`artists.rs`](crates/ui/src/views/artists.rs): Artist catalog (`ArtistsView`) with track count and duration summaries plus Play / Shuffle All actions.
- [`playlists.rs`](crates/ui/src/views/playlists.rs): Playlist catalog/detail (M3U) and the Queues section rendered from [`QueueStore`](crates/queue/src/lib.rs) (`PlaylistsView`).
- [`now_playing.rs`](crates/ui/src/views/now_playing.rs): Now Playing view (`NowPlayingView`) with large cover art, track metadata, and scrollable lyrics.
- [`settings.rs`](crates/ui/src/views/settings.rs): Dedicated settings tabs (`SettingsView`) for Appearance ("Appearance & interface"), Playback ("Playback preferences"), Library ("Library & storage"), and Scrobbling ("Scrobbling & social audio").
- [`player_bar.rs`](crates/ui/src/views/player_bar.rs): Persistent bottom player bar (`PlayerBar`) with cover thumbnail, track info, seek slider, playback buttons, shuffle, repeat, favorite toggle, and volume control.
- [`status_bar.rs`](crates/ui/src/views/status_bar.rs): Bottom status bar (`StatusBar`) displaying track and catalog counts, active filter indicators, and library scanning status.

---

## 6. How-To Extension Recipes

### Recipe 1: Adding a New Vector Icon

1. Open [`crates/components/src/icons.rs`](crates/components/src/icons.rs).
2. Add a `pub const MY_ICON_SVG: &str = r#"<svg xmlns=... viewBox="0 0 24 24"><path ... fill="currentColor"/></svg>"#;`.
3. Render in any view:
    ```rust
    use components::icons;
    let my_icon = icons::icon(icons::MY_ICON_SVG, 16.0, theme_cfg.primary);
    ```

### Recipe 2: Adding a New Audio Command

1. In [`crates/audio/src/commands.rs`](crates/audio/src/commands.rs), add the variant to [`AudioCommand`](crates/audio/src/commands.rs):
    ```rust
    pub enum AudioCommand {
        // ... existing variants
        SetPlaybackSpeed(f32),
    }
    ```
2. In [`crates/audio/src/engine.rs`](crates/audio/src/engine.rs), add a helper method to [`AudioController`](crates/audio/src/engine.rs):
    ```rust
    pub fn set_playback_speed(&self, speed: f32) {
        self.send(AudioCommand::SetPlaybackSpeed(speed));
    }
    ```
3. Handle the command in the worker loop (`handle_command` in [`crates/audio/src/engine.rs`](crates/audio/src/engine.rs)).

### Recipe 3: Adding a New Database Query or Index

1. In [`crates/db/src/lib.rs`](crates/db/src/lib.rs), update the schema creation in `init_schema()` if adding indices or tables.
2. Add query methods to [`LibraryDatabase`](crates/db/src/lib.rs). Always paginate or return lightweight summary types rather than loading all full track rows.
3. If the query filters tracks, extend `filter_clause` in [`crates/db/src/lib.rs`](crates/db/src/lib.rs) and the [`TrackFilter`](crates/model/src/search.rs) field set instead of appending raw SQL at the call site.
4. Add a unit test in [`crates/db/src/lib.rs`](crates/db/src/lib.rs).

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
2. The theme will be automatically discovered at launch from `./themes/` or `user_themes_dir()` (`~/.config/melody/themes/` on Linux, `%APPDATA%\curstantine\melody\themes` on Windows).

### Recipe 5: Adding a New Scoped Search Field

1. Add the field to [`TrackFilter`](crates/model/src/search.rs)
   and parse it in `TrackFilter::parse` (add the key to `split_field` known list and the
   match arm).
2. Emit the field in `to_query_string()` (for the search box round-trip) and, if
   it can name a queue, in `queue_name()`.
3. Bind it in `filter_clause` in [`crates/db/src/lib.rs`](crates/db/src/lib.rs)
   so `get_total_track_count` / `get_track_rows_window` /
   `get_all_track_rows_filtered` all honour it.
4. Add parser unit tests in [`crates/model/src/search.rs`](crates/model/src/search.rs) and a DB assertion in
   [`crates/db/src/lib.rs`](crates/db/src/lib.rs).

### Recipe 6: Adding a New Playback Queue Source

1. Add the variant to [`QueueSource`](crates/queue/src/lib.rs)
   and its `as_str` / `from_str` mappings.
2. Build the queue with `create_queue_from_tracks` in [`crates/ui/src/app.rs`](crates/ui/src/app.rs)
   (it reuses a same-named queue) and persist via `persist_active_queue_task`.
3. For library-derived scopes, extend `queue_source_for_filter` in [`crates/ui/src/app.rs`](crates/ui/src/app.rs)
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
2. The layout will be automatically discovered at launch from `./layouts/` or `user_layouts_dir()`.
3. In **Settings → Appearance**, select the new layout or click **Reload layouts from disk**.
4. See [`docs/string-formatting.md`](docs/string-formatting.md) for available placeholder keys and conditional brackets syntax.

---

## 7. Strict Rules & Anti-Patterns to Avoid

- ❌ **NEVER block the audio thread**: Do not acquire UI mutexes, perform disk scans, or make network calls in CPAL callbacks or the audio worker loop.
- ❌ **NEVER pull entire track tables into memory for rendering**: Never run `SELECT *` without `LIMIT`/`OFFSET` in render paths. Memory footprint must remain within ~30–50 MB. The single sanctioned exception is materializing a playback queue via [`get_all_track_rows_filtered`](crates/db/src/lib.rs).
- ❌ **NEVER use Unicode emojis for UI controls**: Always use vector SVGs via [`crates/components/src/icons.rs`](crates/components/src/icons.rs).
- ❌ **NEVER bypass disk thumbnail caching**: Do not load raw uncompressed audio visuals or high-res artwork directly into UI textures for list/grid items; use [`get_or_create_thumbnail`](crates/thumbnail/src/lib.rs).
- ❌ **NEVER break cross-platform support**: Use platform-agnostic abstractions or `#[cfg(target_os = "...")]` guards. Ensure path handling uses `PathBuf`, `directories`, and centralized paths from [`paths`](crates/paths/src/lib.rs).

---

## 8. Verification Checklist for Agents

Before completing any task, run the pre-commit checks and ensure they all pass:

1. `cargo fmt --all` (formatting; verify clean with `cargo fmt --all -- --check`).
2. `cargo clippy --workspace --all-targets -- -D warnings` (lints; warnings are errors).
3. `cargo test --workspace` passes all unit and integration tests.
4. `cargo check --workspace` returns 0 errors.
5. Code formatting and comments preserve existing documentation integrity.
6. Any new files or modified functions are linked in your responses using GitHub markdown format `[symbol](crates/path/to/file.rs#L123)`.
