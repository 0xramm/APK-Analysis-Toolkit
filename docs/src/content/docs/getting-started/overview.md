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
- Decompile an APK's resources/manifest (apktool) or its Java source (JADX) without separately installing and wiring up either tool.

## How it's organized

The main menu splits into three branches of screens:

**App Manager (adb)** — list every installed package (no name-based filtering — [search instead](/APK-Analysis-Toolkit/app-manager/listing-and-filtering/)), [detailed package info with automated red-flag detection](/APK-Analysis-Toolkit/app-manager/package-info-and-red-flags/), install/uninstall/enable/disable/launch.

**Static Analysis Setup** — everything else that inspects an APK/package without needing it running:

- **[APK tool (apktool)](/APK-Analysis-Toolkit/static-analysis/apktool/)** — decompile a picked package's resources/manifest, or recompile an edited folder.
- **[JADX](/APK-Analysis-Toolkit/static-analysis/jadx/)** — decompile a picked package straight into `jadx-gui`.
- **[MT Manager](/APK-Analysis-Toolkit/static-analysis/mt-manager/)** — get an on-device file/APK manager installed on the emulator.

**Dynamic Analysis Setup** — everything that needs a running, rooted device:

- **Emulator** — detect/list AVDs, create a rootable one, launch it, root/de-root.
- **SSL pinning bypass** — install Frida tooling, push `frida-server`, bypass/re-pin a picked package.
- **mitmproxy** — install, start the proxy + web UI, install the CA certificate as a trusted system root.
- **Run full setup (template)** — chains the whole dynamic-analysis pipeline end-to-end with live progress.

Plus, from the main menu directly: **Select ADB device** — pin a specific device when more than one is attached.

Every long-running step (system image downloads, pip installs, frida-server/apktool/JADX downloads) streams real progress instead of going silent, and the binary checks GitHub for a newer release on every launch.

## Design principles

- **No silent waits.** Anything that can take more than a few seconds streams live progress.
- **Detect, don't assume.** The tool actively checks whether an Android image is rootable, whether multiple devices are attached, and whether a package's identity matches reality — rather than assuming the happy path.
- **Document the gotchas.** Every Android/adb quirk this tool had to work around (see [Troubleshooting](/APK-Analysis-Toolkit/troubleshooting/common-issues/)) is written down, not just silently patched.
