---
title: Overview
description: What the APK Analysis Toolkit is and who it's for.
---

The APK Analysis Toolkit is a single Rust terminal UI (TUI) that bootstraps a full Android pentest / malware-analysis sandbox, instead of hand-assembling Android Studio, Frida, objection, and mitmproxy from separate guides.

## Who it's for

Security researchers, malware analysts, and pentesters who need to:

- Stand up a rootable Android emulator quickly, without guessing which system image supports root.
- Bypass SSL/certificate pinning on a target app to inspect its traffic.
- Intercept and decrypt HTTPS traffic via mitmproxy, with the device's trust store set up correctly the first time.
- Triage a suspicious APK — package identity, permissions, signing certificate — without manually running `dumpsys`, `aapt`, and `apksigner` by hand every time.

## How it's organized

Everything is reachable from one top-level menu, each a self-contained screen:

- **Emulator** — detect/list AVDs, create a rootable one, launch it, root/de-root.
- **SSL pinning bypass** — install Frida tooling, push `frida-server`, bypass/re-pin a picked package.
- **mitmproxy** — install, start the proxy + web UI, install the CA certificate as a trusted system root.
- **App manager (adb)** — list/filter packages, detailed package info with automated red-flag detection, install/uninstall/enable/disable/launch.
- **Select ADB device** — pin a specific device when more than one is attached.
- **Run full setup (template)** — chains the whole pipeline end-to-end with live progress.

Every long-running step (system image downloads, pip installs, frida-server downloads) streams real progress instead of going silent, and the binary checks GitHub for a newer release on every launch.

## Design principles

- **No silent waits.** Anything that can take more than a few seconds streams live progress.
- **Detect, don't assume.** The tool actively checks whether an Android image is rootable, whether multiple devices are attached, and whether a package's identity matches reality — rather than assuming the happy path.
- **Document the gotchas.** Every Android/adb quirk this tool had to work around (see [Troubleshooting](/APK-Analysis-Toolkit/troubleshooting/common-issues/)) is written down, not just silently patched.
