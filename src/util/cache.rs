use std::path::PathBuf;

/// Persistent download cache so re-running a step doesn't re-fetch what's
/// already on disk. Lives outside %TEMP% so it survives cleanup/reboots.
pub fn dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
    let path = PathBuf::from(base).join("apk-toolkit").join("cache");
    let _ = std::fs::create_dir_all(&path);
    path
}
