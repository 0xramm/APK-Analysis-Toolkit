---
title: Full Setup Pipeline
description: What "Run full setup (template)" actually does, step by step.
---

**Run full setup (template)** chains every other screen's core action into one pass, stopping at the first failure instead of continuing past a broken step.

```
== Full setup: emulator, SSL tooling, mitmproxy sandbox ==
```

1. **Detect.** Lists SDK state, AVDs, and connected devices.
2. **Ensure a rootable device is online.**
   - If a device is already connected, skips straight to step 3.
   - Otherwise, checks whether *any* existing AVD is rootable (not just whether one exists) — if the only AVDs around are Play Store images, it creates a fresh rootable one rather than booting something it already knows won't support the later steps.
   - Launches the AVD and polls `sys.boot_completed`, with periodic heartbeats (`still booting... (45s)`) so a multi-minute boot doesn't look frozen.
3. **SSL tooling.** Installs `frida-tools`/`objection`, then downloads (or reuses the cached) matching `frida-server` and pushes/starts it.
4. **mitmproxy.** Installs if missing, starts `mitmweb` with noise filtering and a valid auth token, sets the device's proxy, then runs the full CA-trust sequence (including the dm-verity reboot cycle if the image needs it).

## Why it stops on failure

Early versions of this pipeline logged a step's failure but kept going into the next, unrelated stage — meaning a failed `frida-server` push would be followed by mitmproxy setup starting anyway, burying the real problem in noise. Every step in the chain now returns a plain success/failure signal, and the pipeline checks it:

```
Stopped: couldn't push/start frida-server. Fix the error above, then re-run.
```

Fix whatever it reports and re-run — already-completed steps (an already-installed tool, an already-booted device) are detected and skipped, so re-running is cheap.
