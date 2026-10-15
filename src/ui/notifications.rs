//! Notifications, toast messages, and scan‑result presentation.
//!
//! After a barcode / QR code is decoded the result is:
//! 1. Copied to the clipboard automatically.
//! 2. Appended to the history (if enabled).
//! 3. Shown as a Windows toast notification.
//! 4. Presented in a message‑box with an "Open in Browser" option for URLs.

use arboard::Clipboard;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_MODIFY, NOTIFYICONDATAW, NIF_INFO, NIIF_INFO,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNOCANCEL, MessageBoxW,
};


use crate::history::{self, HistoryItem};
use crate::scanner::ScanResult;
use crate::utils::chrono_now_str;

// ────────────────────────────────────────────────────────────
// Simple message box
// ────────────────────────────────────────────────────────────

/// Show a modal warning / informational message box.
pub fn show_message(msg: &str) {
    let title: Vec<u16> = "Screen Barcode Scanner\0".encode_utf16().collect();
    let text_utf16: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            HWND(std::ptr::null_mut()),
            PCWSTR(text_utf16.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONWARNING,
        );
    }
}

// ────────────────────────────────────────────────────────────
// Windows toast notification (via Shell_NotifyIconW balloon)
// ────────────────────────────────────────────────────────────

/// Push a toast / balloon notification via the tray icon.
///
/// Requires the tray icon to already be registered (the `hWnd` and `uID`
/// must match the ones used in [`crate::tray::create_tray_icon`]).
pub fn show_toast_notification(title_text: &str, msg_text: &str) {
    unsafe {
        if let Some(ref app) = crate::app_state::APP_STATE {
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: app.main_hwnd,
                uID: 1001,
                uFlags: NIF_INFO,
                dwInfoFlags: NIIF_INFO,
                ..Default::default()
            };

            for (i, c) in title_text.encode_utf16().enumerate() {
                if i < nid.szInfoTitle.len() - 1 {
                    nid.szInfoTitle[i] = c;
                }
            }

            for (i, c) in msg_text.encode_utf16().enumerate() {
                if i < nid.szInfo.len() - 1 {
                    nid.szInfo[i] = c;
                }
            }

            let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
        }
    }
}

// ────────────────────────────────────────────────────────────
// Full scan‑result handler
// ────────────────────────────────────────────────────────────

/// Process a successful scan result:
///
/// 1. Copy the decoded text to the system clipboard.
/// 2. Append an entry to the scan history (if recording is enabled).
/// 3. Fire a Windows toast / balloon notification.
/// 4. Show an action dialog (with "Open in Browser" for URLs).
pub fn handle_scan_result(result: &ScanResult) {
    // 1. Clipboard
    let mut cb = Clipboard::new().ok();
    if let Some(ref mut clip) = cb {
        let _ = clip.set_text(result.text.clone());
    }

    // 2. History
    unsafe {
        if let Some(ref mut app) = crate::app_state::APP_STATE {
            if app.config.record_history {
                let now = chrono_now_str();
                let item = HistoryItem {
                    time: now,
                    format: format!("{:?}", result.format),
                    text: result.text.clone(),
                };

                app.history.insert(0, item);
                if app.history.len() > app.config.max_history {
                    app.history.truncate(app.config.max_history);
                }
                history::save_history(&app.history);
                history::refresh_history_list(app.history_hwnd);
            }
        }
    }

    // 3. Toast notification
    let toast_title = format!("Scanned [{:?}] - Copied to Clipboard!", result.format);
    let toast_preview = if result.text.len() > 150 {
        format!("{}...", &result.text[..150])
    } else {
        result.text.clone()
    };
    show_toast_notification(&toast_title, &toast_preview);

    // 4. Action dialog
    let is_url = result.text.starts_with("http://") || result.text.starts_with("https://");
    let prompt = if is_url {
        format!(
            "Detected [{:?}]:\n\n{}\n\n✓ Copied to clipboard!\n\n- Click [Yes] to open URL in browser\n- Click [Cancel] to dismiss",
            result.format, result.text
        )
    } else {
        format!(
            "Detected [{:?}]:\n\n{}\n\n✓ Copied to clipboard!",
            result.format, result.text
        )
    };

    let title: Vec<u16> = "Screen Barcode Scanner\0".encode_utf16().collect();
    let text_utf16: Vec<u16> = prompt.encode_utf16().chain(std::iter::once(0)).collect();

    let buttons = if is_url {
        MB_YESNOCANCEL | MB_ICONINFORMATION
    } else {
        MB_OK | MB_ICONINFORMATION
    };

    let response = unsafe {
        MessageBoxW(
            HWND(std::ptr::null_mut()),
            PCWSTR(text_utf16.as_ptr()),
            PCWSTR(title.as_ptr()),
            buttons,
        )
    };

    // IDYES == 6
    if response.0 == 6 && is_url {
        let _ = open::that(&result.text);
    }
}

// ────────────────────────────────────────────────────────────
// About dialog
// ────────────────────────────────────────────────────────────

/// Show the "About" message box with version number and credits.
pub fn show_about_dialog(hwnd: HWND) {
    let title: Vec<u16> = "About Screen Barcode & QR Scanner\0".encode_utf16().collect();
    let version = env!("CARGO_PKG_VERSION");
    let about_text = format!(
        "Screen Barcode & QR Scanner\nVersion {}\n\n\
         Pure Rust on-screen barcode and QR code reader.\n\
         Supports QR Code, Data Matrix, Aztec, PDF417,\n\
         UPC/EAN, Code 128, and more.\n\n\
         Author: Antigravity\n\
         License: MIT\0",
        version
    );
    let text_utf16: Vec<u16> = about_text.encode_utf16().collect();
    unsafe {
        MessageBoxW(
            hwnd,
            PCWSTR(text_utf16.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
