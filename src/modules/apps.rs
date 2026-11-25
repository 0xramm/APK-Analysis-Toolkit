use std::sync::mpsc::Sender;

use crate::util::{cache, sdk, shell};

/// Permissions that are individually unremarkable but, combined with other
/// signals (name impersonation, placeholder version, sideloaded), are the
/// building blocks of droppers, overlay/banking trojans, and OTP stealers.
const SENSITIVE_PERMISSIONS: &[(&str, &str)] = &[
    ("INSTALL_PACKAGES", "can silently install other apps (dropper capability)"),
    ("REQUEST_INSTALL_PACKAGES", "can request installing other apps"),
    ("REQUEST_DELETE_PACKAGES", "can request removing other apps"),
    ("OVERRIDE_PACKAGE_VERIFICATION", "can bypass Play Protect / package verification"),
    ("QUERY_ALL_PACKAGES", "can enumerate every installed app"),
    ("BIND_ACCESSIBILITY_SERVICE", "can abuse Accessibility Service (screen reading, auto-click — common banking-trojan technique)"),
    ("SYSTEM_ALERT_WINDOW", "can draw over other apps (overlay/fake-login-screen attacks)"),
    ("BIND_DEVICE_ADMIN", "can request device admin (hard to uninstall, lock-screen control)"),
    ("READ_SMS", "can read SMS (OTP/2FA theft)"),
    ("RECEIVE_SMS", "can intercept incoming SMS (OTP/2FA theft)"),
    ("SEND_SMS", "can send SMS (premium-rate fraud, worm propagation)"),
    ("READ_CALL_LOG", "can harvest call history"),
    ("READ_CONTACTS", "can harvest contacts"),
    ("RECORD_AUDIO", "can record audio"),
    ("CAMERA", "can access the camera"),
];

/// The exact DN of AOSP's publicly known "testkey" (build/target/product/security/testkey.*
/// in the AOSP source tree). Genuine Google/OEM production apps are never signed with it —
/// seeing it on a device is close to definitive proof of a repackaged/tampered APK.
const AOSP_TESTKEY_MARKERS: &[&str] = &["CN=Android", "O=Android", "android@android.com"];

/// Sorted installed package names, or empty if no device / the call failed.
/// Shared by the log-streaming `list_packages` and the TUI's package picker.
pub fn packages() -> Vec<String> {
    if !shell::adb_device_connected() {
        return Vec::new();
    }
    let (ok, out) = shell::adb_run(&["shell", "pm", "list", "packages"]);
    if !ok {
        return Vec::new();
    }
    // No name-based filtering here — the picker's type-to-filter search
    // handles narrowing the list, and hiding packages by name is exactly
    // the kind of thing a disguised/malicious package can exploit.
    let mut pkgs: Vec<String> =
        out.lines().filter_map(|l| l.strip_prefix("package:")).map(str::to_string).collect();
    pkgs.sort_unstable();
    pkgs
}

pub fn list_packages(tx: &Sender<String>) {
    tx.send("== Installed packages ==".into()).ok();
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> pm list packages...".into()).ok();
    let pkgs = packages();
    if pkgs.is_empty() {
        tx.send("[FAIL] no packages found (or the adb call failed).".into()).ok();
        return;
    }
    tx.send(format!("[OK] {} packages", pkgs.len())).ok();
    for p in pkgs {
        tx.send(format!("    {p}")).ok();
    }
}

/// Launches a package's launcher activity via `monkey`, which resolves it
/// automatically instead of needing to guess `pkg/.MainActivity`.
pub fn launch(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Launching {pkg} ==")).ok();
    if pkg.is_empty() {
        tx.send("[FAIL] No package name given.".into()).ok();
        return;
    }
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> monkey -p <pkg> -c LAUNCHER...".into()).ok();
    let (ok, out) = shell::adb_run(&["shell", "monkey", "-p", pkg, "-c", "android.intent.category.LAUNCHER", "1"]);
    tx.send(if ok && out.contains("Events injected: 1") {
        "[OK] launched".into()
    } else {
        format!("[FAIL] {out}")
    })
    .ok();
}

/// Pulls everything `dumpsys package <pkg>` exposes and presents it as a
/// curated report (identity, paths, flags, permissions, user state), with
/// the full raw dump appended so nothing extractable is left out.
pub fn package_info(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Package info: {pkg} ==")).ok();

    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> dumpsys package...".into()).ok();
    let (ok, out) = shell::adb_run(&["shell", "dumpsys", "package", pkg]);
    if !ok || out.trim().is_empty() {
        tx.send("[FAIL] no info returned — check the package name.".into()).ok();
        return;
    }
    let lines: Vec<&str> = out.lines().collect();

    let field = |marker: &str| -> Option<String> {
        lines.iter().find(|l| l.trim_start().starts_with(marker)).map(|l| l.trim().to_string())
    };
    let section = |tx: &Sender<String>, title: &str, markers: &[&str]| {
        let found: Vec<String> = markers.iter().filter_map(|m| field(m)).collect();
        if found.is_empty() {
            return;
        }
        tx.send(String::new()).ok();
        tx.send(format!("-- {title} --")).ok();
        for line in found {
            tx.send(format!("  {line}")).ok();
        }
    };

    tx.send("[OK] report:".into()).ok();

    // Pull the APK once (if possible) and reuse it for both the manifest
    // dump and the signature check, instead of pulling it twice.
    tx.send("> pulling APK for manifest/signature checks...".into()).ok();
    let apk_path = pull_apk_to(pkg, &cache::dir(), tx);
    let manifest = apk_path.as_ref().and_then(|p| manifest_info(p));
    let cert = apk_path.as_ref().and_then(|p| signing_cert(p));
    if let Some(p) = &apk_path {
        let _ = std::fs::remove_file(p); // don't leave pulled APKs lying around in the cache
    }

    tx.send(String::new()).ok();
    tx.send(format!("Package: {pkg}")).ok();
    tx.send(format!("App label: {}", manifest.as_ref().map(|m| m.label.as_str()).unwrap_or("(unknown — aapt unavailable or pull failed)"))).ok();

    let requested = indented_block_after(&lines, "requested permissions:");
    let code_path = field("codePath=").unwrap_or_default();
    let looks_like_system_name = pkg.starts_with("com.google") || pkg.starts_with("com.android") || pkg.starts_with("android.");
    let sideloaded = !code_path.contains("/system") && !code_path.contains("/product") && !code_path.contains("/vendor") && !code_path.contains("/apex");

    let mut flags: Vec<String> = Vec::new();
    if looks_like_system_name && sideloaded {
        flags.push(format!(
            "Package name '{pkg}' impersonates a system/Google component{}, but is installed as a \
             regular sideloaded app ({}). Real system packages live in /system, /product, or /vendor — \
             never /data/app.",
            manifest.as_ref().map(|m| format!(" (app label: \"{}\")", m.label)).unwrap_or_default(),
            if code_path.is_empty() { "codePath unknown" } else { &code_path }
        ));
    }
    let version_code = field("versionCode=").and_then(|l| l.split_whitespace().next()?.strip_prefix("versionCode=").map(str::to_string));
    let version_name = field("versionName=").and_then(|l| l.strip_prefix("versionName=").map(|v| v.trim().to_string()));
    if version_code.as_deref() == Some("1") || version_name.as_deref() == Some("1.0") {
        flags.push("Placeholder version (versionCode=1 / versionName=1.0) — unusual for any app with real update history.".into());
    }
    if field("flags=[").map(|l| l.contains("DEBUGGABLE")).unwrap_or(false) {
        flags.push("Marked DEBUGGABLE — unusual for a release build; expands attack surface (can be attached to, run as any user).".into());
    }
    for (perm, reason) in SENSITIVE_PERMISSIONS {
        if requested.iter().any(|p| p.contains(perm)) {
            flags.push(format!("Requests {perm} — {reason}."));
        }
    }
    if let Some((dn, _)) = &cert {
        if AOSP_TESTKEY_MARKERS.iter().all(|m| dn.contains(m)) {
            flags.push(
                "Signed with AOSP's publicly known \"testkey\" (build/target/product/security/testkey.* \
                 in the AOSP source) — legitimate Google/OEM production apps are NEVER signed with this \
                 key. Near-definitive evidence of a repackaged or tampered APK."
                    .into(),
            );
        }
    }

    if !flags.is_empty() {
        tx.send(String::new()).ok();
        tx.send(format!("-- !! RED FLAGS ({}) !! --", flags.len())).ok();
        for f in &flags {
            tx.send(format!("  ! {f}")).ok();
        }
    }

    if let Some(m) = &manifest {
        tx.send(String::new()).ok();
        tx.send("-- Manifest info (aapt) --".into()).ok();
        tx.send(format!("  label: {}", m.label)).ok();
        if let Some(a) = &m.launchable_activity {
            tx.send(format!("  launchable-activity: {a}")).ok();
        }
        if let Some(n) = &m.native_code {
            tx.send(format!("  native-code: {n}")).ok();
        }
        if let Some(s) = &m.sdk_versions {
            tx.send(format!("  {s}")).ok();
        }
    }

    if let Some((dn, sha256)) = &cert {
        tx.send(String::new()).ok();
        tx.send("-- Signing certificate --".into()).ok();
        tx.send(format!("  DN: {dn}")).ok();
        tx.send(format!("  SHA-256: {sha256}")).ok();
    } else if apk_path.is_some() {
        tx.send(String::new()).ok();
        tx.send("[!] Couldn't read a signing certificate (apksigner missing, or the APK is unsigned/corrupt — itself worth noting).".into()).ok();
    }

    section(tx, "Identity", &["versionCode=", "versionName=", "firstInstallTime=", "lastUpdateTime=", "installerPackageName="]);
    section(tx, "Install paths", &["codePath=", "resourcePath=", "dataDir=", "primaryCpuAbi=", "secondaryCpuAbi="]);
    section(tx, "Flags", &["flags=[", "privateFlags=[", "pkgFlags=["]);
    section(tx, "User state", &["installed=", "hidden=", "suspended=", "stopped=", "enabled="]);

    if !requested.is_empty() {
        tx.send(String::new()).ok();
        tx.send(format!("-- Requested permissions ({}) --", requested.len())).ok();
        for p in &requested {
            tx.send(format!("  {p}")).ok();
        }
    }
    let runtime = indented_block_after(&lines, "runtime permissions:");
    if !runtime.is_empty() {
        tx.send(String::new()).ok();
        tx.send(format!("-- Runtime permissions granted ({}) --", runtime.len())).ok();
        for p in runtime {
            tx.send(format!("  {p}")).ok();
        }
    }

    if let Some(m) = &manifest {
        tx.send(String::new()).ok();
        tx.send(format!("-- Raw aapt dump badging output ({} lines) --", m.raw.len())).ok();
        for l in &m.raw {
            tx.send(l.clone()).ok();
        }
    }

    tx.send(String::new()).ok();
    tx.send(format!("-- Raw dumpsys output ({} lines) --", lines.len())).ok();
    for l in lines {
        tx.send(l.to_string()).ok();
    }
}

/// Pulls an installed package's APK into `dest_dir`. None if the device/pull/pm
/// path lookup fails. Shared by package info (signature/manifest checks),
/// apktool, and JADX — each decides for itself whether to keep the file after.
pub fn pull_apk_to(pkg: &str, dest_dir: &std::path::Path, tx: &Sender<String>) -> Option<std::path::PathBuf> {
    let (ok, path_out) = shell::adb_run(&["shell", "pm", "path", pkg]);
    if !ok {
        return None;
    }
    let device_path = path_out.lines().find_map(|l| l.strip_prefix("package:"))?.trim();

    let _ = std::fs::create_dir_all(dest_dir);
    let local = dest_dir.join(format!("{pkg}.apk"));
    let local_str = local.to_string_lossy().to_string();
    let (ok, _) = shell::adb_run(&["pull", device_path, &local_str]);
    if !ok {
        tx.send("[!] couldn't pull the APK.".into()).ok();
        return None;
    }
    Some(local)
}

struct ManifestInfo {
    label: String,
    launchable_activity: Option<String>,
    native_code: Option<String>,
    sdk_versions: Option<String>,
    raw: Vec<String>,
}

/// Runs `aapt dump badging` on a local APK copy for the app's real label
/// (what the user actually sees, as opposed to the package ID) plus a few
/// other manifest-derived facts the installed-package dump doesn't carry.
fn manifest_info(apk_path: &std::path::Path) -> Option<ManifestInfo> {
    let aapt = shell::locate("aapt", "version", sdk::aapt_path())?;
    let apk_str = apk_path.to_string_lossy().to_string();
    let (ok, out) = shell::run(&aapt, &["dump", "badging", &apk_str]);
    if !ok {
        return None;
    }
    let raw: Vec<String> = out.lines().map(str::to_string).collect();

    let label = raw
        .iter()
        .find_map(|l| l.strip_prefix("application-label:"))
        .or_else(|| raw.iter().find_map(|l| l.split_once("application-label-en:").map(|(_, v)| v)))
        .map(|v| v.trim().trim_matches('\'').to_string())
        .unwrap_or_else(|| "(none set)".to_string());

    let launchable_activity =
        raw.iter().find_map(|l| l.strip_prefix("launchable-activity:")).map(|v| v.trim().to_string());
    let native_code = raw.iter().find_map(|l| l.strip_prefix("native-code:")).map(|v| v.trim().to_string());
    let sdk_lines: Vec<&str> = raw
        .iter()
        .filter(|l| l.starts_with("sdkVersion:") || l.starts_with("targetSdkVersion:"))
        .map(String::as_str)
        .collect();
    let sdk_versions = (!sdk_lines.is_empty()).then(|| sdk_lines.join(" "));

    Some(ManifestInfo { label, launchable_activity, native_code, sdk_versions, raw })
}

/// Runs `apksigner` against a local APK copy — a package name or permission
/// set can lie, but the signer is cryptographic. (DN, SHA-256), or None.
fn signing_cert(apk_path: &std::path::Path) -> Option<(String, String)> {
    let apksigner = shell::locate("apksigner", "--version", sdk::apksigner_path())?;
    let apk_str = apk_path.to_string_lossy().to_string();
    let (ok, out) = shell::run(&apksigner, &["verify", "--print-certs", &apk_str]);
    if !ok {
        return None;
    }
    let dn = out.lines().find_map(|l| l.split_once("certificate DN: ").map(|(_, v)| v.trim().to_string()))?;
    let sha256 =
        out.lines().find_map(|l| l.split_once("SHA-256 digest: ").map(|(_, v)| v.trim().to_string())).unwrap_or_default();
    Some((dn, sha256))
}

/// Collects the lines more-indented than `header` immediately following it —
/// e.g. the permission list under "requested permissions:".
fn indented_block_after(lines: &[&str], header: &str) -> Vec<String> {
    let Some(idx) = lines.iter().position(|l| l.trim() == header) else {
        return Vec::new();
    };
    let header_indent = lines[idx].len() - lines[idx].trim_start().len();
    lines[idx + 1..]
        .iter()
        .take_while(|l| !l.trim().is_empty() && l.len() - l.trim_start().len() > header_indent)
        .map(|l| l.trim().to_string())
        .collect()
}

pub fn install_apk(path: &str, tx: &Sender<String>) {
    // Windows' "Copy as path" wraps the result in "quotes" — strip them.
    let path = path.trim().trim_matches('"').trim_matches('\'').trim();
    tx.send(format!("== Installing {path} ==")).ok();
    if path.is_empty() {
        tx.send("[FAIL] No APK path given.".into()).ok();
        return;
    }
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> adb install -r...".into()).ok();
    let (ok, out) = shell::adb_run(&["install", "-r", path]);
    tx.send(if ok { "[OK] installed".into() } else { format!("[FAIL] {out}") }).ok();
}

pub fn uninstall(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Uninstalling {pkg} ==")).ok();
    if pkg.is_empty() {
        tx.send("[FAIL] No package name given.".into()).ok();
        return;
    }
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> adb uninstall...".into()).ok();
    let (ok, out) = shell::adb_run(&["uninstall", pkg]);
    tx.send(if ok { "[OK] uninstalled".into() } else { format!("[FAIL] {out}") }).ok();
}

pub fn disable(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Disabling {pkg} ==")).ok();
    if pkg.is_empty() {
        tx.send("[FAIL] No package name given.".into()).ok();
        return;
    }
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> pm disable-user...".into()).ok();
    let (ok, out) = shell::adb_run(&["shell", "pm", "disable-user", "--user", "0", pkg]);
    tx.send(if ok { format!("[OK] {out}") } else { format!("[FAIL] {out}") }).ok();
}

pub fn enable(pkg: &str, tx: &Sender<String>) {
    let pkg = pkg.trim();
    tx.send(format!("== Enabling {pkg} ==")).ok();
    if pkg.is_empty() {
        tx.send("[FAIL] No package name given.".into()).ok();
        return;
    }
    if !shell::adb_device_connected() {
        tx.send("[FAIL] No device connected.".into()).ok();
        return;
    }

    tx.send("> pm enable...".into()).ok();
    let (ok, out) = shell::adb_run(&["shell", "pm", "enable", pkg]);
    tx.send(if ok { format!("[OK] {out}") } else { format!("[FAIL] {out}") }).ok();
}
