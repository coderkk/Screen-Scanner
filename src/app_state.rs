//! Global application state and the main run‑loop.
//!
//! The single‑threaded nature of the Win32 message pump makes a global
//! `static mut` safe in practice. All UI callbacks access it through
//! `APP_STATE`.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, HICON, MSG, TranslateMessage,
};

use crate::config::{self, AppConfig};
use crate::history::{self, HistoryItem};
use crate::icons;
use crate::options;
use crate::overlay;
use crate::tray;

// ────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────

/// Centralised mutable state for the running application.
pub struct AppState {
    /// Hidden window that owns the hot‑key registrations and tray icon.
    pub main_hwnd: HWND,
    /// Full‑screen overlay used for region selection.
    pub overlay_hwnd: HWND,
    /// Options / shortcuts configuration dialog.
    pub options_hwnd: HWND,
    /// Scan‑history dialog.
    pub history_hwnd: HWND,
    /// Current (possibly user‑modified) configuration.
    pub config: AppConfig,
    /// In‑memory scan history (most recent first).
    pub history: Vec<HistoryItem>,
    /// The dynamically‑generated application icon.
    pub app_icon: HICON,
}

/// Global mutable state – `Some` once `run_app` has completed initialisation.
pub static mut APP_STATE: Option<AppState> = None;

// ────────────────────────────────────────────────────────────
// Entry‑point
// ────────────────────────────────────────────────────────────

/// Initialise the UI, load configuration, and enter the Windows message loop.
///
/// This is the true "main" function called from the binary crate's `fn main()`.
pub fn run_app() {
    unsafe {
        // 1. Register all window classes
        tray::register_tray_class();
        overlay::register_overlay_class();
        options::register_options_class();
        history::register_history_class();

        // 2. Create the hidden main window (receives WM_HOTKEY + tray messages)
        let main_hwnd = tray::create_main_window();

        // 3. Load persisted configuration & register hotkeys
        let cfg = config::load_config();
        tray::register_hotkeys(main_hwnd, &cfg);

        // 4. Build UI resources
        let icon         = icons::create_qr_scanner_icon();
        let overlay_hwnd = overlay::create_overlay_window();
        let options_hwnd = options::create_options_window(icon);
        let history_hwnd = history::create_history_window(icon);
        let _nid         = tray::create_tray_icon(main_hwnd, icon);

        // 5. Load history and store global state
        let hist = history::load_history();

        APP_STATE = Some(AppState {
            main_hwnd,
            overlay_hwnd,
            options_hwnd,
            history_hwnd,
            config: cfg,
            history: hist,
            app_icon: icon,
        });

        // 6. Standard Windows message pump
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
