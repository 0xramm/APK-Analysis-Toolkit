use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

use crate::util::{cache, sdk, shell};

/// Packages this tool has started a frida bypass session for, and the PID of
/// that session. Best-effort: doesn't notice if frida exits on its own.
fn sessions() -> &'static Mutex<HashMap<String, u32>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Packages currently believed to have an active bypass session.
pub fn active_bypasses() -> Vec<String> {
    sessions().lock().unwrap().keys().cloned().collect()
}

pub fn is_bypassed(pkg: &str) -> bool {
    sessions().lock().unwrap().contains_key(pkg)
}

/// Returns true if both frida and objection end up installed.
pub fn install_tools(tx: &Sender<String>) -> bool {
    tx.send("== Install frida-tools & objection ==".into()).ok();

    let frida_ok = if shell::is_installed("frida", "--version") {
        let (_, v) = shell::run("frida", &["--version"]);
        tx.send(format!("[OK] frida {v}")).ok();
        true
    } else {
        tx.send("[MISSING] frida not found.".into()).ok();
        tx.send("> pip install frida-tools...".into()).ok();
        let (ok, out) = shell::run_streaming("pip", &["install", "frida-tools"], tx);
        tx.send(if ok {
            "[OK] frida-tools installed".into()
        } else {
            format!("[FAIL] {out}")
        })
        .ok();
        ok
    };

    let objection_ok = if shell::is_installed("objection", "version") {
        tx.send("[OK] objection installed".into()).ok();
        true
    } else {
        tx.send("[MISSING] objection not found.".into()).ok();
        tx.send("> pip install objection...".into()).ok();
        let (ok, out) = shell::run_streaming("pip", &["install", "objection"], tx);
        tx.send(if ok {
            "[OK] objection installed".into()
        } else {
            format!("[FAIL] {out}")
        })
        .ok();
        ok
    };

    frida_ok && objection_ok
}

/// Downloads the frida-server build matching the host's frida version and the
/// connected device's ABI, pushes it, and starts it on the device (needs root).
/// Returns true only if every step up to "starting on device" succeeded.
pub fn push_frida_server(tx: &Sender<String>) -> bool {
    tx.send("== Push & start frida-server ==".into()).ok();

    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device/emulator connected (adb devices).".into()).ok();
        return false;
    }
    if !shell::is_installed("frida", "--version") {
        tx.send("[FAIL] `frida` not installed — run 'Install frida-tools & objection' first.".into())
            .ok();
        return false;
    }
    if !shell::is_installed("curl", "--version") {
        tx.send("[MISSING] `curl` not on PATH, can't auto-download frida-server.".into()).ok();
        tx.send("  -> Get it manually: https://github.com/frida/frida/releases".into()).ok();
        return false;
    }

    let (_, version) = shell::run("frida", &["--version"]);
    let version = version.trim().to_string();
    tx.send("> reading device ABI...".into()).ok();
    let (_, abi_raw) = shell::adb_run(&["shell", "getprop", "ro.product.cpu.abi"]);
    let abi_raw = abi_raw.trim();
    let abi = map_abi(abi_raw);

    tx.send(format!("Device ABI: {abi_raw} -> frida ABI: {abi}")).ok();
    tx.send(format!("Host frida version: {version}")).ok();

    let bin_name = format!("frida-server-{version}-android-{abi}");
    let cached_bin = cache::dir().join(&bin_name);

    if cached_bin.exists() {
        tx.send(format!("[OK] using cached frida-server ({})", cached_bin.display())).ok();
    } else {
        let fname = format!("{bin_name}.xz");
        let url = format!("https://github.com/frida/frida/releases/download/{version}/{fname}");
        let archive = cache::dir().join(&fname);
        let archive_str = archive.to_string_lossy().to_string();

        tx.send(format!("> downloading {fname}...")).ok();
        let (ok, out) = shell::run_streaming("curl", &["-L", "-o", &archive_str, &url], tx);
        if !ok {
            tx.send(format!("[FAIL] download failed: {out}")).ok();
            return false;
        }
        tx.send("[OK] downloaded".into()).ok();

        tx.send("> decompressing...".into()).ok();
        let decompressor = shell::locate("xz", "--version", sdk::git_for_windows_tool("xz"))
            .or_else(|| shell::locate("unxz", "--version", sdk::git_for_windows_tool("unxz")));
        let Some(decompressor) = decompressor else {
            tx.send("[MISSING] `xz`/`unxz` not found. Install Git for Windows (bundles xz) or 7-Zip.".into())
                .ok();
            return false;
        };
        let (ok, out) = shell::run(&decompressor, &["-d", "-f", &archive_str]);
        if !ok {
            tx.send(format!("[FAIL] decompress failed: {out}")).ok();
            return false;
        }
        tx.send(format!("[OK] cached at {}", cached_bin.display())).ok();
    }

    let bin_str = cached_bin.to_string_lossy().to_string();

    tx.send("> pushing to device...".into()).ok();
    let (ok, out) = shell::adb_run(&["push", &bin_str, "/data/local/tmp/frida-server"]);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return false;
    }
    tx.send("[OK] pushed to /data/local/tmp/frida-server".into()).ok();

    let (ok, out) = shell::adb_run(&["shell", "chmod 755 /data/local/tmp/frida-server"]);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return false;
    }
    tx.send("[OK] made executable".into()).ok();

    match shell::adb_spawn_bg(&["shell", "/data/local/tmp/frida-server &"]) {
        Ok(()) => {
            tx.send("[OK] frida-server starting on device (requires root).".into()).ok();
            true
        }
        Err(e) => {
            tx.send(format!("[FAIL] {e}")).ok();
            false
        }
    }
}

/// Spawns Frida against `package` with the universal SSL-pinning bypass script.
pub fn bypass(package: &str, tx: &Sender<String>) {
    let package = package.trim();
    tx.send(format!("== Bypassing pinning for '{package}' ==")).ok();

    if package.is_empty() {
        tx.send("[FAIL] No package name given.".into()).ok();
        return;
    }
    if !shell::is_installed("frida", "--version") {
        tx.send("[FAIL] frida not installed — run 'Install frida-tools & objection' first.".into())
            .ok();
        return;
    }

    // Target the specific device by serial: frida's bare -U picks arbitrarily
    // when more than one device/emulator is attached, which can silently hang
    // spawning on a device that has no frida-server running.
    let serial = shell::adb_target();
    let mut args: Vec<&str> = Vec::new();
    match &serial {
        Some(s) => args.extend(["-D", s]),
        None => args.push("-U"),
    }
    args.extend(["-f", package, "-c", "akabe1/frida-multiple-unpinning"]);

    tx.send("> spawning target via Frida codeshare (akabe1/frida-multiple-unpinning)...".into()).ok();
    match shell::spawn_child("frida", &args) {
        Ok(child) => {
            sessions().lock().unwrap().insert(package.to_string(), child.id());
            tx.send("[OK] launched in the background — check the device/app.".into()).ok();
            tx.send("  (First run fetches the codeshare script; needs internet once.)".into()).ok();
            tx.send("  Requires a rooted device/AVD — frida-server can't spawn without root.".into()).ok();
        }
        Err(e) => {
            tx.send(format!("[FAIL] {e}")).ok();
        }
    }
}

/// Kills the tracked frida session and force-stops the app so its next launch
/// is clean (no injected agent) — i.e. pinning is back in effect.
pub fn stop_bypass(package: &str, tx: &Sender<String>) {
    let package = package.trim();
    tx.send(format!("== Stopping bypass for '{package}' (re-pinning) ==")).ok();

    let pid = sessions().lock().unwrap().remove(package);
    match pid {
        Some(pid) => {
            let (ok, out) = shell::kill_pid(pid);
            tx.send(if ok {
                "[OK] frida session stopped".into()
            } else {
                format!("[FAIL] couldn't kill frida (pid {pid}): {out}")
            })
            .ok();
        }
        None => {
            tx.send("[!] No bypass session tracked for this package by this tool.".into()).ok();
        }
    }

    tx.send("> force-stopping the app so its next launch is clean...".into()).ok();
    let (ok, out) = shell::adb_run(&["shell", "am", "force-stop", package]);
    tx.send(if ok { "[OK] app stopped".into() } else { format!("[FAIL] {out}") }).ok();
}

fn map_abi(abi: &str) -> &'static str {
    match abi {
        "x86_64" => "x86_64",
        "x86" => "x86",
        "arm64-v8a" => "arm64",
        _ => "arm",
    }
}
