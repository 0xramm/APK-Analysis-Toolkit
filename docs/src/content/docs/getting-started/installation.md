---
title: Installation
description: How to install the APK Analysis Toolkit and its prerequisites.
---

## Download a release

Grab the latest `apk-toolkit.exe` from the [Releases page](https://github.com/0xramm/APK-Analysis-Toolkit/releases). The binary checks for a newer release on every launch and updates itself automatically — see [Auto-Update & Releases](/APK-Analysis-Toolkit/operations/auto-update/).

## Build from source

Requires a [Rust toolchain](https://rustup.rs/) (stable channel):

```bash
git clone https://github.com/0xramm/APK-Analysis-Toolkit.git
cd APK-Analysis-Toolkit
cargo build --release
```

The binary is produced at `target/release/apk-toolkit.exe`.

## Prerequisites

The toolkit shells out to external tools rather than reimplementing them — install whichever ones you plan to use:

| Tool | Used for | Auto-discovered from |
|---|---|---|
| Android Studio / SDK (`adb`, `emulator`, `avdmanager`, `sdkmanager`, `aapt`, `apksigner`) | Everything emulator- and package-related | `%LOCALAPPDATA%\Android\Sdk` (or `ANDROID_SDK_ROOT`/`ANDROID_HOME`) |
| Python + `pip` | Installing `frida-tools`, `objection`, `mitmproxy` | System `PATH` |
| `curl` | Downloading `frida-server`, checking for updates | System `PATH` (Windows 10 1803+ ships one natively) |
| `openssl` | Hashing the mitmproxy CA cert for Android's trust store | System `PATH`, or Git for Windows' bundled copy |
| `xz` | Decompressing downloaded `frida-server` builds | System `PATH`, or Git for Windows' bundled copy |

You don't need all of these up front — each module checks for what it needs and tells you exactly what's missing and how to get it, the first time you use it.
