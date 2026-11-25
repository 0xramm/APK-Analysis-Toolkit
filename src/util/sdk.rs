use std::path::{Path, PathBuf};

/// Finds the Android SDK root: env vars first, then common per-OS install locations.
pub fn sdk_root() -> Option<PathBuf> {
    for var in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
        if let Ok(p) = std::env::var(var) {
            let p = PathBuf::from(p);
            if p.exists() {
                return Some(p);
            }
        }
    }

    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok();
    let mut candidates = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Android").join("Sdk"));
    }
    if let Some(h) = &home {
        candidates.push(PathBuf::from(h).join("Library/Android/sdk"));
        candidates.push(PathBuf::from(h).join("Android/Sdk"));
    }
    candidates.into_iter().find(|p| p.exists())
}

pub fn emulator_path() -> Option<PathBuf> {
    let root = sdk_root()?;
    [root.join("emulator/emulator.exe"), root.join("emulator/emulator")]
        .into_iter()
        .find(|p| p.exists())
}

pub fn adb_path() -> Option<PathBuf> {
    let root = sdk_root()?;
    [root.join("platform-tools/adb.exe"), root.join("platform-tools/adb")]
        .into_iter()
        .find(|p| p.exists())
}

pub fn avdmanager_path() -> Option<PathBuf> {
    find_in_cmdline_tools(&sdk_root()?, "avdmanager")
}

pub fn sdkmanager_path() -> Option<PathBuf> {
    find_in_cmdline_tools(&sdk_root()?, "sdkmanager")
}

/// apksigner lives under build-tools/<version>/, picks the newest version present.
pub fn apksigner_path() -> Option<PathBuf> {
    build_tools_file(&["apksigner.bat", "apksigner"])
}

/// aapt (manifest/label/permissions dump) also lives under build-tools/<version>/.
pub fn aapt_path() -> Option<PathBuf> {
    build_tools_file(&["aapt.exe", "aapt"])
}

fn build_tools_file(names: &[&str]) -> Option<PathBuf> {
    let root = sdk_root()?;
    let base = root.join("build-tools");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&base).ok()?.flatten().map(|e| e.path()).collect();
    versions.sort(); // version dirs sort correctly as plain strings for this purpose
    versions.into_iter().rev().find_map(|dir| names.iter().map(|n| dir.join(n)).find(|p| p.exists()))
}

/// Git for Windows bundles xz/openssl/curl under its own bin dirs, which aren't
/// always on every shell's PATH even when they're on the "real" system PATH.
pub fn git_for_windows_tool(name: &str) -> Option<PathBuf> {
    for dir in [r"C:\Program Files\Git\mingw64\bin", r"C:\Program Files\Git\usr\bin"] {
        let candidate = Path::new(dir).join(format!("{name}.exe"));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// cmdline-tools binaries live under cmdline-tools/<version>/bin/, version usually "latest".
fn find_in_cmdline_tools(root: &Path, tool: &str) -> Option<PathBuf> {
    let base = root.join("cmdline-tools");
    let mut dirs = vec![base.join("latest")];
    if let Ok(entries) = std::fs::read_dir(&base) {
        dirs.extend(entries.flatten().map(|e| e.path()));
    }
    for dir in dirs {
        for fname in [format!("{tool}.bat"), tool.to_string()] {
            let candidate = dir.join("bin").join(&fname);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}
