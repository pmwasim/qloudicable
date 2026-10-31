#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
(cd app/src-tauri && cargo tauri build --bundles deb,appimage)
cargo build --release -p qloudicable-cli
mkdir -p dist-out/linux
cp target/release/bundle/deb/*.deb target/release/bundle/appimage/*.AppImage dist-out/linux/
cp target/release/qloudicable dist-out/linux/qloudicable-cli-linux-x86_64
ls -lh dist-out/linux
