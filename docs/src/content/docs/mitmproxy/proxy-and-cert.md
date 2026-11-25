---
title: Proxy & CA Certificate
description: Starting mitmweb, filtering noise, and getting its CA genuinely trusted on-device.
---

## Install mitmproxy

Installs via `pip` if `mitmweb` isn't already present, streaming progress live.

## Start proxy & point device at it

Starts `mitmweb` (not headless `mitmdump` — you get the live web UI) with two things wired in deliberately:

**A working auth token, and an actually-correct auto-open.** mitmweb normally generates its own random token and opens a browser tab to the right URL itself — but that only works if you let it manage the whole flow. This tool needs to *know* the token (to log the URL, and to be sure your browser is pointed at the right place), so it pins one itself via `--set web_password=<token>`, suppresses mitmweb's own browser launch (`--no-web-open-browser`, since with a fixed password its own URL omits the token and would 403), and opens the browser itself pointed at `http://127.0.0.1:8081/?token=<token>`.

**Noise filtering.** `--ignore-hosts` is passed for `connectivitycheck.gstatic.com`, `connectivitycheck.android.com`, and `play.googleapis.com` — a full TCP passthrough, so these never show up as flows at all (not just hidden by a UI filter).

Also sets the device's global HTTP proxy to `10.0.2.2:<port>` (the host, from the emulator's point of view).

## Install CA cert as system root

This is the step most guides gloss over, and the one most likely to silently fail. The full sequence:

1. Hashes mitmproxy's CA cert the way Android's system trust store expects (`openssl x509 ... -subject_hash_old`).
2. `adb root`, then **waits for the device to actually reconnect** before proceeding — `adb root` restarts `adbd`, causing a disconnect/reconnect window; issuing another command too soon can race it.
3. `adb remount`. On a **dm-verity-enabled image**, this doesn't make `/system` writable yet — it just disables verity and prints `Now reboot your device for settings to take effect`. The tool detects this exact message and:
   - Reboots the device and waits for it to finish booting.
   - Roots and remounts again — this time it actually succeeds (`remount succeeded`).
4. Pushes the cert to `/system/etc/security/cacerts/<hash>.0`, sets permissions, and reboots once more to load it.

The whole sequence pins one device serial for its duration (captured before the first `adb root`), so a mid-sequence `adb devices` hiccup during the reconnect can't make a later step land on a different device than the one that was just rooted.

:::note[Why Chrome might show "No internet"]
If the proxy is set but the CA isn't yet trusted, HTTPS connectivity checks (which Android runs over HTTPS) fail outright — Chrome reports this as "no internet," while plain HTTP content can still partially load. This is a strong signal the cert-install step hasn't completed successfully yet. See [Common Issues](/APK-Analysis-Toolkit/troubleshooting/common-issues/).
:::
