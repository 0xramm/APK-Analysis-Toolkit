use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use super::apps;
use crate::util::{cache, github, shell};

const REPO: &str = "iBotPeaches/Apktool";

/// Everything pulled/decompiled/compiled lives under `workspace/` in the
/// current directory — "the local system of this repo", not a hidden cache.
fn workspace_dir() -> PathBuf {
    Path::new("workspace").join("apktool")
}

fn cached_jar() -> Option<PathBuf> {
    std::fs::read_dir(cache::dir()).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("apktool_") && n.ends_with(".jar"))
            .unwrap_or(false)
    })
}

/// Ensures apktool.jar is cached locally, downloading the latest release if
/// not. Returns the jar path on success.
pub fn install(tx: &Sender<String>) -> Option<PathBuf> {
    if let Some(jar) = cached_jar() {
        return Some(jar);
    }

    tx.send("== Installing apktool ==".into()).ok();
    if !shell::is_installed("java", "-version") {
        tx.send("[MISSING] Java not found — apktool needs a JRE. Install Java first.".into()).ok();
        return None;
    }

    tx.send("> checking latest apktool release...".into()).ok();
    let Some((version, url)) = github::latest_release_asset(REPO, "apktool_") else {
        tx.send("[FAIL] couldn't reach GitHub to find the latest apktool release.".into()).ok();
        return None;
    };
    let dest = cache::dir().join(format!("apktool_{version}.jar"));
    let dest_str = dest.to_string_lossy().to_string();

    tx.send(format!("> downloading apktool {version}...")).ok();
    let (ok, out) = shell::run_streaming("curl", &["-L", "-o", &dest_str, &url], tx);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return None;
    }
    tx.send(format!("[OK] apktool {version} ready")).ok();
    Some(dest)
}

/// Picks a package from the device, pulls its APK into `workspace/apktool/<pkg>/`,
/// decompiles it there, and opens the result in the file explorer.
pub fn decompile_package(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Decompiling {pkg} with apktool ==")).ok();

    let Some(jar) = install(tx) else { return };
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    let dest_dir = workspace_dir().join(pkg);
    tx.send("> pulling APK from device...".into()).ok();
    let Some(apk) = apps::pull_apk_to(pkg, &dest_dir, tx) else { return };
    tx.send(format!("[OK] pulled to {}", apk.display())).ok();

    let out_dir = dest_dir.join("decompiled");
    let _ = std::fs::remove_dir_all(&out_dir); // apktool refuses to overwrite without -f; start clean instead

    tx.send("> apktool d ...".into()).ok();
    let (ok, out) = shell::run_streaming(
        "java",
        &["-jar", &jar.to_string_lossy(), "d", &apk.to_string_lossy(), "-o", &out_dir.to_string_lossy(), "-f"],
        tx,
    );
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return;
    }
    tx.send(format!("[OK] decompiled to {}", out_dir.display())).ok();

    tx.send("> opening in file explorer...".into()).ok();
    let _ = shell::open_url(&out_dir.to_string_lossy());
}

/// Rebuilds a decompiled folder (as produced above, or edited by hand) back into an APK.
pub fn compile_folder(folder: &str, tx: &Sender<String>) {
    let folder = folder.trim().trim_matches('"').trim_matches('\'');
    tx.send(format!("== Compiling {folder} with apktool ==")).ok();
    if folder.is_empty() {
        tx.send("[FAIL] No folder given.".into()).ok();
        return;
    }

    let Some(jar) = install(tx) else { return };

    tx.send("> apktool b ...".into()).ok();
    let (ok, out) = shell::run_streaming("java", &["-jar", &jar.to_string_lossy(), "b", folder], tx);
    if !ok {
        tx.send(format!("[FAIL] {out}")).ok();
        return;
    }
    let out_apk = Path::new(folder).join("dist");
    tx.send(format!("[OK] compiled — output in {}", out_apk.display())).ok();
    let _ = shell::open_url(&out_apk.to_string_lossy());
}
