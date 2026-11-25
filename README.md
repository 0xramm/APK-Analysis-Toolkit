# APK Analysis Toolkit

A Rust-based terminal UI (TUI) that bootstraps a full Android pentest / malware-analysis sandbox on your machine. Instead of hunting down ten different setup guides, you get one menu: run everything end-to-end, or pick a single step (emulator, SSL pinning bypass, mitmproxy) and let the tool detect what's installed, install what's missing, and configure it for you.

> Status: starter kit / work in progress. This README doubles as the design spec and a manual walkthrough of every step the TUI will automate — so you can follow it by hand today, and know exactly what the tool does once it's built.

## What it does

Run the whole flow as one template, or jump into any single module from the TUI's menu:

1. **Emulator check/setup** — detects an existing Android Studio AVD, shows its details (API level, arch, RAM), or walks you through creating one if none exists.
2. **SSL pinning bypass** — detects Frida/objection, installs them if missing, and attaches to a target app to bypass certificate pinning.
3. **mitmproxy + CA cert install** — detects mitmproxy, installs it if missing, starts it, and pushes/installs its CA certificate into the emulator as a trusted root.
4. **(Planned) more tools** — jadx, apktool, MobSF, drozer, Burp Suite config — see [Roadmap](#roadmap).

The goal: go from a fresh machine to "APK installed, traffic decrypted, pinning bypassed" in one guided session.

## Architecture (planned)

```
apk-toolkit/
├── src/
│   ├── main.rs          # TUI entry point (ratatui + crossterm)
│   ├── ui/              # menu screens, logs pane, progress widgets
│   ├── modules/
│   │   ├── emulator.rs  # AVD detection / creation
│   │   ├── ssl_pin.rs   # Frida / objection install + bypass
│   │   ├── mitm.rs      # mitmproxy install + cert push
│   │   └── template.rs  # runs all modules in sequence
│   └── util/
│       ├── shell.rs     # wraps adb / emulator / pip / brew / winget calls
│       └── detect.rs    # "is X installed / running?" checks
└── Cargo.toml
```

Each module follows the same pattern: **detect → report → offer to install → configure → verify**. The "template" option just runs all modules back-to-back with the same prompts.

## Prerequisites

- A terminal (Windows Terminal / PowerShell, or any Unix shell)
- Internet access (to download Android Studio, Python packages, Frida gadget/server, etc.)
- `adb` on your `PATH` once Android Studio / platform-tools is installed
- Admin rights to install software

---

## Tutorial: setting up the sandbox step by step

This section is the manual version of what the TUI automates. Follow it in order the first time; after that, the TUI lets you re-run any single step.

### Step 1 — Android emulator (Android Studio AVD)

**Check if Android Studio / an emulator is already set up:**

```bash
# Is Android Studio's emulator binary on PATH?
emulator -version

# List existing Virtual Devices (AVDs)
emulator -list-avds
avdmanager list avd
```

If an AVD exists, the tool prints its details (API level, target, CPU/ABI, RAM, path to its `config.ini`) so you know what you're working with:

```bash
cat "$ANDROID_SDK_ROOT/.android/avd/<avd-name>.avd/config.ini"
```

**If nothing is installed:**

1. Download Android Studio: https://developer.android.com/studio
2. Install it, then open **Device Manager** → **Create Device**
3. Pick a profile (e.g. Pixel 6) → pick a system image **with Google APIs but without Play Store** if you plan to root it (needed for SSL pinning work) — a `*_x86_64` image without Play Store can be rooted via Magisk-less `adb root`/`adb remount`, since Google Play images are harder to get root on.
4. Finish, then boot it once from Device Manager to confirm it works.

Command-line equivalent (what the TUI will shell out to):

```bash
sdkmanager "system-images;android-33;google_apis;x86_64"
avdmanager create avd -n pentest-avd -k "system-images;android-33;google_apis;x86_64" -d pixel_6
emulator -avd pentest-avd -writable-system
```

`-writable-system` is important — you need a writable `/system` partition to install the mitmproxy CA cert as a trusted *system* root later (Android 7+ ignores user-added CAs for apps targeting API 24+ unless the app opts in).

### Step 2 — SSL pinning bypass (Frida + objection)

**Check if Frida/objection are already installed:**

```bash
frida --version
objection version
```

**If missing, install them:**

```bash
pip install frida-tools objection
```

**Push the Frida server to the emulator** (must match your host Frida version and the emulator's ABI):

```bash
# download frida-server-<version>-android-x86_64.xz from
# https://github.com/frida/frida/releases, matching `frida --version`

adb push frida-server /data/local/tmp/
adb shell "chmod 755 /data/local/tmp/frida-server"
adb shell "su -c /data/local/tmp/frida-server &"   # needs root (adb root on non-Play images)
```

**Bypass pinning on the target app** (objection ships a universal bypass script):

```bash
objection -g com.target.package explore
# inside objection shell:
android sslpinning disable
```

Or one-liner from the host:

```bash
frida -U -f com.target.package -l objection/android/ssl-pinning-bypass.js --no-pause
```

The TUI's SSL-pinning module wraps exactly these three things: *is frida-server running on the device*, *is the host Frida version compatible*, *attach and run the bypass script against a package name you type in*.

### Step 3 — mitmproxy setup + certificate install

**Check if mitmproxy is installed:**

```bash
mitmproxy --version
```

**If missing, install it:**

```bash
pip install mitmproxy
# or: brew install mitmproxy   (macOS)
# or: choco install mitmproxy  (Windows)
```

**Start it** (transparent/regular proxy mode, listening on all interfaces so the emulator can reach it):

```bash
mitmproxy --listen-host 0.0.0.0 --listen-port 8080
```

**Point the emulator at it:**

```bash
adb shell settings put global http_proxy <host-ip>:8080
```

(From the emulator's point of view, `10.0.2.2` is the host machine, so `10.0.2.2:8080` usually works without needing your real LAN IP.)

**Grab the mitmproxy CA certificate and install it on the emulator.** With the proxy running and the device pointed at it, visiting any HTTP site auto-generates the cert at `~/.mitmproxy/mitmproxy-ca-cert.cer`. Push it and install it as a **system** trust anchor (required for apps on API 24+ that ignore user CAs):

```bash
# convert to the hash-named form Android's system store expects
openssl x509 -inform PEM -subject_hash_old -in ~/.mitmproxy/mitmproxy-ca-cert.cer | head -1
# -> e.g. c8750f0d

cp ~/.mitmproxy/mitmproxy-ca-cert.cer c8750f0d.0

adb root
adb remount
adb push c8750f0d.0 /system/etc/security/cacerts/
adb shell "chmod 644 /system/etc/security/cacerts/c8750f0d.0"
adb reboot
```

Once it's rebooted, traffic from most apps (that don't do extra pinning) will show up decrypted in the mitmproxy UI. Apps that *do* pin will still need Step 2's Frida/objection bypass on top of this.

---

## Quick start (once the TUI exists)

```bash
apk-toolkit
```

```
┌─ APK Analysis Toolkit ─────────────────────┐
│ > Run full setup (template)                │
│   Emulator setup                           │
│   SSL pinning bypass                       │
│   mitmproxy + CA cert install               │
│   -------------------------------------    │
│   Quit                                     │
└─────────────────────────────────────────────┘
```

Arrow keys / `j`/`k` to move, `Enter` to run a step, `q` to quit. Each step streams its shell output into a log pane and ends with a ✅/❌ summary plus "what to do next" if something's missing.

## Roadmap

- [ ] `jadx` — APK → Java decompiler, one-key integration
- [ ] `apktool` — decode/rebuild APK resources & manifest
- [ ] `MobSF` — automated static + dynamic analysis (Docker-based)
- [ ] `drozer` — Android attack surface exploration
- [ ] Burp Suite CA cert install (alternative to mitmproxy)
- [ ] Root detection bypass module (Magisk Hide / Shamiko presets)
- [ ] Snapshot/restore emulator state between samples (safety for malware runs)
- [ ] Config file to save your preferred AVD / proxy port / package name defaults

## Safety note

This kit is meant for **authorized pentesting, malware research, and CTFs** on devices/emulators you control. Always analyze untrusted APKs inside the emulator (ideally snapshotted and network-isolated from anything sensitive), never on a personal device.

## License

MIT (or whatever you want to put here — placeholder until decided).
