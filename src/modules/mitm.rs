use std::sync::mpsc::Sender;

use super::emulator;
use crate::util::{sdk, shell};

pub const PROXY_PORT: u16 = 8080;
pub const WEB_PORT: u16 = 8081;

/// Android connectivity-check / Play-services telemetry hosts that aren't
/// useful noise in a traffic-analysis session — fully bypassed via
/// --ignore-hosts (plain TCP passthrough, not just hidden from the UI).
const IGNORED_HOSTS: &[&str] = &[
    r"connectivitycheck\.gstatic\.com",
    r"connectivitycheck\.android\.com",
    r"play\.googleapis\.com",
];

/// Returns true if mitmweb ends up installed.
pub fn install(tx: &Sender<String>) -> bool {
    tx.send("== mitmproxy install ==".into()).ok();
    if shell::is_installed("mitmweb", "--version") {
        let (_, v) = shell::run("mitmweb", &["--version"]);
        tx.send(format!("[OK] {}", v.lines().next().unwrap_or(&v))).ok();
        true
    } else {
        tx.send("[MISSING] mitmproxy not found.".into()).ok();
        tx.send("> pip install mitmproxy...".into()).ok();
        let (ok, out) = shell::run_streaming("pip", &["install", "mitmproxy"], tx);
        tx.send(if ok {
            "[OK] mitmproxy installed".into()
        } else {
            format!("[FAIL] {out}")
        })
        .ok();
        ok
    }
}

/// Starts mitmweb (proxy + browser UI) in the background and points the connected
/// emulator's global HTTP proxy at the host (10.0.2.2 is the host, from the emulator's view).
/// Returns true if mitmweb started; the device-proxy step is a soft warning, not a blocker.
pub fn start_and_set_proxy(tx: &Sender<String>) -> bool {
    tx.send("== Starting mitmweb & pointing device at it ==".into()).ok();

    // mitmweb normally opens the browser itself with its auto-generated token
    // baked into the URL, but when we pin web_password ourselves (so we can
    // know and print the URL), its own web_url omits the token — so we set a
    // known token, suppress mitmweb's launch, and open the browser ourselves.
    let token = shell::random_token();
    let mut args = vec![
        "--listen-host".to_string(),
        "0.0.0.0".to_string(),
        "--listen-port".to_string(),
        PROXY_PORT.to_string(),
        "--web-port".to_string(),
        WEB_PORT.to_string(),
        "--no-web-open-browser".to_string(),
        "--set".to_string(),
        format!("web_password={token}"),
    ];
    for host in IGNORED_HOSTS {
        args.push("--ignore-hosts".to_string());
        args.push(host.to_string());
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    tx.send("> starting mitmweb...".into()).ok();
    match shell::spawn_bg("mitmweb", &args) {
        Ok(()) => {
            tx.send(format!("[OK] mitmweb listening on 0.0.0.0:{PROXY_PORT}")).ok();
            tx.send("  (connectivity-check/Play-services noise bypassed, won't show as flows)".into()).ok();
        }
        Err(e) => {
            tx.send(format!("[FAIL] {e}")).ok();
            return false;
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(800));
    let ui_url = format!("http://127.0.0.1:{WEB_PORT}/?token={token}");
    tx.send(format!("> opening {ui_url} ...")).ok();
    match shell::open_url(&ui_url) {
        Ok(()) => {
            tx.send("[OK] browser opened".into()).ok();
        }
        Err(e) => {
            tx.send(format!("[!] couldn't auto-open a browser ({e}) — open it manually:")).ok();
            tx.send(format!("    {ui_url}")).ok();
        }
    }

    if !shell::adb_device_connected() {
        tx.send("[!] No device connected — once it's up, set its proxy manually:".into()).ok();
        tx.send(format!("    adb shell settings put global http_proxy 10.0.2.2:{PROXY_PORT}")).ok();
        return true;
    }

    tx.send("> setting device proxy...".into()).ok();
    let (ok, out) =
        shell::adb_run(&["shell", &format!("settings put global http_proxy 10.0.2.2:{PROXY_PORT}")]);
    tx.send(if ok {
        format!("[OK] device proxy set to 10.0.2.2:{PROXY_PORT}")
    } else {
        format!("[FAIL] {out}")
    })
    .ok();
    true
}

/// Looks for mitmproxy's generated CA cert, hashes it the way Android's system
/// trust store expects, pushes it as a system CA, and reboots the device.
/// Returns true only if the cert ends up installed and the reboot was issued.
pub fn install_cert(tx: &Sender<String>) -> bool {
    tx.send("== Install mitmproxy CA cert on device ==".into()).ok();

    // adb root restarts adbd (disconnect+reconnect) partway through this
    // sequence. Pin one serial for the whole thing so a mid-sequence device
    // list hiccup can't make remount/push silently land on a different
    // device than the one that was just rooted.
    let previous_override = shell::selected_device();
    let pinned = shell::adb_target();
    if let Some(serial) = &pinned {
        shell::set_selected_device(Some(serial.clone()));
    }
    let result = install_cert_pinned(tx, pinned.as_deref());
    shell::set_selected_device(previous_override);
    result
}

fn install_cert_pinned(tx: &Sender<String>, serial: Option<&str>) -> bool {
    let cert_path = cert_source_path();
    if !cert_path.exists() {
        tx.send(format!("[!] {} not found yet.", cert_path.display())).ok();
        tx.send("    Start the proxy, point the device at it, then visit any http://".into()).ok();
        tx.send("    site on the device so mitmproxy generates its CA cert, then retry.".into()).ok();
        return false;
    }

    let Some(openssl) = shell::locate("openssl", "version", sdk::git_for_windows_tool("openssl")) else {
        tx.send("[MISSING] openssl not found, can't compute the Android cert filename.".into()).ok();
        return false;
    };

    tx.send("> hashing cert...".into()).ok();
    let cert_str = cert_path.to_string_lossy().to_string();
    let (ok, hash_out) = shell::run(
        &openssl,
        &["x509", "-inform", "PEM", "-subject_hash_old", "-in", &cert_str],
    );
    let hash = hash_out.lines().next().unwrap_or("").trim().to_string();
    if !ok || hash.is_empty() {
        tx.send(format!("[FAIL] could not hash cert: {hash_out}")).ok();
        return false;
    }
    tx.send(format!("[OK] cert hash: {hash}")).ok();

    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return false;
    }

    let tmp_named = std::env::temp_dir().join(format!("{hash}.0"));
    if std::fs::copy(&cert_path, &tmp_named).is_err() {
        tx.send("[FAIL] could not stage renamed cert file.".into()).ok();
        return false;
    }
    let tmp_str = tmp_named.to_string_lossy().to_string();
    let dest = format!("/system/etc/security/cacerts/{hash}.0");

    let Some(remount_out) = root_and_remount(tx, serial) else {
        return false;
    };

    // On dm-verity-enabled images, `remount` can't make /system writable in
    // place — it just disables verity and demands a reboot before a second
    // remount will actually succeed. Do that reboot-and-retry cycle here
    // instead of pushing straight into what's still a read-only filesystem.
    if remount_out.contains("Now reboot") || remount_out.contains("Disabling verity") {
        tx.send("> verity needs a reboot before /system is actually writable — rebooting...".into())
            .ok();
        if let Err(e) = shell::adb_spawn_bg(&["reboot"]) {
            tx.send(format!("[FAIL] {e}")).ok();
            return false;
        }
        std::thread::sleep(std::time::Duration::from_secs(3)); // let it actually go down first
        if !emulator::wait_for_boot(tx, 120) {
            return false;
        }
        let Some(_) = root_and_remount(tx, serial) else {
            return false;
        };
    }

    tx.send("> pushing cert...".into()).ok();
    let (ok, out) = shell::adb_run(&["push", &tmp_str, &dest]);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return false;
    }
    tx.send(format!("[OK] pushed cert to {dest}")).ok();

    let (ok, out) = shell::adb_run(&["shell", &format!("chmod 644 {dest}")]);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return false;
    }
    tx.send("[OK] permissions set".into()).ok();

    tx.send("> rebooting device to load the new system CA...".into()).ok();
    match shell::adb_spawn_bg(&["reboot"]) {
        Ok(()) => {
            tx.send("[OK] reboot issued — traffic should decrypt once it's back up.".into()).ok();
            true
        }
        Err(e) => {
            tx.send(format!("[FAIL] {e}")).ok();
            false
        }
    }
}

/// `adb root` (waiting for adbd's restart to settle) then `adb remount`.
/// Returns remount's raw output on success (the caller checks it for the
/// dm-verity "reboot needed" case), or None if either step failed.
fn root_and_remount(tx: &Sender<String>, serial: Option<&str>) -> Option<String> {
    tx.send("> adb root...".into()).ok();
    let (ok, out) = shell::adb_run(&["root"]);
    let rooted = ok && !out.contains("cannot run as root");
    tx.send(if rooted { "[OK] adb root".into() } else { format!("[FAIL] adb root: {out}") }).ok();
    if !rooted {
        tx.send("This device's image doesn't support root (likely a Play Store system image).".into())
            .ok();
        tx.send("Use 'Create new AVD' for a non-Play-Store, rootable image instead.".into()).ok();
        return None;
    }

    // adb root just restarted adbd — give it a moment to actually reconnect
    // in root mode before issuing anything else, or remount can silently
    // race a device that's momentarily offline/reconnecting.
    if let Some(serial) = serial {
        tx.send("> waiting for adbd to reconnect as root...".into()).ok();
        if !shell::wait_for_device(serial, 15) {
            tx.send("[FAIL] device didn't come back online after adb root.".into()).ok();
            return None;
        }
    }

    tx.send("> adb remount...".into()).ok();
    let (ok, out) = shell::adb_run(&["remount"]);
    tx.send(format!("{} adb remount: {out}", if ok { "[OK]" } else { "[FAIL]" })).ok();
    ok.then_some(out)
}

fn cert_source_path() -> std::path::PathBuf {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".mitmproxy").join("mitmproxy-ca-cert.cer")
}
