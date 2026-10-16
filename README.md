# Melody

A modern, native music player built in pure Rust. It renders crisp interfaces with **[Iced](https://github.com/iced-rs/iced)**, decodes audio cleanly with **[FFmpeg](https://github.com/zmwangx/rust-ffmpeg)**, resamples with **[Rubato](https://github.com/HEnquist/rubato)**, outputs audio using **[CPAL](https://github.com/RustAudio/cpal)**, and manages persistent library data with an embedded **[Turso](https://github.com/tursodatabase/turso)**.

## Workspace Layout

```
melody/
├── flake.nix             # Nix dev shell with all native graphic/audio libraries
├── themes/               # TOML color themes (hot-reloadable)
│   ├── default.toml
│   ├── midnight_amber.toml
│   └── xcode_default_darker.toml
└── crates/
    ├── core/         # Embedded Turso SQLite engine, FFmpeg metadata extraction, thumbnail caching, M3U playlists, scoped search filters, queue persistence
    ├── audio/        # Audio worker thread, CPAL playback, FFmpeg decoding & Rubato resampling, lock-free ring buffer
    ├── plugins/      # Plugin contracts, MPRIS (Linux), Scrobbling (ListenBrainz & Last.fm), lyrics, event dispatchers
    └── ui/           # Iced GUI, views (Library, Albums, Artists, Playlists, Now Playing, Settings), DB-backed playback queues
```

## Views & Features

- **Tracks (Library)**: Fast, virtualized track browser with customizable page sizes, column sorting, and scoped search (`artist:…`, `album:"…"`, `album_artist:…`, `genre:…`, `year:…`, `title:…` plus free text). Clicking an album or artist tiles into its exact scope.
- **Albums & Artists**: Aggregated catalog views with cover art, flexible grid/list layouts, and configurable tile dimensions. Album tiles carry play/shuffle actions; artist tiles are click-to-filter, with play/shuffle kept in list rows.
- **Playlists & Queues**: M3U/M3U8 playlists (Melody-managed files plus external references) with import, rename, and play support. DB-backed playback queues (`playback_queues` / `playback_queue_tracks`) — every playback context (album, artist, search, playlist, all songs) becomes a named, reusable queue with a virtualized detail view.
- **Lyrics & Now Playing**: Dedicated player view showcasing artwork, track metadata, and scrollable lyrics with configurable cover sizes.
- **Player Bar**: Persistent bottom controls featuring live seek scrubbing, volume controls, shuffle, repeat, track favoriting, and now-playing details.
- **Status Bar**: Live catalog counts, active filter indicators, and library scanning progress.
- **Settings**:
    - **Appearance**: Select themes, configure primary system fonts, customize pagination limits, and set card/tile dimensions for catalog views.
    - **Library**: Manage indexed folders, rescan metadata, inspect database file size, and view or clear the thumbnail cache.
    - **Scrobbling**: Built-in scrobbler support for ListenBrainz and Last.fm with customizable scrobble thresholds and now-playing updates.
- **Integrations**:
    - **Linux MPRIS**: Native desktop media controls, track metadata, and playback status integration via D-Bus.
- **Icons**: Inline vector SVGs sourced from [Boxicons](https://boxicons.com) (MIT), tinted at render time — no emoji or font-dependent glyphs.

## Getting Started

### Prerequisites

#### Linux (Nix / NixOS)

If you use Nix or Direnv, everything (FFmpeg, ALSA, Fontconfig, FreeType, Wayland, Vulkan) is packaged in `flake.nix`:

```bash
# Enter the nix dev environment
nix develop

# Or with direnv
direnv allow
```

#### Linux (Standard)

Ensure you have the development libraries for FFmpeg, ALSA, fontconfig, and pkg-config installed:

```bash
# Debian / Ubuntu
sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libasound2-dev libfontconfig1-dev pkg-config
```

#### macOS / Windows

Standard Rust toolchain (`cargo`, `rustc`).

### Building & Running

Run the player:

```bash
cargo run
# or: cargo run -p ui --bin melody
```

Run tests across all crates:

```bash
cargo test --workspace
```

Build an optimized release binary:

```bash
cargo build --workspace --release
```

## Configuration & Data Paths

Melody stores settings and cached data according to standard OS conventions:

- **Database**: `~/.local/share/melody/library.db` (Linux) / OS app data directory — tracks, playlist file refs, and playback queues
- **Config**: `~/.config/melody/config.toml` (Linux) / OS config directory
- **Playlists**: `~/.local/share/melody/playlists/` (Linux) / OS app data directory — Melody-managed `.m3u8` files (external playlists may live anywhere)
- **Thumbnail Cache**: `~/.cache/melody/thumbnails/` (Linux) / OS cache directory
- **Themes**: `./themes/` or `~/.config/melody/themes/`

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

