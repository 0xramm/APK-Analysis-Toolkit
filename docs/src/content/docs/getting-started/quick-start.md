---
title: Quick Start
description: Get a working rooted sandbox in one pass.
---

## Fastest path: Run full setup

From the main menu, select **Run full setup (template)**. This single action:

1. Detects your Android SDK and lists existing AVDs, tagging each as rootable or not.
2. If no AVD exists — or none of the existing ones support root — creates a Google APIs (no Play Store) AVD, which is rootable and has broad package compatibility.
3. Launches it and waits for boot, with periodic progress (`still booting... (Ns)`).
4. Installs `frida-tools` and `objection` if missing.
5. Downloads, caches, and pushes a matching `frida-server` to the device.
6. Installs `mitmproxy` if missing, starts `mitmweb` (auto-opens your browser with a valid auth token), and points the device's system proxy at it.
7. Installs mitmproxy's CA certificate as a trusted system root — including the reboot cycle required on dm-verity–enabled images (see [Troubleshooting](/APK-Analysis-Toolkit/troubleshooting/common-issues/)).

If any step fails, the pipeline **stops immediately** with a clear reason instead of continuing past a broken step — fix what it reports, then re-run.

## Navigating the TUI

| Key | Action |
|---|---|
| `↑`/`↓` or `j`/`k` | Move selection |
| `Enter` | Select / run |
| `Esc` or `Backspace` | Back to the previous menu |
| `PageUp` / `PageDown` | Scroll a long log/report |
| `c` | Copy the current log/report to the clipboard |
| `q` | Quit (from the main menu) |

Long-running steps show a spinner and live progress in the log pane — `Enter` is disabled until the current job finishes, but you can still browse with the arrow keys.

## Next steps

Once the pipeline completes, you have a rooted emulator with traffic routed through mitmproxy. From here:

- Install a target APK via **App manager → Install APK**.
- Run **App manager → Package info** on it first — the automated red-flags check (see [Package Info & Red Flags](/APK-Analysis-Toolkit/app-manager/package-info-and-red-flags/)) is worth running on anything you didn't build yourself.
- Use **SSL pinning bypass → Bypass pinning for a package** if the app pins its own certificates.
