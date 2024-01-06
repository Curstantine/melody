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

#### Compiling ffmpeg

The backend requires the ffmpeg binary to be linked to it statically. Use the [`scripts/build_dependencies`](./scripts/build_dependencies/README.md) to automatically do it for you, or follow rsmpeg's [guide](https://github.com/larksuite/rsmpeg?tab=readme-ov-file#ffmpeg-compilation).

### Starting

1. Run `pnpm i` in the workspace root.
2. Run `pnpm build:packages` to build the projects in `packages/`.
3. Build the project with `cargo build`.
4. Start the backend by running `pnpm run dev:backend` with `RUST_LOG = "none,backend=debug"` env.
5. Start the web server with `pnpm dev:app`.
