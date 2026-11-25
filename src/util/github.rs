use crate::util::shell;

/// Raw JSON body of a repo's `/releases/latest`. None on any failure
/// (offline, API down, no release yet).
pub fn fetch_release_json(repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let (ok, body) = shell::run("curl", &["-s", "--max-time", "8", "-H", "User-Agent: apk-toolkit", &url]);
    (ok && !body.is_empty()).then_some(body)
}

/// Returns (tag_name without leading 'v', asset download URL) for a repo's
/// latest release, picking the one asset whose name contains `asset_contains`.
/// Only safe when exactly one asset is expected to match — if a release has
/// several similarly-named assets (e.g. a GUI-only build alongside the full
/// one), fetch the JSON yourself and match on the exact, version-qualified name.
pub fn latest_release_asset(repo: &str, asset_contains: &str) -> Option<(String, String)> {
    let body = fetch_release_json(repo)?;
    let tag = json_string(&body, "tag_name")?;
    let version = tag.trim_start_matches('v').to_string();
    let asset_url = json_string_matching(&body, "browser_download_url", asset_contains)?;
    Some((version, asset_url))
}

/// First `"key": "value"` match.
pub fn json_string(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let after = &json[json.find(&pat)? + pat.len()..];
    let after = &after[after.find(':')? + 1..].trim_start();
    let after = after.strip_prefix('"')?;
    Some(after[..after.find('"')?].to_string())
}

/// First `"key": "value"` match whose value contains `must_contain` — used to
/// pick the right asset URL out of a release's (possibly multi-entry) asset list.
pub fn json_string_matching(json: &str, key: &str, must_contain: &str) -> Option<String> {
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
pub fn is_newer(candidate: &str, current: &str) -> bool {
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
