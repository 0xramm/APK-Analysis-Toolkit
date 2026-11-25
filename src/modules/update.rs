use crate::util::{cache, github, shell};

pub const REPO: &str = "0xramm/APK-Analysis-Toolkit";
const ASSET_NAME: &str = "apkat.exe";

/// Checks GitHub's latest release against the running version, and if newer,
/// downloads it, swaps it in for the current exe, and relaunches — all before
/// the TUI starts. Silently gives up on any failure (offline, API down, no
/// release yet) so a normal launch never blocks on network issues.
pub fn check_and_apply(current_version: &str) {
    cleanup_previous_version();

    let Some((latest, asset_url)) = github::latest_release_asset(REPO, ASSET_NAME) else { return };
    if !github::is_newer(&latest, current_version) {
        return;
    }

    // A release whose uploaded exe doesn't actually report the new version
    // (e.g. Cargo.toml wasn't bumped before tagging) would otherwise redownload
    // the same broken asset forever, since every relaunch sees the same tag as
    // "newer". Once we've tried a given tag and it didn't move the version,
    // don't try it again until a genuinely different tag appears.
    let attempted_marker = cache::dir().join("update_attempted.tag");
    if std::fs::read_to_string(&attempted_marker).ok().as_deref() == Some(latest.as_str()) {
        return;
    }
    let _ = std::fs::write(&attempted_marker, &latest);

    println!("Update available: v{current_version} -> v{latest}. Downloading...");
    let Ok(current_exe) = std::env::current_exe() else { return };
    let new_path = current_exe.with_file_name("apkat.new.exe");
    let new_path_str = new_path.to_string_lossy().to_string();

    let (ok, out) = shell::run("curl", &["-L", "--max-time", "60", "-o", &new_path_str, &asset_url]);
    if !ok {
        eprintln!("Update download failed: {out}");
        return;
    }

    let old_path = current_exe.with_file_name("apkat.old.exe");
    let _ = std::fs::remove_file(&old_path);
    if std::fs::rename(&current_exe, &old_path).is_err() {
        eprintln!("Update failed: couldn't move aside the running exe.");
        return;
    }
    if std::fs::rename(&new_path, &current_exe).is_err() {
        eprintln!("Update failed: couldn't install the new exe.");
        let _ = std::fs::rename(&old_path, &current_exe); // best-effort revert
        return;
    }

    println!("Updated to v{latest}, relaunching...");
    let _ = shell::spawn_bg(&current_exe.to_string_lossy(), &[]);
    std::process::exit(0);
}

/// Removes the previous version's renamed-aside exe, now that this (new)
/// process holds no lock on it — the one file Windows wouldn't let us delete
/// mid-update last launch.
fn cleanup_previous_version() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_path = current_exe.with_file_name("apkat.old.exe");
        let _ = std::fs::remove_file(old_path);
    }
}
