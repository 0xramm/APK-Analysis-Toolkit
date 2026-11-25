---
title: Bypass & Re-pin
description: Installing Frida tooling, pushing frida-server, bypassing pinning, and reversing it.
---

## Install frida-tools & objection

Installs both via `pip` if either is missing, streaming install progress live. Idempotent — if already present, just reports the installed version.

## Push & start frida-server

1. Reads the host's `frida --version` and the connected device's ABI (`adb shell getprop ro.product.cpu.abi`), mapping Android ABI names to Frida's naming (`arm64-v8a` → `arm64`, etc).
2. Downloads the matching `frida-server-<version>-android-<abi>.xz` build from GitHub — **cached** at `%LOCALAPPDATA%\apk-toolkit\cache`, so re-running this doesn't redownload on every pass.
3. Decompresses it (`xz`/`unxz`, auto-discovered even if not on the invoking shell's `PATH` — see [Troubleshooting](/APK-Analysis-Toolkit/troubleshooting/common-issues/)).
4. Pushes it to `/data/local/tmp/frida-server`, `chmod`s it executable, and starts it in the background.

`frida-server` can start unprivileged (as the `shell` user), but it can only **spawn or attach to other processes with root** — this is enforced by Android, not by this tool.

## Bypass pinning for a package

Opens the package picker (see [Listing & Filtering](/APK-Analysis-Toolkit/app-manager/listing-and-filtering/)) — each entry is annotated `[bypass active]` if this tool already has a tracked session running against it. On selection, it:

```
frida -D <serial> -f <package> -c akabe1/frida-multiple-unpinning
```

- Targets the device by **explicit serial** (`-D`, resolved the same way every other `adb` call in this tool is), not Frida's bare `-U` — with more than one device attached, `-U` picks arbitrarily and can silently hang spawning against a device that has no `frida-server` running.
- Fetches the well-known universal SSL-unpinning script live from Frida's CodeShare (`akabe1/frida-multiple-unpinning`) — no local script file to manage.
- Requires root on the device; see [AVDs & Rooting](/APK-Analysis-Toolkit/emulator/avds-and-rooting/).

## Stop bypass / re-pin a package

Lists only packages with an active tracked session. On selection:

1. Kills the tracked `frida` process by PID.
2. Force-stops the app (`adb shell am force-stop <pkg>`) so its *next* launch has no injected agent — this is what actually restores normal pinned behavior, since killing the Frida client alone doesn't un-hook an already-running process.

If no session is tracked for a package (e.g. Frida already exited on its own), it says so rather than silently no-op'ing.
