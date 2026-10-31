# QloudiCable

USB-C cable diagnostics by qloudihub.com, for Windows, macOS, Android and Ubuntu.

**Early release:** QloudiCable has not yet been tested with real USB-C cables on any platform, and
the macOS build is untested. See [docs/STATUS.md](docs/STATUS.md).

Design and reasoning: [docs/DESIGN.md](docs/DESIGN.md). Current state: [docs/STATUS.md](docs/STATUS.md).

## Develop

```bash
cargo test                        # all logic tests
cargo run -p qloudicable-cli         # CLI report for this machine
cargo run -p qloudicable-cli -- --help
cargo tauri dev -c app/src-tauri/tauri.conf.json   # desktop app
```

Preview the UI with sample data by serving `app/ui` and opening `?demo=mac`, `windows`,
`android` or `empty`.

## Build

```bash
scripts/build-linux.sh     # .deb and AppImage
scripts/build-windows.sh   # NSIS installer, cross-compiled here
scripts/build-android.sh   # APK; needs ANDROID_HOME set to an Android SDK with an NDK
scripts/build-macos.sh     # run on a Mac; Apple's tools don't run on Linux
```

Outputs go to `dist-out/`.

## Licence

Apache License 2.0: see [LICENSE](LICENSE) and [NOTICE](NOTICE). The licence doesn't cover the
QloudiCable name or logo, so modified versions need their own.
