# QloudiCable status (v0.1.0, 2026-10-31)

Build scripts put installers in `dist-out/`, which is gitignored.

| Platform | Built here | Run for real | Tested with real USB-C data |
| --- | --- | --- | --- |
| Ubuntu | Yes: .deb, AppImage, CLI | Yes: app and CLI on one desktop PC | No: that PC has no typec ports |
| Windows | Yes: NSIS installer, CLI | CLI under Wine only; installer and app not run | No |
| macOS | Yes: GitHub Actions (`.github/workflows/macos.yml`) | CLI on an Apple silicon MacBook and a CI runner; app not opened | Partly: phone over USB-C, no e-marked cable or charger |
| Android | Yes: unsigned release APK for 4 ABIs | x86_64 emulator only (API 35); no phone | No |

Logic is tested with fixtures built from the kernel ABI and PD tables, not captures from
real hardware. Rust type-checks: CLI for macOS; both for Windows.

## Android: what was and wasn't verified
An emulator proves the app and the Kotlin-to-Rust bridge work. Its battery is simulated (fixed
5.00 V and 0.90 A, whatever the plug state), so every watt figure seen there is fake.

Verified on a Pixel 6 profile emulator (API 35, WebView 124, 411 dp wide). Unless noted, this
ran on a copy of the final release APK, re-signed with the local debug key because an unsigned
APK won't install; R8 didn't break the plugin:
- The app opens and the UI renders, with no crash in logcat, light or dark, including switching
  theme while open.
- Kotlin `facts` reaches Rust with no plugin or capability error: the battery percent on screen
  follows `adb emu power capacity`. `capabilities/default.json` works on mobile as it stands.
- Android snapshot: the Drive write test card is hidden (it keys on `platform == android`), and
  the Android wording and the `BatteryManager` source show. Percent, status and plug state follow
  `adb emu power` live.
- Copy report reaches the system clipboard. The charge test completes.
- Unplugging mid charge test reports a dropout.
- The header fits at 360 dp wide, the app's minimum.

Not verified:
- Any real phone: charging readings, the sign and units of battery current, or how accuracy
  varies by maker. The 20 mA noise floor in `android.rs` is untested on hardware.
- arm64-v8a, armeabi-v7a and x86 builds: they compile and package, but have never been run.
- Android versions other than API 35, including the minSdk of 24 and the targetSdk of 37.
- USB device listing: the emulator has no USB devices, so that path has only run empty.
- Whether any phone exposes typec sysfs. The emulator's SELinux policy denies it, which the app
  handles, but it logs three audit denials per poll (every 3 s). Harmless log noise.
- The `dialog` plugin on Android: the only thing that calls it, the Drive write test, is hidden.
- Signed-APK install, Play Store checks and battery use.

## macOS: what was and wasn't verified
On an Apple silicon MacBook with a phone attached over USB-C, the CLI found the right port and
phone, and its battery values and the absence of a cable chip matched `ioreg`. Not verified: opening
the app, an e-marked cable, a charger's power profiles, and MagSafe. USB devices aren't yet linked
to their port on macOS, so data-speed findings can't name the cable there.

## Before shipping
1. Run Android on a real phone, with the battery below 80% and a known charger.
3. Run every platform with real cables: an e-marked one, an unmarked one, a charger.
4. Sign: Windows code-signing certificate, Apple Developer ID, Android release keystore.
