#!/usr/bin/env bash
# Run on a Mac with Xcode command line tools and Rust. Without a Developer ID, the
# app is ad-hoc signed and Gatekeeper warns other users.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo install tauri-cli --version "^2" --locked
(cd app/src-tauri && cargo tauri build --bundles dmg)
cargo build --release -p qloudicable-cli
mkdir -p dist-out/macos
cp target/release/bundle/dmg/*.dmg dist-out/macos/
cp target/release/qloudicable dist-out/macos/qloudicable-cli-macos
ls -lh dist-out/macos
