---
title: Common Issues
description: Real problems found and fixed during development, and why they happened.
---

These are genuine bugs this toolkit hit (and fixed) while being built against a real Android SDK install and a real emulator — not hypotheticals. Documented here because the root causes are useful to understand even if you never touch this codebase.

## "more than one device/emulator"

**Symptom:** any `adb` command fails outright once a second device entry appears — including stale "ghost" entries left behind by a previous emulator session that never fully cleared.

**Cause:** `adb` refuses to guess which device you mean once more than one is listed, even if only one is actually online.

**Fix:** every `adb` call is routed through a shared targeting function that resolves `-s <serial>` explicitly — see [Multi-Device Selection](/APK-Analysis-Toolkit/operations/multi-device/).

## `avdmanager` always reports "not found"

**Symptom:** `avdmanager` is genuinely installed and working, but a presence check using `avdmanager --version` always fails.

**Cause:** this version of `avdmanager` has no `--version` flag at all — it exits with status 1 and a usage error on it. A presence check based on exit status silently misreports it as missing.

**Fix:** use `avdmanager list` (a harmless, always-zero-exit no-op) as the presence probe instead.

## mitmweb's web UI returns 403

**Symptom:** the browser opens (or you open it manually) to `http://127.0.0.1:8081/`, and get a 403.

**Cause:** mitmweb requires a `?token=` query parameter (or an `Authorization: Bearer` header) unless you explicitly disable auth. The bare URL has no token.

**Fix:** generate a token, pass it via `--set web_password=<token>`, and open the browser at the fully-qualified URL (`?token=<token>`) yourself — see [Proxy & CA Certificate](/APK-Analysis-Toolkit/mitmproxy/proxy-and-cert/).

## `remount` reports success, but pushing to `/system` still fails with "Read-only file system"

**Symptom:** `adb root` and `adb remount` both report `[OK]`, but writing to `/system/etc/security/cacerts/` fails anyway.

**Cause:** on a **dm-verity-enabled image**, the first `remount` doesn't actually make `/system` writable — it disables verity and requires a **reboot** before a second `remount` will succeed. The real remount output (`Disabling verity for /system — Now reboot your device for settings to take effect`) makes this obvious once you look at it, but a wrapper that only reports a bare success/failure status hides it completely.

**Fix:** detect that exact message, reboot, wait for boot, then root + remount again — the second attempt reports `remount succeeded` for real. See [Proxy & CA Certificate](/APK-Analysis-Toolkit/mitmproxy/proxy-and-cert/).

## Chrome shows "No internet" after setting up the proxy

**Symptom:** the device's system proxy is set correctly and mitmweb is running, but Chrome shows "No internet," and some pages partially fail to load.

**Cause:** Android's connectivity check runs over **HTTPS**. If the device's proxy is set but mitmproxy's CA isn't yet trusted (see the dm-verity issue above), every HTTPS connection — including that connectivity check — fails with a certificate error. Plain HTTP content, which needs no certificate validation, can still partially load, which is why it looks like a partial rather than total outage.

**Fix:** this resolves itself once the CA-trust sequence actually completes (see above) — it's a symptom of an incomplete cert install, not a separate bug.

## `xz`/`openssl` reported missing despite being installed

**Symptom:** a tool is genuinely on the machine (confirmed via `Get-Command` in PowerShell) but the toolkit reports it missing.

**Cause:** tools bundled with Git for Windows (`xz`, `openssl`, `curl`) live under Git's own `mingw64/bin`/`usr/bin`, which isn't always on every shell's `PATH` even when it's on the "real" system `PATH` — this varies by which shell launched the process.

**Fix:** fall back to checking Git for Windows' known install locations directly, the same pattern already used for Android SDK tool discovery, rather than relying solely on `PATH`.

## Pasted install paths with quotes fail

**Symptom:** `Install APK` fails when the path was copied via Windows' "Copy as path," which wraps the result in literal double quotes.

**Fix:** strip matching leading/trailing quote characters before using the path.
