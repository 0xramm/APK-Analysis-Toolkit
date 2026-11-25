use crate::util::shell;

pub const REPO: &str = "0xramm/APK-Analysis-Toolkit";
const ASSET_NAME: &str = "apk-toolkit.exe";

/// Checks GitHub's latest release against the running version, and if newer,
/// downloads it, swaps it in for the current exe, and relaunches — all before
/// the TUI starts. Silently gives up on any failure (offline, API down, no
/// release yet) so a normal launch never blocks on network issues.
pub fn check_and_apply(current_version: &str) {
    cleanup_previous_version();

    let Some((latest, asset_url)) = fetch_latest() else { return };
    if !is_newer(&latest, current_version) {
        return;
    }

    println!("Update available: v{current_version} -> v{latest}. Downloading...");
    let Ok(current_exe) = std::env::current_exe() else { return };
    let new_path = current_exe.with_file_name("apk-toolkit.new.exe");
    let new_path_str = new_path.to_string_lossy().to_string();

    let (ok, out) = shell::run("curl", &["-L", "--max-time", "60", "-o", &new_path_str, &asset_url]);
    if !ok {
        eprintln!("Update download failed: {out}");
        return;
    }

    let old_path = current_exe.with_file_name("apk-toolkit.old.exe");
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
        let old_path = current_exe.with_file_name("apk-toolkit.old.exe");
        let _ = std::fs::remove_file(old_path);
    }
}

/// Returns (version without the leading 'v', asset download URL) for the latest release.
fn fetch_latest() -> Option<(String, String)> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let (ok, body) = shell::run("curl", &["-s", "--max-time", "5", "-H", "User-Agent: apk-toolkit", &url]);
    if !ok || body.is_empty() {
        return None;
    }
    let tag = json_string(&body, "tag_name")?;
    let version = tag.trim_start_matches('v').to_string();
    let asset_url = json_string_matching(&body, "browser_download_url", ASSET_NAME)?;
    Some((version, asset_url))
}

/// First `"key": "value"` match.
fn json_string(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let after = &json[json.find(&pat)? + pat.len()..];
    let after = &after[after.find(':')? + 1..].trim_start();
    let after = after.strip_prefix('"')?;
    Some(after[..after.find('"')?].to_string())
}

/// First `"key": "value"` match whose value contains `must_contain` — used to
/// pick the right asset URL out of a release's (possibly multi-entry) asset list.
fn json_string_matching(json: &str, key: &str, must_contain: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let mut search_from = 0;
    while let Some(rel_idx) = json[search_from..].find(&pat) {
        let idx = search_from + rel_idx;
        let after = &json[idx + pat.len()..];
        let Some(colon) = after.find(':') else { break };
        let after = after[colon + 1..].trim_start();
        if let Some(rest) = after.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                let value = &rest[..end];
                if value.contains(must_contain) {
                    return Some(value.to_string());
                }
                search_from = idx + pat.len() + colon + 1 + end;
                continue;
            }
        }
        break;
    }
    None
}

/// Plain numeric semver comparison (handles "0.9.0" < "0.10.0" correctly,
/// unlike a string compare).
fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    parse(candidate) > parse(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed real-world shape: pretty-printed, multiple assets, non-matching
    // entries before the one we want (name differs from the URL's filename).
    const SAMPLE: &str = r#"{
  "url": "https://api.github.com/repos/0xramm/APK-Analysis-Toolkit/releases/1",
  "tag_name": "v0.2.0",
  "name": "v0.2.0",
  "assets": [
    {
      "name": "apk-toolkit-source.tar.gz",
      "browser_download_url": "https://github.com/0xramm/APK-Analysis-Toolkit/releases/download/v0.2.0/apk-toolkit-source.tar.gz"
    },
    {
      "name": "apk-toolkit.exe",
      "browser_download_url": "https://github.com/0xramm/APK-Analysis-Toolkit/releases/download/v0.2.0/apk-toolkit.exe"
    }
  ]
}"#;

    #[test]
    fn parses_tag_and_picks_the_right_asset() {
        assert_eq!(json_string(SAMPLE, "tag_name").as_deref(), Some("v0.2.0"));
        assert_eq!(
            json_string_matching(SAMPLE, "browser_download_url", "apk-toolkit.exe").as_deref(),
            Some("https://github.com/0xramm/APK-Analysis-Toolkit/releases/download/v0.2.0/apk-toolkit.exe")
        );
    }

    #[test]
    fn version_compare_handles_double_digits() {
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(!is_newer("0.9.0", "0.10.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
    }
}
