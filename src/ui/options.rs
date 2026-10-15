//! Options / shortcut‑configuration dialog.
//!
//! Lets the user pick modifier + key combos for the two global hotkeys,
//! toggle history recording, and choose the max‑history limit.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{self, HIST_LIMITS, KEY_OPTIONS, MOD_OPTIONS};
use crate::history;

// ────────────────────────────────────────────────────────────
// Control IDs (private to this dialog)
// ────────────────────────────────────────────────────────────

const IDC_FULL_MOD:    usize = 3001;
const IDC_FULL_KEY:    usize = 3002;
const IDC_REG_MOD:     usize = 3003;
const IDC_REG_KEY:     usize = 3004;
const IDC_CHK_HISTORY: usize = 3005;
const IDC_CB_MAX_HIST: usize = 3006;
const IDC_BTN_SAVE:    usize = 3007;
const IDC_BTN_CANCEL:  usize = 3008;

// ────────────────────────────────────────────────────────────
// Window procedure
// ────────────────────────────────────────────────────────────

/// WndProc for the options dialog window.
///
/// * **WM_CREATE** → builds all child controls (labels, combo‑boxes, buttons).
/// * **WM_SHOWWINDOW** → refreshes combo selections from the live config.
/// * **WM_COMMAND / Save** → reads controls, updates config, re‑registers hotkeys.
/// * **WM_COMMAND / Cancel** or **WM_CLOSE** → hides the dialog.
unsafe extern "system" fn options_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let h_instance = GetModuleHandleW(None).unwrap();

            // ── Fullscreen shortcut ──────────────────────
            create_label(hwnd, h_instance, "Fullscreen Scan Shortcut:", 20, 20, 250);
            create_combo(hwnd, h_instance, IDC_FULL_MOD, 20, 45, 150, &MOD_OPTIONS.iter().map(|m| m.0).collect::<Vec<_>>());
            create_combo(hwnd, h_instance, IDC_FULL_KEY, 185, 45, 115, &KEY_OPTIONS.iter().map(|k| k.0).collect::<Vec<_>>());

            // ── Region shortcut ──────────────────────────
            create_label(hwnd, h_instance, "Region Scan Shortcut:", 20, 95, 250);
            create_combo(hwnd, h_instance, IDC_REG_MOD, 20, 120, 150, &MOD_OPTIONS.iter().map(|m| m.0).collect::<Vec<_>>());
            create_combo(hwnd, h_instance, IDC_REG_KEY, 185, 120, 115, &KEY_OPTIONS.iter().map(|k| k.0).collect::<Vec<_>>());

            // ── History checkbox ──────────────────────────
            CreateWindowExW(
                Default::default(), w!("BUTTON"),
                w!("Record scanned items in history"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x0003), // BS_AUTOCHECKBOX
                20, 170, 280, 24, hwnd,
                HMENU(IDC_CHK_HISTORY as _), h_instance, None,
            ).unwrap();

            // ── Max history combo ────────────────────────
            create_label(hwnd, h_instance, "Max history records:", 20, 205, 150);
            let limits: Vec<String> = HIST_LIMITS.iter().map(|l| format!("{} records", l)).collect();
            let limit_strs: Vec<&str> = limits.iter().map(|s| s.as_str()).collect();
            create_combo(hwnd, h_instance, IDC_CB_MAX_HIST, 185, 202, 115, &limit_strs);

            // ── Save / Cancel buttons ────────────────────
            CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Save"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x0001), // BS_DEFPUSHBUTTON
                120, 255, 85, 28, hwnd,
                HMENU(IDC_BTN_SAVE as _), h_instance, None,
            ).unwrap();

            CreateWindowExW(
                Default::default(), w!("BUTTON"), w!("Cancel"),
                WS_CHILD | WS_VISIBLE,
                215, 255, 85, 28, hwnd,
                HMENU(IDC_BTN_CANCEL as _), h_instance, None,
            ).unwrap();

            LRESULT(0)
        }

        // ── Refresh controls when dialog is shown ────────
        WM_SHOWWINDOW => {
            if wparam.0 != 0 {
                let cfm = GetDlgItem(hwnd, IDC_FULL_MOD as i32);
                let cfk = GetDlgItem(hwnd, IDC_FULL_KEY as i32);
                let crm = GetDlgItem(hwnd, IDC_REG_MOD as i32);
                let crk = GetDlgItem(hwnd, IDC_REG_KEY as i32);
                let ch  = GetDlgItem(hwnd, IDC_CHK_HISTORY as i32);
                let cmh = GetDlgItem(hwnd, IDC_CB_MAX_HIST as i32);

                if let (Ok(cfm), Ok(cfk), Ok(crm), Ok(crk), Ok(ch), Ok(cmh)) = (cfm, cfk, crm, crk, ch, cmh) {
                    if let Some(ref app) = crate::app_state::APP_STATE {
                        SendMessageW(cfm, CB_SETCURSEL, WPARAM(app.config.fullscreen_hotkey.mod_index), LPARAM(0));
                        SendMessageW(cfk, CB_SETCURSEL, WPARAM(app.config.fullscreen_hotkey.key_index), LPARAM(0));
                        SendMessageW(crm, CB_SETCURSEL, WPARAM(app.config.region_hotkey.mod_index), LPARAM(0));
                        SendMessageW(crk, CB_SETCURSEL, WPARAM(app.config.region_hotkey.key_index), LPARAM(0));

                        let check_state = if app.config.record_history { 1 } else { 0 };
                        SendMessageW(ch, BM_SETCHECK, WPARAM(check_state), LPARAM(0));

                        let limit_idx = HIST_LIMITS.iter().position(|&l| l == app.config.max_history).unwrap_or(2);
                        SendMessageW(cmh, CB_SETCURSEL, WPARAM(limit_idx), LPARAM(0));
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        // ── Button commands ──────────────────────────────
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as usize;
            match id {
                IDC_BTN_SAVE => {
                    let cb_full_mod = GetDlgItem(hwnd, IDC_FULL_MOD as i32).unwrap();
                    let cb_full_key = GetDlgItem(hwnd, IDC_FULL_KEY as i32).unwrap();
                    let cb_reg_mod  = GetDlgItem(hwnd, IDC_REG_MOD as i32).unwrap();
                    let cb_reg_key  = GetDlgItem(hwnd, IDC_REG_KEY as i32).unwrap();
                    let chk_hist    = GetDlgItem(hwnd, IDC_CHK_HISTORY as i32).unwrap();
                    let cb_max_hist = GetDlgItem(hwnd, IDC_CB_MAX_HIST as i32).unwrap();

                    let full_mod_idx = SendMessageW(cb_full_mod, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    let full_key_idx = SendMessageW(cb_full_key, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    let reg_mod_idx  = SendMessageW(cb_reg_mod,  CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    let reg_key_idx  = SendMessageW(cb_reg_key,  CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    let record_hist  = SendMessageW(chk_hist,    BM_GETCHECK,  WPARAM(0), LPARAM(0)).0 == 1;
                    let max_hist_idx = SendMessageW(cb_max_hist, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                    let max_hist     = *HIST_LIMITS.get(max_hist_idx).unwrap_or(&50);

                    if let Some(ref mut app) = crate::app_state::APP_STATE {
                        app.config.fullscreen_hotkey.mod_index = full_mod_idx;
                        app.config.fullscreen_hotkey.key_index = full_key_idx;
                        app.config.region_hotkey.mod_index     = reg_mod_idx;
                        app.config.region_hotkey.key_index     = reg_key_idx;
                        app.config.record_history              = record_hist;
                        app.config.max_history                 = max_hist;

                        config::save_config(&app.config);

                        if app.history.len() > max_hist {
                            app.history.truncate(max_hist);
                            history::save_history(&app.history);
                            history::refresh_history_list(app.history_hwnd);
                        }

                        if !app.main_hwnd.0.is_null() {
                            crate::tray::register_hotkeys(app.main_hwnd, &app.config);
                        }
                    }

                    let _ = ShowWindow(hwnd, SW_HIDE);
                    let ok_title: Vec<u16> = "Options Saved\0".encode_utf16().collect();
                    let ok_msg: Vec<u16>   = "Options updated successfully!\0".encode_utf16().collect();
                    MessageBoxW(hwnd, PCWSTR(ok_msg.as_ptr()), PCWSTR(ok_title.as_ptr()), MB_OK | MB_ICONINFORMATION);
                }
                IDC_BTN_CANCEL => {
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
// Private helpers for building child controls
// ────────────────────────────────────────────────────────────

/// Create a `STATIC` label.
unsafe fn create_label(
    parent: HWND,
    h_instance: windows::Win32::Foundation::HMODULE,
    text: &str, x: i32, y: i32, w: i32,
) {
    let text_w: Vec<u16> = format!("{}\0", text).encode_utf16().collect();
    CreateWindowExW(
        Default::default(), w!("STATIC"),
        PCWSTR(text_w.as_ptr()),
        WS_CHILD | WS_VISIBLE,
        x, y, w, 20, parent, None, h_instance, None,
    ).unwrap();
}

/// Create a `COMBOBOX` (CBS_DROPDOWNLIST) and populate it.
unsafe fn create_combo(
    parent: HWND,
    h_instance: windows::Win32::Foundation::HMODULE,
    id: usize, x: i32, y: i32, w: i32,
    items: &[&str],
) {
    let cb = CreateWindowExW(
        Default::default(), w!("COMBOBOX"), w!(""),
        WS_CHILD | WS_VISIBLE | WS_VSCROLL | WINDOW_STYLE(0x0003), // CBS_DROPDOWNLIST
        x, y, w, 350, parent,
        HMENU(id as _), h_instance, None,
    ).unwrap();

    for name in items {
        let name_utf16: Vec<u16> = format!("{}\0", name).encode_utf16().collect();
        SendMessageW(cb, CB_ADDSTRING, WPARAM(0), LPARAM(name_utf16.as_ptr() as _));
    }
}

// ────────────────────────────────────────────────────────────
// Public creation helpers
// ────────────────────────────────────────────────────────────

/// Register the options‑dialog window class. Call once at startup.
pub fn register_options_class() {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("ScannerOptionsWindowClass");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(options_wndproc),
            hInstance: h_instance.into(),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
            hbrBackground: Gdi::HBRUSH((Gdi::COLOR_WINDOW.0 + 1) as _),
            ..Default::default()
        };

        RegisterClassW(&wnd_class);
    }
}

/// Create the (initially hidden) options dialog, centred on the primary monitor.
pub fn create_options_window(icon: HICON) -> HWND {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("ScannerOptionsWindowClass");

        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let w = 340;
        let h = 340;
        let x = (sw - w) / 2;
        let y = (sh - h) / 2;

        let style = WINDOW_STYLE(
            WS_OVERLAPPEDWINDOW.0 & !WS_THICKFRAME.0 & !WS_MAXIMIZEBOX.0,
        );

        let hwnd = CreateWindowExW(
            Default::default(), class_name,
            w!("Options & Shortcuts"), style,
            x, y, w, h,
            HWND(std::ptr::null_mut()), None, h_instance, None,
        ).unwrap();

        SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as _),   LPARAM(icon.0 as _));
        SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as _), LPARAM(icon.0 as _));

        hwnd
    }
}
