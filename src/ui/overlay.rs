//! Snipping / region‑selection overlay window.
//!
//! When the user triggers a region scan the overlay is displayed full‑screen
//! with a semi‑transparent dark tint. The user drags a rectangle; on mouse‑up
//! the selected area is cropped from the captured screenshot and decoded.

use image::DynamicImage;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, EndPaint, FillRect,
    GetStockObject, InvalidateRect, Rectangle, SelectObject, SetBkMode,
    SetTextColor, TextOutW, NULL_BRUSH, PAINTSTRUCT, PS_SOLID, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::notifications;
use crate::scanner::{decode_barcode, ScanResult};

// ────────────────────────────────────────────────────────────
// Overlay state (lives only while the overlay is visible)
// ────────────────────────────────────────────────────────────

/// Transient state for a single region‑selection session.
pub struct SnippingState {
    pub is_selecting: bool,
    pub start_pt: POINT,
    pub current_pt: POINT,
    /// Virtual‑screen origin X.
    pub vx: i32,
    /// Virtual‑screen origin Y.
    pub vy: i32,
    /// Virtual‑screen width.
    pub vw: i32,
    /// Virtual‑screen height.
    pub vh: i32,
    /// The full screenshot taken just before the overlay appeared.
    pub captured_screen: Option<DynamicImage>,
}

/// Global mutable overlay state; `Some` only while the overlay is active.
pub static mut SNIP_STATE: Option<SnippingState> = None;

// ────────────────────────────────────────────────────────────
// Window procedure
// ────────────────────────────────────────────────────────────

/// WndProc for the snipping overlay window.
///
/// * **ESC** → cancel and hide.
/// * **LButtonDown** → begin selection.
/// * **MouseMove** → update selection rectangle.
/// * **LButtonUp** → finalise selection, crop, decode, and present result.
/// * **WM_PAINT** → draw the dark tint, guide text, and selection rectangle.
pub(crate) unsafe extern "system" fn snip_wndproc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        // ── Keyboard ──────────────────────────────────────
        WM_KEYDOWN => {
            if wparam.0 == VK_ESCAPE.0 as usize {
                let _ = ShowWindow(hwnd, SW_HIDE);
                SNIP_STATE = None;
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        // ── Mouse down → start selection ──────────────────
        WM_LBUTTONDOWN => {
            SetCapture(hwnd);
            let mut pt = POINT::default();
            GetCursorPos(&mut pt).unwrap_or_default();
            if let Some(ref mut state) = SNIP_STATE {
                state.is_selecting = true;
                state.start_pt = pt;
                state.current_pt = pt;
            }
            let _ = InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }

        // ── Mouse move → update rubber‑band ───────────────
        WM_MOUSEMOVE => {
            if let Some(ref mut state) = SNIP_STATE {
                if state.is_selecting {
                    let mut pt = POINT::default();
                    GetCursorPos(&mut pt).unwrap_or_default();
                    state.current_pt = pt;
                    let _ = InvalidateRect(hwnd, None, false);
                }
            }
            LRESULT(0)
        }

        // ── Mouse up → crop & decode ─────────────────────
        WM_LBUTTONUP => {
            ReleaseCapture().unwrap_or_default();
            let mut captured_result: Option<ScanResult> = None;

            if let Some(ref mut state) = SNIP_STATE {
                state.is_selecting = false;
                let mut pt = POINT::default();
                GetCursorPos(&mut pt).unwrap_or_default();
                state.current_pt = pt;

                let min_x = state.start_pt.x.min(state.current_pt.x);
                let max_x = state.start_pt.x.max(state.current_pt.x);
                let min_y = state.start_pt.y.min(state.current_pt.y);
                let max_y = state.start_pt.y.max(state.current_pt.y);

                let sel_w = (max_x - min_x) as u32;
                let sel_h = (max_y - min_y) as u32;

                if sel_w > 8 && sel_h > 8 {
                    if let Some(ref full_img) = state.captured_screen {
                        let rel_x = (min_x - state.vx).max(0) as u32;
                        let rel_y = (min_y - state.vy).max(0) as u32;

                        let crop_w = sel_w.min(full_img.width().saturating_sub(rel_x));
                        let crop_h = sel_h.min(full_img.height().saturating_sub(rel_y));

                        if crop_w > 8 && crop_h > 8 {
                            let cropped = full_img.crop_imm(rel_x, rel_y, crop_w, crop_h);
                            captured_result = decode_barcode(&cropped);
                        }
                    }
                }
            }

            let _ = ShowWindow(hwnd, SW_HIDE);
            SNIP_STATE = None;

            if let Some(res) = captured_result {
                notifications::handle_scan_result(&res);
            } else {
                notifications::show_message("No barcode or QR code found in selected region.");
            }

            LRESULT(0)
        }

        // ── Paint ─────────────────────────────────────────
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            if let Some(ref state) = SNIP_STATE {
                // Dark tint
                let rc_full = RECT { left: 0, top: 0, right: state.vw, bottom: state.vh };
                let dark_brush = CreateSolidBrush(COLORREF(0x00000000));
                FillRect(hdc, &rc_full, dark_brush);
                let _ = DeleteObject(dark_brush);

                // Guide text
                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0x00FFFFFF));
                let guide_text: Vec<u16> =
                    "Drag to select a barcode/QR code region | ESC to cancel\0"
                        .encode_utf16()
                        .collect();
                let _ = TextOutW(hdc, (state.vw / 2) - 180, 20, &guide_text[..guide_text.len() - 1]);

                // Selection rectangle
                if state.is_selecting {
                    let min_x = state.start_pt.x.min(state.current_pt.x) - state.vx;
                    let max_x = state.start_pt.x.max(state.current_pt.x) - state.vx;
                    let min_y = state.start_pt.y.min(state.current_pt.y) - state.vy;
                    let max_y = state.start_pt.y.max(state.current_pt.y) - state.vy;

                    let pen = CreatePen(PS_SOLID, 2, COLORREF(0x00FFFF00));
                    let old_pen   = SelectObject(hdc, pen);
                    let null_brush = GetStockObject(NULL_BRUSH);
                    let old_brush  = SelectObject(hdc, null_brush);

                    let _ = Rectangle(hdc, min_x, min_y, max_x, max_y);

                    SelectObject(hdc, old_pen);
                    SelectObject(hdc, old_brush);
                    let _ = DeleteObject(pen);
                }
            }

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ────────────────────────────────────────────────────────────
// Window creation helpers
// ────────────────────────────────────────────────────────────

/// Register the overlay window class. Must be called once before
/// [`create_overlay_window`].
pub fn register_overlay_class() {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("BarcodeSnipOverlayClass");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(snip_wndproc),
            hInstance: h_instance.into(),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_CROSS).unwrap(),
            ..Default::default()
        };

        RegisterClassW(&wnd_class);
    }
}

/// Create the (initially hidden) overlay window that spans the entire virtual desktop.
pub fn create_overlay_window() -> HWND {
    unsafe {
        let h_instance = GetModuleHandleW(None).unwrap();
        let class_name = w!("BarcodeSnipOverlayClass");

        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            class_name,
            w!("Barcode Snip Overlay"),
            WS_POPUP,
            vx, vy, vw, vh,
            HWND(std::ptr::null_mut()),
            None,
            h_instance,
            None,
        ).unwrap();

        SetLayeredWindowAttributes(hwnd, COLORREF(0), 140, LWA_ALPHA).unwrap();
        hwnd
    }
}
