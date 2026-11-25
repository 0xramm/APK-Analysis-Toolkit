use std::sync::mpsc::Sender;

use crate::util::shell;

const DOWNLOAD_PAGE: &str = "https://mt-manager.en.uptodown.com/android/download";

/// MT Manager's real download is served from a JS/AJAX-driven endpoint on
/// uptodown rather than a stable, scrapable URL — opening the page for a
/// manual download is more honest than shipping a scraper that silently
/// breaks the moment their frontend changes.
pub fn install(tx: &Sender<String>) {
    tx.send("== MT Manager ==".into()).ok();
    tx.send(format!("> opening {DOWNLOAD_PAGE} ...")).ok();
    match shell::open_url(DOWNLOAD_PAGE) {
        Ok(()) => {
            tx.send("[OK] opened in your browser.".into()).ok();
            tx.send("  Download the APK, then use 'Install APK' (App manager) with that path.".into()).ok();
        }
        Err(e) => {
            tx.send(format!("[FAIL] couldn't open a browser: {e}")).ok();
            tx.send(format!("  Download it manually from: {DOWNLOAD_PAGE}")).ok();
        }
    }
}
