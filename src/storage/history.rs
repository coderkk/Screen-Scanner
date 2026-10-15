//! Scan‑history persistence and History dialog.
//!
//! Scanned results are stored as a JSON array next to the executable.
//! The History dialog shows a scrollable list with buttons to copy, open,
//! clear, or close.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use arboard::Clipboard;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::notifications;

// ────────────────────────────────────────────────────────────
// Data types
// ────────────────────────────────────────────────────────────

/// A single entry in the scan history log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryItem {
    /// Human‑readable timestamp (UTC).
    pub time: String,
    /// Barcode symbology (e.g. `"QR_CODE"`).
    pub format: String,
    /// Decoded text content.
    pub text: String,
}

// ────────────────────────────────────────────────────────────
// Persistence
// ────────────────────────────────────────────────────────────

/// Path to the JSON history file (next to the executable).
pub fn get_history_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push("scanner_history.json");
    p
}

/// Load the history array from disk.
pub fn load_history() -> Vec<HistoryItem> {
    let path = get_history_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(hist) = serde_json::from_str(&content) {
            return hist;
        }
    }
    Vec::new()
}

/// Save the history array to disk.
pub fn save_history(hist: &[HistoryItem]) {
    let path = get_history_path();
    let _ = fs::write(&path, serde_json::to_string_pretty(hist).unwrap_or_default());
}

// ────────────────────────────────────────────────────────────
// Control IDs (private to this dialog)
// ────────────────────────────────────────────────────────────

const IDC_HIST_LIST:  usize = 4001;
const IDC_HIST_COPY:  usize = 4002;
const IDC_HIST_OPEN:  usize = 4003;
const IDC_HIST_CLEAR: usize = 4004;
const IDC_HIST_CLOSE: usize = 4005;

// ────────────────────────────────────────────────────────────
// ListBox helpers
// ────────────────────────────────────────────────────────────

/// Repopulate the LISTBOX with the current in‑memory history.
pub fn refresh_history_list(hwnd: HWND) {
    unsafe {
        let lb = GetDlgItem(hwnd, IDC_HIST_LIST as i32);
        if let Ok(lb_hwnd) = lb {
            SendMessageW(lb_hwnd, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
            if let Some(ref app) = crate::app_state::APP_STATE {
                for (i, item) in app.history.iter().enumerate() {
                    let preview = if item.text.len() > 60 {
                        format!("{}...", &item.text[..60])
                    } else {
                        item.text.clone()
                    };
                    let display_str = format!("{}. [{}] ({}) {}", i + 1, item.format, item.time, preview);
                    let display_utf16: Vec<u16> = format!("{}\0", display_str).encode_utf16().collect();
                    SendMessageW(lb_hwnd, LB_ADDSTRING, WPARAM(0), LPARAM(display_utf16.as_ptr() as _));
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────
// Window procedure
// ────────────────────────────────────────────────────────────

/// WndProc for the history dialog.
///
/// * **WM_CREATE** → builds the list‑box and action buttons.
/// * **WM_SHOWWINDOW** → refreshes the list.
/// * **WM_COMMAND** → handles Copy, Open, Clear, Close, and list double‑click.
unsafe extern "system" fn history_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let h_instance = GetModuleHandleW(None).unwrap();

            // Label
            let label_text: Vec<u16> = "Scan History (Double-click or select an item):\0".encode_utf16().collect();
            CreateWindowExW(
                Default::default(), w!("STATIC"),
                PCWSTR(label_text.as_ptr()),
                WS_CHILD | WS_VISIBLE,
                20, 15, 450, 20, hwnd, None, h_instance, None,
            ).unwrap();

            // ListBox (LBS_NOTIFY | WS_BORDER)
            CreateWindowExW(
                Default::default(), w!("LISTBOX"), w!(""),
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | WINDOW_STYLE(0x0001 | 0x0100),
                20, 40, 540, 310, hwnd,
                HMENU(IDC_HIST_LIST as _), h_instance, None,
            ).unwrap();

            // Buttons
            create_button(hwnd, h_instance, "Copy to Clipboard", IDC_HIST_COPY,  20,  365, 140);
            create_button(hwnd, h_instance, "Open in Browser",   IDC_HIST_OPEN,  170, 365, 130);
            create_button(hwnd, h_instance, "Clear All",         IDC_HIST_CLEAR, 310, 365, 100);
            create_button(hwnd, h_instance, "Close",             IDC_HIST_CLOSE, 460, 365, 100);

            LRESULT(0)
        }

        WM_SHOWWINDOW => {
            if wparam.0 != 0 {
                refresh_history_list(hwnd);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        WM_COMMAND => {
            let id          = (wparam.0 & 0xFFFF) as usize;
            let notify_code = ((wparam.0 >> 16) & 0xFFFF) as u32;

            match id {
                // Double‑click on a list item → copy to clipboard
                IDC_HIST_LIST => {
                    if notify_code == 2 {
                        copy_selected_item(hwnd);
                    }
                }
                IDC_HIST_COPY => {
                    copy_selected_item(hwnd);
                }
                IDC_HIST_OPEN => {
                    let lb = GetDlgItem(hwnd, IDC_HIST_LIST as i32).unwrap();
                    let cur_sel = SendMessageW(lb, LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    if let Some(ref app) = crate::app_state::APP_STATE {
                        if let Some(item) = app.history.get(cur_sel) {
                            if item.text.starts_with("http://") || item.text.starts_with("https://") {
                                let _ = open::that(&item.text);
                            } else {
                                notifications::show_message("Selected item is not a web URL.");
                            }
                        } else {
                            notifications::show_message("Please select an item from the history list first.");
                        }
                    }
                }
                IDC_HIST_CLEAR => {
                    if let Some(ref mut app) = crate::app_state::APP_STATE {
                        app.history.clear();
                        save_history(&app.history);
                        refresh_history_list(hwnd);
                    }
                }
                IDC_HIST_CLOSE => {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ────────────────────────────────────────────────────────────
// Private helpers
// ────────────────────────────────────────────────────────────

/// Copy the currently selected history item's text to the clipboard.
unsafe fn copy_selected_item(hwnd: HWND) {
    let lb = GetDlgItem(hwnd, IDC_HIST_LIST as i32).unwrap();
    let cur_sel = SendMessageW(lb, LB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
    if let Some(ref app) = crate::app_state::APP_STATE {
        if let Some(item) = app.history.get(cur_sel) {
            let mut cb = Clipboard::new().ok();
            if let Some(ref mut clip) = cb {
                let _ = clip.set_text(item.text.clone());
            }
            notifications::show_toast_notification("Copied to Clipboard", &item.text);
        } else {
            notifications::show_message("Please select an item from the history list first.");
        }
    }
}

/// Create a simple push‑button child control.
unsafe fn create_button(
    parent: HWND,
    h_instance: windows::Win32::Foundation::HMODULE,
    text: &str, id: usize, x: i32, y: i32, w: i32,
) {
    let text_w: Vec<u16> = format!("{}\0", text).encode_utf16().collect();
    CreateWindowExW(
        Default::default(), w!("BUTTON"),
        PCWSTR(text_w.as_ptr()),
        WS_CHILD | WS_VISIBLE,
        x, y, w, 30, parent,
        HMENU(id as _), h_instance, None,
    ).unwrap();
}

// ────────────────────────────────────────────────────────────
// Public creation helpers
// ────────────────────────────────────────────────────────────

/// Register the history‑dialog window class. Call once at startup.
pub fn register_history_class() {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("ScannerHistoryWindowClass");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(history_wndproc),
            hInstance: h_instance.into(),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hIcon: HICON(std::ptr::null_mut()),
            hbrBackground: Gdi::HBRUSH((Gdi::COLOR_WINDOW.0 + 1) as _),
            ..Default::default()
        };

        RegisterClassW(&wnd_class);
    }
}

/// Create the (initially hidden) history dialog, centred on the primary monitor.
pub fn create_history_window(icon: HICON) -> HWND {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("ScannerHistoryWindowClass");

        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let w = 595;
        let h = 450;
        let x = (sw - w) / 2;
        let y = (sh - h) / 2;

        let style = WINDOW_STYLE(
            WS_OVERLAPPEDWINDOW.0 & !WS_THICKFRAME.0 & !WS_MAXIMIZEBOX.0,
        );

        let hwnd = CreateWindowExW(
            Default::default(), class_name,
            w!("Scan History"), style,
            x, y, w, h,
            HWND(std::ptr::null_mut()), None, h_instance, None,
        ).unwrap();

        SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as _),   LPARAM(icon.0 as _));
        SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as _), LPARAM(icon.0 as _));

        hwnd
    }
}

// ────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_item_round_trips_through_json() {
        let items = vec![
            HistoryItem { time: "2026-01-01 12:00:00".into(), format: "QR_CODE".into(), text: "https://example.com".into() },
            HistoryItem { time: "2026-01-02 13:00:00".into(), format: "CODE_128".into(), text: "ABC-123".into() },
        ];
        let json = serde_json::to_string_pretty(&items).unwrap();
        let restored: Vec<HistoryItem> = serde_json::from_str(&json).unwrap();
        assert_eq!(items, restored);
    }

    #[test]
    fn empty_history_deserialises_correctly() {
        let json = "[]";
        let hist: Vec<HistoryItem> = serde_json::from_str(json).unwrap();
        assert!(hist.is_empty());
    }
}
