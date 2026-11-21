#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

cargo build -p melody-ext-lastfm -p melody-ext-listenbrainz --target wasm32-unknown-unknown --release

TARGET_DIR="$REPO_ROOT/target/wasm32-unknown-unknown/release"
cp "$TARGET_DIR/melody_ext_lastfm.wasm" "$REPO_ROOT/extensions/lastfm/extension.wasm"
cp "$TARGET_DIR/melody_ext_listenbrainz.wasm" "$REPO_ROOT/extensions/listenbrainz/extension.wasm"

ls -lh "$REPO_ROOT"/extensions/*/extension.wasm
