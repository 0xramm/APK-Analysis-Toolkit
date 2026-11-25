<p align="center">
  <img src="assets/logo.png" alt="APK Analysis Toolkit" width="420">
</p>

<h1 align="center">APK Analysis Toolkit</h1>

<p align="center">
  A Rust terminal UI that bootstraps a rooted Android sandbox, bypasses SSL pinning,
  intercepts traffic through mitmproxy, and automatically flags malware-style red flags
  in any package — instead of hand-assembling Android Studio, Frida, mitmproxy, apktool,
  and JADX from a dozen separate guides.
</p>

<p align="center">
  <a href="https://0xramm.github.io/APK-Analysis-Toolkit/"><b>Documentation</b></a>
  ·
  <a href="https://github.com/0xramm/APK-Analysis-Toolkit/releases/latest"><b>Latest Release</b></a>
</p>

<p align="center">
  <a href="https://github.com/0xramm/APK-Analysis-Toolkit/releases/latest">
    <img src="https://img.shields.io/github/v/release/0xramm/APK-Analysis-Toolkit?label=latest%20release&color=blue" alt="Latest release">
  </a>
</p>

---

## What it does

One TUI, three menus:

**App manager (adb)** — list every installed package (no name-based filtering — type-to-filter search instead), pull detailed package info with automated red-flag detection (name impersonation, dangerous permission combos, signing certificate), install/uninstall/enable/disable/launch.

**Static Analysis Setup** — everything else that inspects an APK/package without needing it running:
- **APK tool (apktool)** — decompile a picked package's resources/manifest, or recompile an edited folder.
- **JADX** — decompile a picked package straight into `jadx-gui`.
- **MT Manager** — get an on-device file/APK manager installed on the emulator.

**Dynamic Analysis Setup** — everything that needs a running, rooted device:
- **Emulator** — detect/list AVDs (tagged rootable or not), create a rootable one, launch it, root/de-root.
- **SSL pinning bypass** — push `frida-server`, bypass pinning for a picked package via Frida's codeshare, track and reverse active sessions.
- **mitmproxy** — install, start the proxy + web UI (with a working auth token — no 403s), install its CA as a trusted system root (including the dm-verity reboot cycle most guides skip).
- **Run full setup (template)** — chains the whole dynamic pipeline end-to-end with live progress.

Every long-running step (system image downloads, pip installs, `frida-server`/apktool/JADX downloads) streams real progress instead of going silent, and the app checks GitHub for a newer release on every launch.

Full walkthrough, every gotcha it had to work around, and why: **[read the docs](https://0xramm.github.io/APK-Analysis-Toolkit/)**.

## Tools it wraps

This toolkit doesn't reimplement any of these — it detects, installs, configures, and drives them:

| Tool | Purpose |
|---|---|
| **Android SDK** (`adb`, `emulator`, `avdmanager`, `sdkmanager`, `aapt`, `apksigner`) | Emulator lifecycle, package inspection, manifest/signature checks |
| **Frida** (`frida-tools`, `frida-server`) + **objection** | Dynamic instrumentation and SSL-pinning bypass |
| **mitmproxy** (`mitmweb`) | HTTPS interception |
| **apktool** | APK resource/manifest decompilation and recompilation |
| **JADX** | Java/Kotlin source decompilation |
| **MT Manager** | On-device file/APK manager |
| `curl`, `openssl`, `xz`, `tar` | Downloads, certificate hashing, archive extraction — auto-discovered even when not on a shell's `PATH` |

## Install

Grab the latest `apkat-setup.exe` from **[Releases](https://github.com/0xramm/APK-Analysis-Toolkit/releases/latest)** and run it — it installs `apkat.exe` and adds it to your `PATH`, so `apkat` works from any terminal. From then on it checks for and installs updates on its own.

Or build from source:

```bash
git clone https://github.com/0xramm/APK-Analysis-Toolkit.git
cd APK-Analysis-Toolkit
cargo build --release
```

See **[Installation](https://0xramm.github.io/APK-Analysis-Toolkit/getting-started/installation/)** for prerequisites (Android SDK, Python/pip, Java).

## Safety note

This kit is meant for **authorized pentesting, malware research, and CTFs** on devices/emulators you control. Always analyze untrusted APKs inside the emulator — ideally snapshotted and network-isolated from anything sensitive — never on a personal device.

## License

MIT.
