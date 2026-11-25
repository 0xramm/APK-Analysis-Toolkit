---
title: Multi-Device Selection
description: How the toolkit handles more than one connected ADB device.
---

## The problem

Every `adb` command this tool runs needs to know which device to target. With exactly one device attached, there's no ambiguity. With two or more — a real device plus an emulator, or a stale/"ghost" device entry left behind by a previous session — a bare `adb shell ...` fails outright:

```
adb.exe: more than one device/emulator
```

This happened repeatedly during development: closing and relaunching an AVD can leave an old entry behind in `adb devices` that never really went away, and running `adb root` restarts `adbd`, which causes a brief disconnect/reconnect that can confuse naive device selection mid-sequence.

## How it's handled

Every `adb` call in the toolkit goes through a shared targeting function that:

1. Uses a **manually pinned device** if you've set one (see below).
2. Otherwise prefers the first device actually in `"device"` state (online), falling back to whatever's listed if none are, so an explicit `-s <serial>` always avoids the ambiguity error — even against a ghost/offline entry.

For sequences that restart `adbd` mid-flight (rooting, then remounting, then pushing a cert), the device serial is **resolved once and pinned for the whole sequence**, so a device-list hiccup during the `adb root` reconnect window can't make a later step silently land on a different device than the one that was just rooted.

## Select ADB device

From the main menu, lists every entry `adb devices` reports (serial + state), plus an **Auto** option to clear any manual pin and go back to the automatic selection above. Your choice applies globally to every screen until changed.
