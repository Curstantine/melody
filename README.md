# Melody

A modern, native music player built in pure Rust. It renders crisp interfaces with **[Iced](https://github.com/iced-rs/iced)**, decodes audio cleanly using **[Symphonia](https://github.com/pdeljanov/Symphonia)**, and handles audio output with **[CPAL](https://github.com/RustAudio/cpal)**.

## Workspace Layout

```
melody/
├── flake.nix             # Nix dev shell with all native graphic/audio libraries
├── themes/               # TOML color themes (hot-reloadable)
│   ├── default.toml
│   └── xcode_default_darker.toml
└── crates/
    ├── core/         # SQLite schema, track metadata parsing, disk thumbnail cache
    ├── audio/        # Audio worker thread, CPAL output stream, lock-free ring buffer
    ├── plugins/      # Plugin contracts, lyrics fetcher, event dispatchers
    └── ui/           # Iced GUI, views (Music, Albums, Artists, Now Playing, Settings)
```

## Views & Features

- **Tracks (Library)**: Fast paginated track browser with customizable page sizes, search filtering across titles, artists, and albums, and column sorting. Supports both compact table and card layouts.
- **Albums & Artists**: Aggregated views with album art covers, sortable by album title, artist, year, or track count.
- **Lyrics & Now Playing**: Full-screen player view showcasing cover artwork, track metadata, and scrollable lyrics.
- **Player Bar**: Persistent bottom controls featuring live seek scrubbing, volume controls, shuffle, repeat, track favoriting, and now-playing details.
- **Status Bar**: Live counts, active filter indicators, and scanning status.
- **Settings**:
    - **Appearance**: Select themes, configure primary display fonts, customize pagination sizes, and pick default catalog view modes.
    - **Library**: Manage indexed folders, trigger folder scans, inspect database size, and view or clear the thumbnail cache.
    - **Debug**: Audio telemetry, volume level, and active plugin counts.

## Getting Started

### Prerequisites

#### Linux (Nix / NixOS)

If you use Nix or Direnv, everything (ALSA, Fontconfig, FreeType, Wayland, Vulkan) is packaged in `flake.nix`:

```bash
# Enter the nix dev environment
nix develop

# Or with direnv
direnv allow
```

#### Linux (Standard)

Ensure you have the development libraries for ALSA, fontconfig, and pkg-config installed:

```bash
# Debian / Ubuntu
sudo apt install libasound2-dev libfontconfig1-dev pkg-config
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

- **Database**: `~/.local/share/melody/library.db` (Linux) / OS app data directory
- **Config**: `~/.config/melody/config.toml` (Linux) / OS config directory
- **Thumbnail Cache**: `~/.cache/melody/thumbnails/` (Linux) / OS cache directory
- **Themes**: `./themes/` or `~/.config/melody/themes/`

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
