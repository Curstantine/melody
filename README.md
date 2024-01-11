# Melody

[WIP] Standalone cross-platform music player.

## Features

- Theming support.
- Extensible plugin support.
- Niche features like re-encoding and tagging.

## Development

If you are interested in contributing to this project, make sure to go through the [specification](docs/spec.md) first.

### Prerequisites

- `pnpm >=8.x.x`
- `rustc >= 1.71.1`
- [`cargo-vcpkg`](https://github.com/mcgoo/cargo-vcpkg)

#### Compiling ffmpeg

FFmpeg is compiled through the vcpkg definition in the [`backend/Cargo.toml`](./backend//Cargo.toml).

Before running `cargo vcpkg build`, run the setup script specific to your platform.

```sh
# Windows
./scripts/setup_windows.ps1

# unix
./scripts/setup_unix.sh
```

### Starting

1. Run `pnpm i` in the workspace root.
2. Run `pnpm build:packages` to build the projects in `packages/`.
3. Build the project with `cargo build`.
4. Start the backend by running `pnpm run dev:backend` with `RUST_LOG = "none,backend=debug"` env.
5. Start the web server with `pnpm dev:app`.
