use std::sync::mpsc::Sender;

use crate::util::{sdk, shell};

pub const DEFAULT_AVD: &str = "pentest-avd";
/// Google APIs, no Play Store: the one combination that's both rootable
/// (adb root works) and has the broadest package/API compatibility.
const SYSTEM_IMAGE: &str = "system-images;android-33;google_apis;x86_64";

pub fn detect(tx: &Sender<String>) {
    tx.send("== Emulator detection ==".into()).ok();

    match shell::locate("emulator", "-version", sdk::emulator_path()) {
        Some(bin) => {
            let (_, version) = shell::run(&bin, &["-version"]);
            let short_version = version.split_whitespace().nth(3).unwrap_or("?");
            tx.send(format!("[OK] emulator installed (v{short_version})")).ok();

            tx.send("> listing AVDs...".into()).ok();
            let names = list_avd_names(&bin);
            if names.is_empty() {
                tx.send("[!] No AVDs found. Use 'Create new AVD' below.".into()).ok();
            } else {
                tx.send("[OK] AVDs found:".into()).ok();
                for name in &names {
                    let rootable = if avd_is_rootable(name) { "rootable" } else { "Play Store, no root" };
                    let line = match avd_summary(name) {
                        Some(summary) => format!("    - {name}  ({summary}, {rootable})"),
                        None => format!("    - {name}  ({rootable})"),
                    };
                    tx.send(line).ok();
                }
            }
        }
        None => {
            tx.send("[MISSING] Android emulator not found.".into()).ok();
            match sdk::sdk_root() {
                Some(root) => {
                    tx.send(format!(
                        "  SDK found at {} but no emulator binary — install the 'Android Emulator' package via Android Studio's SDK Manager.",
                        root.display()
                    ))
                    .ok();
                }
                None => {
                    tx.send("  -> Install Android Studio: https://developer.android.com/studio".into()).ok();
                }
            }
        }
    }

    tx.send("> checking connected devices...".into()).ok();
    let devices = shell::adb_devices();
    let online: Vec<_> = devices.iter().filter(|(_, s)| s == "device").collect();
    match online.len() {
        0 => {
            tx.send("[!] No device online yet (adb devices shows none, or it's still booting/offline).".into())
                .ok();
        }
        1 => {
            tx.send(format!("[OK] device online: {}", online[0].0)).ok();
        }
        _ => {
            tx.send(format!("[OK] {} devices online.", online.len())).ok();
            tx.send("[!] Multiple devices attached — use 'Select ADB device' (main menu) to pick which one actions target.".into()).ok();
        }
    }
}

/// Returns true if the AVD ends up created.
pub fn create_avd(tx: &Sender<String>) -> bool {
    tx.send(format!("== Creating AVD '{DEFAULT_AVD}' ==")).ok();

    let Some(sdkmanager) = shell::locate("sdkmanager", "--version", sdk::sdkmanager_path()) else {
        tx.send("[MISSING] sdkmanager not found — install Android Studio's cmdline-tools first.".into())
            .ok();
        return false;
    };
    // avdmanager has no --version flag (exits 1 with a usage error on it);
    // `list` is a harmless no-op that reliably exits 0 when it's working.
    let Some(avdmanager) = shell::locate("avdmanager", "list", sdk::avdmanager_path()) else {
        tx.send("[MISSING] avdmanager not found — install Android Studio's cmdline-tools first.".into())
            .ok();
        return false;
    };

    tx.send(format!("> installing system image {SYSTEM_IMAGE} (this can take a while)...")).ok();
    let (ok, out) = shell::run_streaming(&sdkmanager, &[SYSTEM_IMAGE], tx);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return false;
    }
    tx.send("[OK] system image ready".into()).ok();

    tx.send("> creating AVD...".into()).ok();
    let (ok, out) = shell::run(
        &avdmanager,
        &[
            "create", "avd", "-n", DEFAULT_AVD, "-k", SYSTEM_IMAGE, "-d", "pixel_6", "--force",
        ],
    );
    tx.send(if ok {
        format!("[OK] AVD '{DEFAULT_AVD}' created — Google APIs, no Play Store, fully rootable")
    } else {
        format!("[FAIL] {out}")
    })
    .ok();
    ok
}

fn list_avd_names(emulator_bin: &str) -> Vec<String> {
    let (_, avds) = shell::run(emulator_bin, &["-list-avds"]);
    avds.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect()
}

/// An AVD built from a Play Store system image refuses `adb root`
/// unconditionally — not something this tool (or any tool) can work around.
pub fn avd_is_rootable(name: &str) -> bool {
    avd_config_path(name)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|contents| !contents.lines().any(|l| l.trim() == "PlayStore.enabled=true"))
        .unwrap_or(true) // unknown config shouldn't block the flow
}

/// Prefers an AVD known to support root; falls back to whatever exists.
pub fn preferred_avd_name() -> Option<String> {
    let bin = shell::locate("emulator", "-version", sdk::emulator_path())?;
    let names = list_avd_names(&bin);
    names.iter().find(|n| avd_is_rootable(n)).cloned().or_else(|| names.into_iter().next())
}

/// True if none of the existing AVDs support root (including "no AVDs at all").
pub fn needs_rootable_avd() -> bool {
    let Some(bin) = shell::locate("emulator", "-version", sdk::emulator_path()) else {
        return true;
    };
    !list_avd_names(&bin).iter().any(|n| avd_is_rootable(n))
}

/// Launches the preferred (rootable, if one exists) AVD — falling back to
/// DEFAULT_AVD only when none exist at all (that only works if 'Create new
/// AVD' already ran).
pub fn launch_avd(tx: &Sender<String>) -> bool {
    let Some(bin) = shell::locate("emulator", "-version", sdk::emulator_path()) else {
        tx.send("[MISSING] Android emulator not found.".into()).ok();
        return false;
    };
    let name = preferred_avd_name().unwrap_or_else(|| DEFAULT_AVD.to_string());
    tx.send(format!("== Launching '{name}' ==")).ok();

    tx.send("> clearing stale adb device state...".into()).ok();
    shell::adb_run(&["kill-server"]);

    match shell::spawn_bg(&bin, &["-avd", &name, "-writable-system"]) {
        Ok(()) => {
            tx.send(format!("[OK] '{name}' launching in the background (boot takes ~30-90s).")).ok();
            true
        }
        Err(e) => {
            tx.send(format!("[FAIL] {e}")).ok();
            false
        }
    }
}

/// Restarts adbd as root (`adb root`). Only works on non-Play-Store/"userdebug"
/// system images — production (Play Store) builds refuse this unconditionally.
pub fn root_device(tx: &Sender<String>) {
    tx.send("== adb root ==".into()).ok();
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }
    let (ok, out) = shell::adb_run(&["root"]);
    let rooted = ok && !out.contains("cannot run as root");
    if rooted {
        tx.send(format!("[OK] {out}")).ok();
    } else {
        tx.send(format!("[FAIL] {out}")).ok();
        tx.send("This device's image doesn't support root (it's a Play Store system image —".into()).ok();
        tx.send("adb root is disabled unconditionally on these, no workaround exists).".into()).ok();
        tx.send("Run 'Create new AVD': it builds a Google APIs image (no Play Store, Android 13,".into()).ok();
        tx.send("x86_64) that's rootable and has the widest package/API compatibility for testing.".into())
            .ok();
    }
}

/// Restarts adbd back to unprivileged mode (`adb unroot`).
pub fn deroot_device(tx: &Sender<String>) {
    tx.send("== adb unroot ==".into()).ok();
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }
    let (ok, out) = shell::adb_run(&["unroot"]);
    tx.send(if ok { format!("[OK] {out}") } else { format!("[FAIL] {out}") }).ok();
}

/// Polls `sys.boot_completed` until the device finishes booting or times out.
pub fn wait_for_boot(tx: &Sender<String>, timeout_secs: u64) -> bool {
    tx.send(format!("> waiting up to {timeout_secs}s for the device to finish booting...")).ok();
    let start = std::time::Instant::now();
    let mut last_heartbeat = 0u64;
    while start.elapsed().as_secs() < timeout_secs {
        let (ok, out) = shell::adb_run(&["shell", "getprop", "sys.boot_completed"]);
        if ok && out.trim() == "1" {
            tx.send("[OK] device finished booting.".into()).ok();
            return true;
        }
        let elapsed = start.elapsed().as_secs();
        if elapsed / 15 > last_heartbeat / 15 {
            tx.send(format!("> still booting... ({elapsed}s)")).ok();
        }
        last_heartbeat = elapsed;
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    tx.send(format!("[FAIL] device didn't finish booting within {timeout_secs}s — check it manually.")).ok();
    false
}

/// Condenses config.ini into one readable fragment, e.g. "Android 37.0, x86_64, 2816MB RAM".
fn avd_summary(name: &str) -> Option<String> {
    let contents = std::fs::read_to_string(avd_config_path(name)?).ok()?;
    let value_of = |key: &str| -> Option<String> {
        contents
            .lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_once('='))
            .map(|(_, v)| v.trim().to_string())
    };

    let abi = value_of("abi.type").unwrap_or_else(|| "?".into());
    // Some AVDs store a bare MB number ("2816"), others a value+unit ("2G") — only append "MB" to the former.
    let ram = value_of("hw.ramSize")
        .map(|r| if r.chars().all(|c| c.is_ascii_digit()) { format!("{r}MB RAM") } else { format!("{r} RAM") })
        .unwrap_or_else(|| "? RAM".into());
    let api = value_of("image.sysdir.1")
        .and_then(|sysdir| {
            sysdir
                .split(['\\', '/'])
                .find(|part| part.starts_with("android-"))
                .map(|part| part.trim_start_matches("android-").to_string())
        })
        .unwrap_or_else(|| "?".into());

    Some(format!("Android {api}, {abi}, {ram}"))
}

/// AVDs live under ~/.android/avd regardless of where the SDK itself is installed.
fn avd_config_path(name: &str) -> Option<std::path::PathBuf> {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok()?;
    Some(
        std::path::Path::new(&home)
            .join(".android")
            .join("avd")
            .join(format!("{name}.avd"))
            .join("config.ini"),
    )
}
