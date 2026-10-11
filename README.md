# Vibecore — Native Cross-Platform Rust Music Player

A lightweight (~30–50 MB memory footprint), high-fidelity, cross-platform music player built in pure Rust using **Iced**, **CPAL**, **Symphonia**, and **SQLite**.

---

## Architecture Overview

```
player-workspace/
├── Cargo.toml
├── themes/               # External data-driven TOML theme definitions
│   ├── catppuccin-mocha.toml
│   ├── tokyo-night.toml
│   ├── nord.toml
│   ├── dracula.toml
│   └── musicbee-classic.toml
└── crates/
    ├── player-core/      # Domain models, SQLite schema & windowed queries, Symphonia metadata extractor
    ├── player-audio/     # Dedicated audio worker thread, CPAL audio pipeline, lock-free ring buffer
    ├── player-plugins/   # Extension engine, plugin API contracts, lyrics & rich presence hooks
    └── player-ui/        # Iced Elm architecture GUI, live theme engine, virtualized track list
```

### 1. `player-core` (Data & Storage)
- **Embedded SQLite** with WAL mode and indexing on `artist`, `album`, `title`, and `file_path`.
- **Viewport Virtualization & Pagination**: Windowed querying using `LIMIT ? OFFSET ?`. Never pulls whole libraries into memory, maintaining a tiny ~30–50 MB RAM footprint even on 100,000+ track libraries.
- **Lightweight Index Headers**: In-memory `TrackIdHeader { id: i64, duration_ms: u32 }` struct for fast global shuffles and counts.
- **Metadata Extraction**: Pure-Rust decoding via `symphonia` for MP3, FLAC, AAC, Vorbis, Opus, WAV, ALAC, and MP4 tags (ID3v1, ID3v2, Vorbis Comments, MP4 atoms).

### 2. `player-audio` (Audio Engine)
- **Zero UI-Audio Blocking**: Audio processing never runs on the UI thread and never acquires UI locks.
- **Lock-Free Communication**: Commands (`Play`, `Pause`, `Seek`, `Load`, `SetVolume`, `Stop`) sent over unbounded `crossbeam-channel`.
- **Atomic Status Polling**: `AudioStatus` exposes `Arc<AtomicU64>` (position in ms), `Arc<AtomicBool>` (play/pause state), `Arc<AtomicU32>` (volume), and `Arc<AtomicBool>` (track finished) for zero-cost, lock-free UI polling.
- **Lock-Free Ring Buffer**: Interleaved `f32` SPSC ring buffer (`ringbuf`) feeds the `cpal::Stream` audio callback.
- **Universal Resampling & Channel Mapping**: Handles mono, stereo, and multi-channel streams across any source and destination sample rate with linear interpolation.

### 3. `player-plugins` (Extensibility)
- **Host Contracts**:
  - `on_track_change(TrackMetadata)`
  - `on_playback_end()`
  - `fetch_lyrics(artist, title) -> Option<String>`
  - `discord_presence_update(TrackMetadata)`
- **Fault-Isolated Execution**: Plugin errors are caught and logged without crashing the host application.

### 4. `player-ui` (GUI & Live Theming)
- **Elm Architecture**: Clean Model-View-Update cycle implemented with `iced`.
- **Full CJK Font Support**: Automatically discovers and registers Chinese, Japanese, and Korean fonts from system directories (Fontconfig / Windows `Fonts` / macOS `Fonts`), user config (`~/.config/player/fonts`), and `./fonts/` directory, preventing corrupt/tofu glyphs.
- **Data-Driven Dynamic Theming**: TOML themes loaded at runtime from `~/.config/player/themes/` and `./themes/`. Switches themes instantaneously without recompiling.
- **Interactive Controls**:
  - Track seek bar with live scrubbing.
  - Master volume slider.
  - Instant search filtering.
  - Column sorting (Title, Artist, Album, Duration).
  - Folder scanner with native system dialogs (`rfd`).
  - Catalog views for Tracks, Albums, Artists, and Lyrics.

---

## Running the Player

```bash
# Run the application
cargo run -p player-ui

# Run test suite across all workspace crates
cargo test --workspace
```
