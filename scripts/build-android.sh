#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
export ANDROID_HOME="${ANDROID_HOME:?Set ANDROID_HOME to your Android SDK}"
export NDK_HOME=$(ls -d "$ANDROID_HOME"/ndk/* | sort -V | tail -1)
export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk-amd64}"
cd app/src-tauri
[ -d gen/android ] || cargo tauri android init --ci
cargo tauri android build --apk "$@"
cd ../..
mkdir -p dist-out/android
kind=release
case " $* " in *" --debug "*) kind=debug ;; esac
find app/src-tauri/gen/android/app/build/outputs/apk -path "*/$kind/*" -name '*.apk' -exec cp {} dist-out/android/ \;
ls -lh dist-out/android
