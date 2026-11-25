---
title: Listing & Filtering
description: Listing installed packages and toggling the system-package noise filter.
---

## List installed packages

Runs `pm list packages` and prints the result sorted. By default, `com.android.*` and `com.google.*` packages are filtered out — on a stock emulator image these can be 150+ entries of pure platform noise, drowning out the handful of user-installed apps you actually care about.

```
[OK] 4 packages (system packages hidden — toggle via 'Toggle system-package filter')
    android
    android.auto_generated_rro_product__
    android.auto_generated_rro_vendor__
    com.breel.wallpapers18
```

## Toggle system-package filter

Flips the filter on/off instantly (no device round-trip beyond the next list). With it off, the same device shows the full platform picture:

```
[OK] 206 packages (showing all, including system packages)
    ...
    com.android.vending
    com.google.android.gms
    com.google.android.gmscore
    ...
```

This matters beyond cosmetics: a malicious package can deliberately claim a `com.google.*`/`com.android.*` name specifically to hide in this kind of noise (or get filtered out by exactly this feature). See [Package Info & Red Flags](/APK-Analysis-Toolkit/app-manager/package-info-and-red-flags/) for how the toolkit catches that anyway.

## Shared picker

This filtered/unfiltered package list is the single source every other App Manager action pulls from — Launch, Package Info, Install/Uninstall/Enable/Disable, and the SSL-pinning bypass picker all show the same list, so the filter toggle applies everywhere at once.
