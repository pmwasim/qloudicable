#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export ANDROID_HOME="${ANDROID_HOME:-$HOME/workspace/tools/android-sdk}"
export NDK_HOME=$(ls -d "$ANDROID_HOME"/ndk/* | sort -V | tail -1)
export JAVA_HOME="${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk-amd64}"
cd app/src-tauri
[ -d gen/android ] || cargo tauri android init --ci
cargo tauri android build --apk --aab "$@"
cd ../..
mkdir -p dist-out/android
kind=release
case " $* " in *" --debug "*) kind=debug ;; esac
find app/src-tauri/gen/android/app/build/outputs/apk -path "*/$kind/*" -name '*.apk' -exec cp {} dist-out/android/ \;
find app/src-tauri/gen/android/app/build/outputs/bundle -iname "*${kind}*" -name '*.aab' -exec cp {} dist-out/android/ \;

# Export and archive native debug symbols for Google Play Console crash/ANR analysis
sym_dir="app/src-tauri/gen/android/app/build/intermediates/merged_native_libs/${kind}/merge${kind^}NativeLibs/out/lib"
if [ -d "$sym_dir" ]; then
    echo "Archiving native debug symbols for Play Console..."
    (cd "$sym_dir" && zip -r -q "$OLDPWD/dist-out/android/native-debug-symbols.zip" .)
    echo "Generated dist-out/android/native-debug-symbols.zip"
fi

ls -lh dist-out/android
