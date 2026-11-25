use std::sync::mpsc::Sender;

use super::{emulator, mitm, ssl_pin};
use crate::util::shell;

/// End-to-end: boot (or create+boot) an AVD, wait for it, install SSL-pinning
/// tooling and push frida-server, then stand up mitmweb and trust its CA.
/// Stops at the first failed step instead of continuing past it.
pub fn run(tx: &Sender<String>) {
    tx.send("== Full setup: emulator, SSL tooling, mitmproxy sandbox ==".into()).ok();

    emulator::detect(tx);

    if shell::adb_device_connected() {
        tx.send("[OK] device already online — skipping boot.".into()).ok();
    } else {
        if emulator::needs_rootable_avd() {
            tx.send(
                "No rootable AVD found (either none exist, or the only ones are Play Store \
                 images) — the cert-install and frida steps both need root, so creating one."
                    .into(),
            )
            .ok();
            if !emulator::create_avd(tx) {
                tx.send("Stopped: couldn't create an AVD. Fix the error above, then re-run.".into()).ok();
                return;
            }
        }
        if !emulator::launch_avd(tx) {
            tx.send("Stopped: couldn't launch the emulator. Fix the error above, then re-run.".into())
                .ok();
            return;
        }
        if !emulator::wait_for_boot(tx, 150) {
            tx.send("Stopped: emulator didn't finish booting. Fix the issue, then re-run.".into()).ok();
            return;
        }
    }

    if !ssl_pin::install_tools(tx) {
        tx.send("Stopped: frida-tools/objection install failed. Fix the error above, then re-run.".into())
            .ok();
        return;
    }
    if !ssl_pin::push_frida_server(tx) {
        tx.send("Stopped: couldn't push/start frida-server. Fix the error above, then re-run.".into())
            .ok();
        return;
    }

    if !mitm::install(tx) {
        tx.send("Stopped: mitmproxy install failed. Fix the error above, then re-run.".into()).ok();
        return;
    }
    if !mitm::start_and_set_proxy(tx) {
        tx.send("Stopped: couldn't start mitmweb. Fix the error above, then re-run.".into()).ok();
        return;
    }
    if !mitm::install_cert(tx) {
        tx.send("Stopped: CA cert install failed. Fix the error above, then re-run.".into()).ok();
        return;
    }

    tx.send(String::new()).ok();
    tx.send("[OK] Sandbox ready: emulator booted, frida-server running, traffic".into()).ok();
    tx.send("routed through mitmweb with its CA trusted at the system level.".into()).ok();
    tx.send("Use 'SSL pinning bypass -> Bypass pinning for a package' per target app.".into()).ok();
}
