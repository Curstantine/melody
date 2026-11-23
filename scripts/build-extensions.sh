#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Load environment variables from .env file
if [[ -f "$REPO_ROOT/.env" ]]; then
    while IFS='=' read -r key val; do
        key="${key#"${key%%[![:space:]]*}"}"
        key="${key%"${key##*[![:space:]]}"}"
        [[ -z "$key" || "$key" == \#* ]] && continue
        val="${val%$'\r'}"
        val="${val#\"}"
        val="${val%\"}"
        val="${val#\'}"
        val="${val%\'}"
        if [[ -z "${!key:-}" ]]; then
            export "$key=$val"
        fi
    done < "$REPO_ROOT/.env"
fi

cargo build -p melody-ext-lastfm -p melody-ext-listenbrainz -p melody-ext-discord-rpc --target wasm32-unknown-unknown --release

TARGET_DIR="$REPO_ROOT/target/wasm32-unknown-unknown/release"
cp "$TARGET_DIR/melody_ext_lastfm.wasm" "$REPO_ROOT/extensions/lastfm/extension.wasm"
cp "$TARGET_DIR/melody_ext_listenbrainz.wasm" "$REPO_ROOT/extensions/listenbrainz/extension.wasm"
cp "$TARGET_DIR/melody_ext_discord_rpc.wasm" "$REPO_ROOT/extensions/discord-rpc/extension.wasm"

ls -lh "$REPO_ROOT"/extensions/*/extension.wasm
