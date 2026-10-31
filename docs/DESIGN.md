# QloudiCable design

## Purpose
Tell people, in plain words, what a USB-C cable can do and what is holding their
charging or data speed back, on Windows, macOS, Android and Ubuntu.

Reference: WhatCable (macOS only). QloudiCable's differences are that it runs on four
platforms, adds active tests, and labels how it knows each thing.

## The name
"QloudiCable" follows the Qloudi<Thing> pattern of qloudihub.com's other products, such as
QloudiPDF.

## Core principle: never present a blank as a verdict
Operating systems differ enormously in what they reveal, so:

- Each finding carries its evidence: `reported`, `measured` or `inferred`.
- Each snapshot carries `capabilities`, stating what the OS hides and why.
- A missing cable record blames the cable only where the OS could have shown one.

## What each platform can see

| Data | macOS (Apple silicon) | Linux | Windows | Android |
| --- | --- | --- | --- | --- |
| Cable e-marker | Yes | If the port driver publishes it | No public API | Hidden, unless sysfs happens to be readable |
| Charger profiles | Yes | If the driver publishes it | No public API | Hidden, same exception |
| Link speed now | Yes | Yes | Yes | No |
| Device's top speed | From bcdUSB | From bcdUSB | Yes, exact flags (best of the four) | From bcdUSB, where allowed |
| Measured power | Battery | Battery | Battery | Battery |

Windows and Android hide e-marker data from apps at the OS level. No software-only
tool can change that, so hardware would be the only route. The search for alternatives
found BLE and USB hardware testers, and no software route.

## Architecture

```
app/ui (HTML/CSS/JS, no build step)      crates/qloudicable-cli
        | Tauri commands                         |
app/src-tauri ---- tauri-plugin-qloudicable (Kotlin on Android)
        |                                        |
crates/qloudicable-backend  linux | macos | windows | android
        |
crates/qloudicable-core  model, PD decoding, diagnostics, test statistics
```

- `qloudicable-core` has no OS code, so its logic is tested with fixtures on any machine.
- `qloudicable-backend` fills one `Snapshot` per OS. The parsing is separate from the FFI
  where possible: Linux takes a root path, and macOS parsing lives in `macos_parse.rs`.
- Rust with Tauri 2 gives one codebase, one UI and small binaries across all four.

## macOS sources
IORegistry services: `AppleHPMInterfaceType10/11/12/18` and `AppleTCControllerType10/11`
(ports), `IOPortTransportComponentCCUSBPDSOP` and `...SOPp` (partner and cable identity),
`IOPortFeaturePowerSource` (charger), `IOUSBHostDevice`, `AppleSmartBattery`. Properties are
read one key at a time, because the bulk read can abort the process if a service disappears
mid-read (found in WhatCable issue #181).

## Active tests
- **Charge test:** samples power each second for 30 s and classifies it as steady,
  fluctuating or dropping out.
- **Drive write test:** writes uniquely named temporary data with `create_new`, forces it
  to disk, deletes it. Existing files can't be overwritten. Desktop only for now: Android
  needs the system folder picker wired in.

## Privacy
Nothing is sent anywhere. There is no telemetry, no update check and no account. Device
serial numbers are not collected. A report leaves the device only if the user copies it.

## Deliberately not in v0.1
- Auto-update.
- Cable fingerprint database (brand and model lookup).
- Translations (English only).
- Windows product names: reading them means querying each device, which WhatCable found
  upsets some KVM switches and hubs. Windows shows vendor names instead.
