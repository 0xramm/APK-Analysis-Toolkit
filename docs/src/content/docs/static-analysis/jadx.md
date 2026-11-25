---
title: JADX
description: Decompiling Java/Kotlin sources from an APK and opening them directly in jadx-gui.
---

JADX decompiles DEX bytecode back to readable Java, which is usually a faster way to understand an app's actual logic than reading smali from apktool's output.

## Install

Checked/installed automatically the first time you use it: downloads the latest [skylot/jadx](https://github.com/skylot/jadx) release and extracts it into `%LOCALAPPDATA%\apk-toolkit\cache\jadx`. Requires a JRE (checked via `java -version`).

JADX's releases ship several similarly-named assets — the full cross-platform bundle (`jadx-<version>.zip`, containing both the `jadx` CLI and `jadx-gui`) alongside GUI-only platform builds (`jadx-gui-<version>-win.zip`, and macOS `.dmg`s). The installer matches the **exact** full-bundle filename once it knows the release's version number, rather than a loose substring match — a naive "contains jadx-" check would just as happily match the Windows GUI-only zip.

Extraction uses Windows' built-in `tar` (the real `bsdtar` shipped in `System32`, not whatever `tar` a shell's own `PATH` might resolve to first — see [Troubleshooting](/APK-Analysis-Toolkit/troubleshooting/common-issues/) for why that distinction matters).

## Open package in JADX

Opens the package picker, pulls the selected package's APK into `workspace/jadx/<package>/<package>.apk` (same repo-local workspace convention as apktool), and launches `jadx-gui.bat` directly with that file — no separate "open file" step needed in the GUI.
