---
title: Listing & Searching Packages
description: Listing every installed package and narrowing the list with type-to-filter search.
---

## List installed packages

Runs `pm list packages` and prints the full, sorted result — every package, with no name-based hiding. On a stock emulator image that's 150+ entries including the `com.android.*`/`com.google.*` platform packages:

```
[OK] 206 packages
    android
    android.auto_generated_rro_product__
    android.auto_generated_rro_vendor__
    com.android.vending
    com.breel.wallpapers18
    com.google.android.gms
    com.google.android.gmscore
    ...
```

Nothing is filtered out by name. A malicious package can deliberately claim a `com.google.*`/`com.android.*`-style name specifically to hide in platform noise — filtering by name would reward exactly that trick, so the toolkit never does it. See [Package Info & Red Flags](/APK-Analysis-Toolkit/app-manager/package-info-and-red-flags/) for how it catches disguised packages instead.

## Type-to-filter search

Every screen that shows a package list — Launch, Package Info, Install/Uninstall/Enable/Disable, and the SSL-pinning bypass picker — is the same picker widget with a live search box built in. Start typing and the list narrows to packages whose name contains what you've typed (case-insensitive substring match); `Backspace` removes characters, arrow keys move within the narrowed results, and `Enter` selects.

This replaces the need for any list-level filtering: instead of hiding packages ahead of time, you search down to the one you want on the fly, without ever losing visibility into the full list.
