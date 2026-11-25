---
title: Package Info & Red Flags
description: Automated malware-style triage for any installed package.
---

`Package info` pulls together everything extractable about a package from the device and the APK itself, and — unlike a plain `dumpsys` dump — automatically flags what's actually suspicious, right at the top of the report.

## What it gathers

1. **`dumpsys package <pkg>`** — identity (version, install/update times, installer), install paths, flags, requested/runtime permissions. The full raw output is appended at the end so nothing is left out even if a future Android version changes field names the curated sections look for.
2. **The APK itself**, pulled once via `adb pull` and reused for both of the next two checks:
   - **`aapt dump badging`** — the app's real **label** (what you actually see on the device, as opposed to its package ID), its launchable activity class name (handy if you want to Frida-hook the entry point directly), and manifest-declared SDK versions.
   - **`apksigner verify --print-certs`** — the real signing certificate (DN + SHA-256). A package name or permission list can lie; the signer is cryptographic.

Package name and app label are shown immediately, before anything else — a mismatch between the two (package claims to be a system component, label says something else entirely) is exactly the kind of thing worth seeing without scrolling.

## Automated red flags

The report opens with a `!! RED FLAGS !!` section (if any fire) before the routine detail sections:

| Check | Fires when |
|---|---|
| Name/location impersonation | Package name starts with `com.google.*`/`com.android.*`/`android.*`, but `codePath` is `/data/app/...` (sideloaded) instead of `/system`, `/product`, `/vendor`, or `/apex` |
| Placeholder version | `versionCode=1` and/or `versionName=1.0` |
| Debuggable | `DEBUGGABLE` flag set — unusual for a release build |
| Sensitive permissions | 14 tracked permissions, each with a plain-English reason — `INSTALL_PACKAGES`/`REQUEST_DELETE_PACKAGES`/`OVERRIDE_PACKAGE_VERIFICATION` (dropper capability), `QUERY_ALL_PACKAGES`, `BIND_ACCESSIBILITY_SERVICE` (banking-trojan overlay technique), `SYSTEM_ALERT_WINDOW`, `BIND_DEVICE_ADMIN`, SMS/contacts/call-log/camera/microphone access |
| AOSP test-key signature | Signing DN matches AOSP's publicly known `testkey` (`build/target/product/security/testkey.*` in the AOSP source) — legitimate Google/OEM production apps are **never** signed with it |

## Worked example

During development, this exact flow caught a real sample: an APK downloaded as a fake Indian vehicle-registration app ("mParivahan") whose manifest package name was `com.google.android.gmscore` — deliberately impersonating Google Play Services' package ID so it would sit unnoticed among platform noise (and specifically slip past the system-package filter above).

```
Package: com.google.android.gmscore
App label: NxtGen mParivahan

-- !! RED FLAGS (9) !! --
  ! Package name 'com.google.android.gmscore' impersonates a system/Google component
    (app label: "NxtGen mParivahan"), but is installed as a regular sideloaded app
    (codePath=/data/app/...). Real system packages live in /system, /product, or
    /vendor — never /data/app.
  ! Placeholder version (versionCode=1 / versionName=1.0) — unusual for any app
    with real update history.
  ! Marked DEBUGGABLE — unusual for a release build; expands attack surface.
  ! Requests INSTALL_PACKAGES — can silently install other apps (dropper capability).
  ! Requests REQUEST_INSTALL_PACKAGES — can request installing other apps.
  ! Requests REQUEST_DELETE_PACKAGES — can request removing other apps.
  ! Requests OVERRIDE_PACKAGE_VERIFICATION — can bypass Play Protect / package verification.
  ! Requests QUERY_ALL_PACKAGES — can enumerate every installed app.
  ! Signed with AOSP's publicly known "testkey" — legitimate Google/OEM production
    apps are NEVER signed with this key. Near-definitive evidence of a repackaged
    or tampered APK.
```

Real Google Play Services is a system component baked into `/system`, is never a standalone downloadable `.apk`, and is never signed with AOSP's public test key. The permission combination (`INSTALL_PACKAGES` + `OVERRIDE_PACKAGE_VERIFICATION` + `QUERY_ALL_PACKAGES`) is a classic **dropper/loader** profile — its job is most likely installing a second-stage payload while wearing a trusted name, counting on neither the user nor casual tooling to look closely.
