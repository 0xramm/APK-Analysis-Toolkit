---
title: Auto-Update & Releases
description: How the toolkit updates itself, and how to cut a release.
---

## How auto-update works

On every bare launch (not during `--check` or other diagnostic flags), the toolkit:

1. Queries `https://api.github.com/repos/0xramm/APK-Analysis-Toolkit/releases/latest`, with a 5-second timeout. Any failure (offline, API down, no release yet) is silent — a normal launch never blocks on network issues.
2. Compares the release tag against the running binary's version **numerically** (`0.10.0` is correctly newer than `0.9.0` — a plain string compare would get this wrong).
3. If newer, downloads the release's `apkat.exe` asset, then performs the update:
   - Renames the **currently running** exe aside to `apkat.old.exe` — Windows allows renaming a file that's in use, just not deleting or overwriting it.
   - Moves the newly downloaded exe into the original path.
   - Relaunches from that path and exits the old process.
4. On the **next** launch, the leftover `apkat.old.exe` (no longer locked by anything) is deleted — the one file Windows wouldn't let the update process remove mid-flight.

No new dependency was added for this — GitHub's release JSON is parsed with targeted substring scanning rather than a full JSON parser, since the shape being read (`tag_name`, and the `browser_download_url` of the one asset named `apkat.exe`) is narrow and stable. Covered by unit tests against a realistic multi-asset release payload.

## Cutting a release

The GitHub Actions workflow at `.github/workflows/release.yml` builds on `windows-latest` and attaches the binary whenever a tag matching `v*` is pushed:

```bash
# bump `version` in Cargo.toml first, then:
git add Cargo.toml
git commit -m "Bump version to 0.2.0"
git tag v0.2.0
git push origin main v0.2.0
```

The workflow produces `apkat.exe` and uploads it as a release asset with that exact name — matching what the auto-updater looks for — alongside `apkat-setup.exe`, the Inno Setup installer meant for first-time installs (see [Installation](/APK-Analysis-Toolkit/getting-started/installation/)).
