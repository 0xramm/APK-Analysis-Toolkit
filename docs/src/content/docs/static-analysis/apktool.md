---
title: APK Tool (apktool)
description: Decompiling and recompiling an APK, with the output kept in the repo's own workspace.
---

apktool decodes an APK's resources and `AndroidManifest.xml` back to (near-)source form, and can rebuild an edited copy — useful for inspecting resources, tampering with the manifest, or patching smali directly, which JADX's Java-focused view doesn't cover.

## Install

apktool ships as a single jar with no installer. **Install/verify** checks a local cache (`%LOCALAPPDATA%\apk-toolkit\cache`) for `apktool_*.jar`; if missing, it fetches the latest release from [iBotPeaches/Apktool](https://github.com/iBotPeaches/Apktool) on GitHub and downloads the jar — a one-time ~15MB download, with live progress. Requires a JRE (checked via `java -version`); if Java isn't installed, it says so rather than failing cryptically mid-download.

## Decompile APK (pick package)

Opens the same package picker used throughout the app (see [Listing & Filtering](/APK-Analysis-Toolkit/app-manager/listing-and-filtering/)). On selection:

1. Pulls the package's APK via `adb pull` into `workspace/apktool/<package>/<package>.apk` — **in the repo's own `workspace/` directory**, not a hidden cache, so it's easy to find and the folder is already `.gitignore`d.
2. Runs `apktool d <apk> -o workspace/apktool/<package>/decompiled -f`.
3. Opens the resulting `decompiled/` folder directly in File Explorer.

```
workspace/apktool/com.example.app/
├── com.example.app.apk
└── decompiled/
    ├── AndroidManifest.xml
    ├── res/
    ├── smali/
    └── ...
```

## Compile APK (from folder)

Takes a folder path (typically one of the `decompiled/` directories above, possibly edited) and runs `apktool b <folder>`, which writes the rebuilt APK to `<folder>/dist/`. Opens that output folder when done.

:::note
A compiled APK from apktool is unsigned — install it with `adb install -r` as-is only works on a device that doesn't enforce signature verification strictly in your workflow, or re-sign it (`apksigner`) before installing on a stricter device.
:::
