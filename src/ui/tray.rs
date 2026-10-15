//! System‑tray icon, context menu, and global hot‑key management.
//!
//! The hidden main window receives:
//! * `WM_HOTKEY` – triggers full‑screen or region scans.
//! * `WM_TRAYICON` (custom message) – shows the right‑click context menu.
//! * `WM_COMMAND` – dispatches menu‑item clicks.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::{AppConfig, KEY_OPTIONS, MOD_OPTIONS};
use crate::notifications;
use crate::scanner;

// ────────────────────────────────────────────────────────────
// Constants
// ────────────────────────────────────────────────────────────

/// Hot‑key ID for the fullscreen‑scan shortcut.
const HOTKEY_FULLSCREEN: i32 = 1;
/// Hot‑key ID for the region‑scan shortcut.
const HOTKEY_REGION: i32 = 2;

/// Custom window message used by the tray icon callback.
const WM_TRAYICON: u32 = WM_USER + 101;

// Context‑menu command IDs
const IDM_SCAN_FULLSCREEN: usize = 2001;
const IDM_SCAN_REGION:     usize = 2002;
const IDM_HISTORY:         usize = 2003;
const IDM_OPTIONS:         usize = 2004;
const IDM_ABOUT:           usize = 2005;
const IDM_EXIT:            usize = 2006;

// ────────────────────────────────────────────────────────────
// Hot‑key registration
// ────────────────────────────────────────────────────────────

/// (Re‑)register the two global hotkeys according to the current config.
///
/// Any previously registered hotkeys are unregistered first, so this
/// function is safe to call repeatedly (e.g. after the user changes
/// shortcuts in the Options dialog).
pub fn register_hotkeys(hwnd: HWND, cfg: &AppConfig) {
    unsafe {
        let _ = UnregisterHotKey(hwnd, HOTKEY_FULLSCREEN);
        let _ = UnregisterHotKey(hwnd, HOTKEY_REGION);

        let full_mod = MOD_OPTIONS.get(cfg.fullscreen_hotkey.mod_index).map(|m| m.1)
            .unwrap_or(MOD_CONTROL.0 | MOD_ALT.0 | MOD_NOREPEAT.0);
        let full_vk = KEY_OPTIONS.get(cfg.fullscreen_hotkey.key_index).map(|k| k.1)
            .unwrap_or(0x53);

        let reg_mod = MOD_OPTIONS.get(cfg.region_hotkey.mod_index).map(|m| m.1)
            .unwrap_or(MOD_CONTROL.0 | MOD_ALT.0 | MOD_NOREPEAT.0);
        let reg_vk = KEY_OPTIONS.get(cfg.region_hotkey.key_index).map(|k| k.1)
            .unwrap_or(0x52);

        let hk1 = RegisterHotKey(hwnd, HOTKEY_FULLSCREEN, HOT_KEY_MODIFIERS(full_mod), full_vk);
        let hk2 = RegisterHotKey(hwnd, HOTKEY_REGION,     HOT_KEY_MODIFIERS(reg_mod),  reg_vk);

        if hk1.is_err() || hk2.is_err() {
            eprintln!("Warning: One or both hotkeys could not be registered (may be in use by another app).");
        }
    }
}

// ────────────────────────────────────────────────────────────
// Tray window procedure
// ────────────────────────────────────────────────────────────

/// WndProc for the hidden main / tray window.
///
/// Handles:
/// * `WM_HOTKEY` → scan triggers.
/// * `WM_TRAYICON` + right‑click → context menu.
/// * `WM_COMMAND` → menu item dispatch.
/// * `WM_DESTROY` → clean shutdown.
pub(crate) unsafe extern "system" fn tray_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        // ── Global hotkeys ───────────────────────────────
        WM_HOTKEY => {
            match wparam.0 as i32 {
                HOTKEY_FULLSCREEN => {
                    scanner::trigger_fullscreen_scan();
                }
                HOTKEY_REGION => {
                    if let Some(ref app) = crate::app_state::APP_STATE {
                        scanner::trigger_region_snip(app.overlay_hwnd);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }

        // ── Tray icon right‑click → context menu ────────
        WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            if event == WM_RBUTTONUP {
                let mut pt = POINT::default();
                GetCursorPos(&mut pt).unwrap_or_default();

                // Build human‑readable hotkey labels for the menu
                let (full_hk, reg_hk) = if let Some(ref app) = crate::app_state::APP_STATE {
                    (app.config.format_fullscreen_hotkey(), app.config.format_region_hotkey())
                } else {
                    ("Ctrl + Alt + S".to_string(), "Ctrl + Alt + R".to_string())
                };

                let hmenu = CreatePopupMenu().unwrap();
                let full_text:  Vec<u16> = format!("Scan Whole Screen\t{}\0",  full_hk).encode_utf16().collect();
                let reg_text:   Vec<u16> = format!("Select Region to Scan\t{}\0", reg_hk).encode_utf16().collect();
                let hist_text:  Vec<u16> = "Scan History...\0".encode_utf16().collect();
                let opt_text:   Vec<u16> = "Options (Configure Shortcuts & History)...\0".encode_utf16().collect();
                let about_text: Vec<u16> = "About Screen Scanner...\0".encode_utf16().collect();
                let exit_text:  Vec<u16> = "Exit\0".encode_utf16().collect();

                let _ = AppendMenuW(hmenu, MF_STRING,    IDM_SCAN_FULLSCREEN, PCWSTR(full_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_STRING,    IDM_SCAN_REGION,     PCWSTR(reg_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0,                   PCWSTR::null());
                let _ = AppendMenuW(hmenu, MF_STRING,    IDM_HISTORY,         PCWSTR(hist_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_STRING,    IDM_OPTIONS,         PCWSTR(opt_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_STRING,    IDM_ABOUT,           PCWSTR(about_text.as_ptr()));
                let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0,                   PCWSTR::null());
                let _ = AppendMenuW(hmenu, MF_STRING,    IDM_EXIT,            PCWSTR(exit_text.as_ptr()));

                let _ = SetForegroundWindow(hwnd);
                let _ = TrackPopupMenu(hmenu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, 0, hwnd, None);
                let _ = DestroyMenu(hmenu);
            }
            LRESULT(0)
        }

        // ── Menu command dispatch ────────────────────────
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as usize;
            match cmd_id {
                IDM_SCAN_FULLSCREEN => {
                    scanner::trigger_fullscreen_scan();
                }
                IDM_SCAN_REGION => {
                    if let Some(ref app) = crate::app_state::APP_STATE {
                        scanner::trigger_region_snip(app.overlay_hwnd);
                    }
                }
                IDM_HISTORY => {
                    if let Some(ref app) = crate::app_state::APP_STATE {
                        let _ = ShowWindow(app.history_hwnd, SW_SHOW);
                        let _ = SetForegroundWindow(app.history_hwnd);
                    }
                }
                IDM_OPTIONS => {
                    if let Some(ref app) = crate::app_state::APP_STATE {
                        let _ = ShowWindow(app.options_hwnd, SW_SHOW);
                        let _ = SetForegroundWindow(app.options_hwnd);
                    }
                }
                IDM_ABOUT => {
                    notifications::show_about_dialog(hwnd);
                }
                IDM_EXIT => {
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }

        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ────────────────────────────────────────────────────────────
// Public creation helpers
// ────────────────────────────────────────────────────────────

/// Register the tray‑window class. Call once at startup.
pub fn register_tray_class() {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("ScreenBarcodeScannerTrayClass");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(tray_wndproc),
            hInstance: h_instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };

        RegisterClassW(&wnd_class);
    }
}

/// Create the hidden main window that receives hot‑key and tray messages.
pub fn create_main_window() -> HWND {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("ScreenBarcodeScannerTrayClass");

        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("Screen Barcode Scanner"),
            Default::default(),
            0, 0, 0, 0,
            HWND(std::ptr::null_mut()),
            None,
            h_instance,
            None,
        ).unwrap();

        let _ = ShowWindow(hwnd, SW_HIDE);
        hwnd
    }
}

/// Register a `NOTIFYICONDATAW` with the system‑tray shell.
pub fn create_tray_icon(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1001,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };

        let tip = "Screen Barcode & QR Scanner\0";
        for (i, c) in tip.encode_utf16().enumerate() {
            if i < nid.szTip.len() {
                nid.szTip[i] = c;
            }
        }

        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        nid
    }
}
