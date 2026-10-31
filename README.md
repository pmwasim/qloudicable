# QloudiCable

USB-C cable diagnostics by qloudihub.com, for Windows, macOS, Android and Ubuntu.

**Early release:** not yet tested with an e-marked cable or a charger on any platform, and the Windows app has never been run on Windows. See [docs/STATUS.md](docs/STATUS.md).

Design and reasoning: [docs/DESIGN.md](docs/DESIGN.md). Current state: [docs/STATUS.md](docs/STATUS.md). Releasing and rollback: [docs/RELEASING.md](docs/RELEASING.md).

## Commands

- `qloudicable`: Report on every port and device
- `qloudicable --json`: Same report as JSON
- `qloudicable --watch`: Reprint whenever the report changes (Ctrl+C to stop)
- `qloudicable --charge-test [S]`: Sample charging power for S seconds (default 30)
- `qloudicable --resistance [S]`: Sample dynamic load steps to estimate loop resistance (default 15)
- `qloudicable --sentinel [S]`: Monitor connection stability & micro-dropouts for S seconds (default 30)
- `qloudicable --matrix-test`: Run synthetic hardware scenario simulation matrix
- `qloudicable --virtual-test [P]`: Simulate a virtual cable against chargers/devices (default: all profiles)
- `qloudicable --self-check`: Check this report's internal consistency
- `qloudicable --write-test DIR`: Time a 256 MB write to DIR, then delete the test file
- `qloudicable --version`: Show version
- `qloudicable --help`: Show help

Note: `--matrix-test` and `--virtual-test` use simulated cables, not real ones.

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
