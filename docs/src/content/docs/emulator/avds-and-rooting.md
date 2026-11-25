---
title: AVDs & Rooting
description: Detecting, creating, launching, rooting, and de-rooting Android virtual devices.
---

## Why rootability matters

Android system images come in two flavors:

- **Play Store / production images** — locked down like a retail phone. `adb root` is disabled unconditionally; there is no workaround. Any package with `PlayStore.enabled=true` in its AVD `config.ini` falls in this category.
- **Google APIs (no Play Store) images** — development builds where `adb root` works natively.

Anything requiring root — pushing `frida-server` so it can actually spawn/attach to processes, writing the mitmproxy CA into `/system`— only works on the second kind.

## Detect & list AVDs

Lists every AVD `emulator -list-avds` reports, with a condensed summary per entry (Android version, ABI, RAM) and a rootability tag:

```
[OK] AVDs found:
    - pentest-avd  (Android 33, x86_64, 2G RAM, rootable)
    - Medium_Phone  (Android 37.0, x86_64, 2816MB RAM, Play Store, no root)
```

Also reports how many devices are currently online, warning if more than one is attached (see [Multi-Device Selection](/APK-Analysis-Toolkit/operations/multi-device/)).

## Create new AVD

Creates `pentest-avd` using `system-images;android-33;google_apis;x86_64` — Google APIs, no Play Store, x86_64 — the combination that's both rootable and broadly compatible. The system-image download streams live percentage progress the whole way through (`Fetch remote repository... → Downloading ...zip... → Unzipping...`), since this can be a multi-minute, gigabyte-scale download.

Only runs if no rootable AVD already exists — the **Run full setup** pipeline checks this automatically rather than assuming the first AVD it finds is usable.

## Launch existing AVD

Boots whichever AVD is preferred (rootable one first, falling back to whatever exists), always with `-writable-system` so a later `remount` can succeed. Clears stale `adb` server state first, since ghost device entries can otherwise interfere with targeting.

## Root device / De-root device

Thin wrappers around `adb root` / `adb unroot`. On a Play Store image, `adb root` fails with `adbd cannot run as root in production builds` — the tool detects this exact message and tells you to use **Create new AVD** instead of just reporting a bare failure.

```
[FAIL] adb root: adbd cannot run as root in production builds
This device's image doesn't support root (it's a Play Store system image —
adb root is disabled unconditionally on these, no workaround exists).
Run 'Create new AVD': it builds a Google APIs image (no Play Store, Android 13,
x86_64) that's rootable and has the widest package/API compatibility for testing.
```
