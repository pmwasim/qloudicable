#!/usr/bin/env bash
# Cross-compiles with mingw-w64 and packages with NSIS. Unsigned: signing needs a
# code-signing certificate.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
T=x86_64-pc-windows-gnu
(cd app/src-tauri && cargo tauri build --target "$T" --bundles nsis)
cargo build --release -p qloudicable-cli --target "$T"
mkdir -p dist-out/windows
cp target/$T/release/bundle/nsis/*.exe dist-out/windows/
cp target/$T/release/qloudicable.exe dist-out/windows/qloudicable-cli-windows-x86_64.exe
ls -lh dist-out/windows
