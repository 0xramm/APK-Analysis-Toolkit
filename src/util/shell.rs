use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

use super::sdk;

/// Windows can't exec .bat/.cmd directly via CreateProcess — route through cmd /C.
fn build(cmd: &str, args: &[&str]) -> Command {
    if cfg!(windows) && (cmd.ends_with(".bat") || cmd.ends_with(".cmd")) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(cmd).args(args);
        c
    } else {
        let mut c = Command::new(cmd);
        c.args(args);
        c
    }
}

/// Runs `cmd args...` and returns (success, combined stdout+stderr, trimmed).
/// stdin is nulled so a prompting subprocess can't hang the TUI thread.
pub fn run(cmd: &str, args: &[&str]) -> (bool, String) {
    match build(cmd, args).stdin(Stdio::null()).output() {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            (out.status.success(), s.trim().to_string())
        }
        Err(e) => (false, format!("`{cmd}` not found: {e}")),
    }
}

/// Cheap "is this on PATH and does it respond" check.
pub fn is_installed(cmd: &str, version_arg: &str) -> bool {
    build(cmd, &[version_arg])
        .stdin(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Starts a long-running process and hands back the `Child` (e.g. so its
/// PID can be tracked and killed later, like a frida bypass session).
pub fn spawn_child(cmd: &str, args: &[&str]) -> Result<std::process::Child, String> {
    build(cmd, args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("`{cmd}` failed to start: {e}"))
}

/// [`spawn_child`], discarding the handle, for fire-and-forget background processes.
pub fn spawn_bg(cmd: &str, args: &[&str]) -> Result<(), String> {
    spawn_child(cmd, args).map(|_| ())
}

/// Kills a process by PID (e.g. a tracked frida session).
pub fn kill_pid(pid: u32) -> (bool, String) {
    if cfg!(windows) {
        run("taskkill", &["/PID", &pid.to_string(), "/T", "/F"])
    } else {
        run("kill", &["-9", &pid.to_string()])
    }
}

/// Resolves a tool to something runnable: tries the bare name on PATH first,
/// then falls back to an SDK-discovered absolute path (Android Studio's tools
/// usually aren't on PATH even when installed).
pub fn locate(bare: &str, version_arg: &str, fallback: Option<PathBuf>) -> Option<String> {
    if is_installed(bare, version_arg) {
        return Some(bare.to_string());
    }
    let path = fallback?;
    let path_str = path.to_string_lossy().to_string();
    is_installed(&path_str, version_arg).then_some(path_str)
}

/// `adb` resolved via PATH, falling back to the SDK-discovered path.
pub fn adb_bin() -> String {
    locate("adb", "version", sdk::adb_path()).unwrap_or_else(|| "adb".to_string())
}

/// (serial, state) for every device `adb devices` lists, e.g. `("emulator-5556", "device")`.
pub fn adb_devices() -> Vec<(String, String)> {
    let (ok, out) = run(&adb_bin(), &["devices"]);
    if !ok {
        return Vec::new();
    }
    out.lines()
        .skip(1)
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}

pub fn adb_device_connected() -> bool {
    adb_devices().iter().any(|(_, state)| state == "device")
}

fn selected_device_slot() -> &'static Mutex<Option<String>> {
    static SLOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Manually pins which device adb actions target (set via the "Select ADB
/// device" menu). `None` goes back to auto-picking.
pub fn set_selected_device(serial: Option<String>) {
    *selected_device_slot().lock().unwrap() = serial;
}

pub fn selected_device() -> Option<String> {
    selected_device_slot().lock().unwrap().clone()
}

/// Picks which device `adb` commands should target: a manual selection if one
/// was made, else the first device actually online ("device"), else whatever's
/// listed (so an explicit -s still avoids adb's "more than one device" error),
/// else none.
pub fn adb_target() -> Option<String> {
    if let Some(serial) = selected_device() {
        return Some(serial);
    }
    let devices = adb_devices();
    devices
        .iter()
        .find(|(_, state)| state == "device")
        .or_else(|| devices.first())
        .map(|(serial, _)| serial.clone())
}

/// Runs `adb <args...>` targeted at the current device via -s, so it works
/// even when stale/ghost entries make a bare `adb` call ambiguous.
pub fn adb_run(args: &[&str]) -> (bool, String) {
    let bin = adb_bin();
    match adb_target() {
        Some(serial) => {
            let mut full: Vec<&str> = vec!["-s", &serial];
            full.extend_from_slice(args);
            run(&bin, &full)
        }
        None => run(&bin, args),
    }
}

/// Blocks until `serial` shows up in "device" state, or `timeout_secs` elapses.
/// `adb root`/`adb unroot` restart adbd, which disconnects and reconnects the
/// device — calling another adb command too soon can race that reconnect.
pub fn wait_for_device(serial: &str, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout_secs {
        if adb_devices().iter().any(|(s, state)| s == serial && state == "device") {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
    false
}

/// Background version of [`adb_run`] (e.g. `adb reboot`, backgrounded shell commands).
pub fn adb_spawn_bg(args: &[&str]) -> Result<(), String> {
    let bin = adb_bin();
    match adb_target() {
        Some(serial) => {
            let mut full: Vec<&str> = vec!["-s", &serial];
            full.extend_from_slice(args);
            spawn_bg(&bin, &full)
        }
        None => spawn_bg(&bin, args),
    }
}

/// Opens `url` in the default desktop browser.
pub fn open_url(url: &str) -> Result<(), String> {
    if cfg!(windows) {
        spawn_bg("cmd", &["/C", "start", "", url])
    } else if cfg!(target_os = "macos") {
        spawn_bg("open", &[url])
    } else {
        spawn_bg("xdg-open", &[url])
    }
}

/// Short, URL-safe local token — not a cryptographic secret, just enough to
/// keep mitmweb's UI from 403'ing on the auto-opened tab.
pub fn random_token() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Runs `cmd args...`, forwarding its output to `tx` live instead of only at
/// the end — needed for long downloads/installs (sdkmanager, pip, curl) so
/// the TUI doesn't sit silent for minutes and look frozen. Progress bars that
/// update in place via `\r` (not `\n`) are forwarded as lines prefixed with
/// `\r`, which the TUI's log treats as "replace the last line" instead of
/// appending — so a real percentage-style progress indicator shows up live.
pub fn run_streaming(cmd: &str, args: &[&str], tx: &Sender<String>) -> (bool, String) {
    let mut child = match build(cmd, args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return (false, format!("`{cmd}` not found: {e}")),
    };

    let captured = std::sync::Arc::new(Mutex::new(String::new()));

    let stderr_handle = child.stderr.take().map(|s| {
        let tx = tx.clone();
        let captured = std::sync::Arc::clone(&captured);
        std::thread::spawn(move || read_progress_stream(s, &tx, &captured))
    });
    if let Some(stdout) = child.stdout.take() {
        read_progress_stream(stdout, tx, &captured);
    }
    if let Some(h) = stderr_handle {
        let _ = h.join();
    }

    match child.wait() {
        Ok(status) => (status.success(), captured.lock().unwrap().trim().to_string()),
        Err(e) => (false, format!("`{cmd}` wait failed: {e}")),
    }
}

/// Reads raw bytes (not `BufRead::lines`, which only splits on `\n` and would
/// swallow `\r`-updated progress bars entirely) and forwards each completed
/// line, prefixing `\r`-terminated ones so the TUI knows to overwrite in place.
fn read_progress_stream(mut r: impl std::io::Read, tx: &Sender<String>, captured: &std::sync::Mutex<String>) {
    let mut buf = [0u8; 4096];
    let mut line: Vec<u8> = Vec::new();
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &b in &buf[..n] {
            if b == b'\n' || b == b'\r' {
                let text = String::from_utf8_lossy(&line).trim().to_string();
                if !text.is_empty() {
                    let mut c = captured.lock().unwrap();
                    c.push_str(&text);
                    c.push('\n');
                    drop(c);
                    let overwrite = b == b'\r';
                    tx.send(if overwrite { format!("\r{text}") } else { text }).ok();
                }
                line.clear();
            } else {
                line.push(b);
            }
        }
    }
}

/// Copies `text` to the OS clipboard via the platform's built-in tool
/// (`clip` on Windows, `pbcopy` on macOS, `xclip` elsewhere) — no crate needed.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;

    let (cmd, args): (&str, &[&str]) = if cfg!(windows) {
        ("clip", &[])
    } else if cfg!(target_os = "macos") {
        ("pbcopy", &[])
    } else {
        ("xclip", &["-selection", "clipboard"])
    };

    let mut child = build(cmd, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("`{cmd}` not available: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("no stdin handle")?
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())?;
    child.wait().map_err(|e| e.to_string())?;
    Ok(())
}
