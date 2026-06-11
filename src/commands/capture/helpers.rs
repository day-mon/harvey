//! Utility helpers.

use std::path::PathBuf;
use std::sync::Arc;

use chromiumoxide::cdp::browser_protocol::network::{
    GetResponseBodyParams, RequestId,
};
use chromiumoxide::Page as ChromiumPage;

pub(super) async fn fetch_body_inner(
    page: &Arc<ChromiumPage>,
    request_id: &RequestId,
) -> Option<String> {
    let params = GetResponseBodyParams::new(request_id.clone());
    match page.activate().await.ok()?.execute(params).await {
        Ok(result) => {
            if result.base64_encoded {
                Some(format!("[base64: {} bytes]", result.body.len()))
            } else {
                Some(result.body.clone())
            }
        }
        Err(_) => None,
    }
}

/// Auto-discover a Chrome/Chromium/Helium user data directory.
///
/// Checks known locations on macOS and Linux in order of preference.
pub(super) fn discover_chrome_user_data() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let home = PathBuf::from(home);

    let candidates: &[&str] = {
        #[cfg(target_os = "macos")]
        {
            &[
                "Library/Application Support/Google/Chrome",
                "Library/Application Support/Chromium",
                "Library/Application Support/net.imput.helium",
            ]
        }
        #[cfg(target_os = "linux")]
        {
            &[
                ".config/google-chrome",
                ".config/chromium",
                ".config/helium",
            ]
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            &[]
        }
    };

    for path in candidates {
        let full = home.join(path);
        if full.exists() {
            return Some(full);
        }
    }
    None
}
