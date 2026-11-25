use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use super::apps;
use crate::util::{cache, github, shell};

const REPO: &str = "skylot/jadx";

fn install_dir() -> PathBuf {
    cache::dir().join("jadx")
}

fn gui_bin() -> Option<PathBuf> {
    let candidate = install_dir().join("bin").join("jadx-gui.bat");
    candidate.exists().then_some(candidate)
}

/// Ensures JADX is extracted locally, downloading the latest release if not.
/// Returns the path to `jadx-gui.bat` on success.
pub fn install(tx: &Sender<String>) -> Option<PathBuf> {
    if let Some(bin) = gui_bin() {
        return Some(bin);
    }

    tx.send("== Installing JADX ==".into()).ok();
    if !shell::is_installed("java", "-version") {
        tx.send("[MISSING] Java not found — JADX needs a JRE. Install Java first.".into()).ok();
        return None;
    }

    tx.send("> checking latest JADX release...".into()).ok();
    let Some(body) = github::fetch_release_json(REPO) else {
        tx.send("[FAIL] couldn't reach GitHub to find the latest JADX release.".into()).ok();
        return None;
    };
    let Some(tag) = github::json_string(&body, "tag_name") else {
        tx.send("[FAIL] couldn't parse the latest JADX release.".into()).ok();
        return None;
    };
    let version = tag.trim_start_matches('v');
    // The release also ships GUI-only platform builds (jadx-gui-<v>-win.zip,
    // *.dmg) — match the exact full-bundle name so we don't grab one of those.
    let exact_name = format!("jadx-{version}.zip");
    let Some(url) = github::json_string_matching(&body, "browser_download_url", &exact_name) else {
        tx.send(format!("[FAIL] couldn't find a {exact_name} asset in the latest release.")).ok();
        return None;
    };

    let zip_path = cache::dir().join(&exact_name);
    let zip_str = zip_path.to_string_lossy().to_string();
    tx.send(format!("> downloading {exact_name}...")).ok();
    let (ok, out) = shell::run_streaming("curl", &["-L", "-o", &zip_str, &url], tx);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return None;
    }

    let dir = install_dir();
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    tx.send("> extracting...".into()).ok();
    let (ok, out) = shell::run("tar", &["-xf", &zip_str, "-C", &dir.to_string_lossy()]);
    let _ = std::fs::remove_file(&zip_path);
    if !ok {
        tx.send(format!("[FAIL] extract failed: {out}")).ok();
        return None;
    }

    let bin = dir.join("bin").join("jadx-gui.bat");
    if !bin.exists() {
        tx.send("[FAIL] extracted, but jadx-gui.bat wasn't where expected.".into()).ok();
        return None;
    }
    tx.send(format!("[OK] JADX {version} ready")).ok();
    Some(bin)
}

/// Picks a package from the device, pulls its APK into `workspace/jadx/<pkg>/`,
/// and opens it directly in jadx-gui.
pub fn open_package(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Opening {pkg} in JADX ==")).ok();

    let Some(gui) = install(tx) else { return };
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    let dest_dir = Path::new("workspace").join("jadx").join(pkg);
    tx.send("> pulling APK from device...".into()).ok();
    let Some(apk) = apps::pull_apk_to(pkg, &dest_dir, tx) else { return };
    tx.send(format!("[OK] pulled to {}", apk.display())).ok();

    tx.send("> launching jadx-gui...".into()).ok();
    match shell::spawn_bg(&gui.to_string_lossy(), &[&apk.to_string_lossy()]) {
        Ok(()) => {
            tx.send("[OK] jadx-gui launched.".into()).ok();
        }
        Err(e) => {
            tx.send(format!("[FAIL] {e}")).ok();
        }
    }
}
