//! Programmatic icon generation.
//!
//! Creates a modern QR‑scanner icon at runtime using Win32 GDI.
//! The design features a deep‑navy background, cyan targeting brackets,
//! white QR‑finder blocks, and a bright‑red laser scan line.

use windows::Win32::Foundation::{BOOL, COLORREF, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, FillRect, GetDC, GetStockObject, LineTo, MoveToEx,
    NULL_BRUSH, Rectangle, ReleaseDC, SelectObject, CreateBitmap, PS_SOLID,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, HICON, ICONINFO,
};

/// Generate a 32×32 HICON depicting a stylised QR‑code scanner.
///
/// The icon is drawn entirely through GDI so it works on all Windows versions
/// without bundling an external `.ico` file.
///
/// # Layout
///
/// ```text
/// ┌──────────────────────────┐
/// │  ╔══╗        ╔══╗        │   cyan border
/// │  ║▓▓║        ║▓▓║        │   white finder blocks
/// │  ╚══╝        ╚══╝        │
/// │ ─────── laser ────────── │   red scan line
/// │  ╔══╗     ▓▓             │
/// │  ║▓▓║     ▓▓▓            │
/// │  ╚══╝                    │
/// └──────────────────────────┘
/// ```
pub fn create_qr_scanner_icon() -> HICON {
    unsafe {
        let hdc_screen = GetDC(HWND(std::ptr::null_mut()));
        let hdc_mem = CreateCompatibleDC(hdc_screen);

        let size = 32;
        let hbm_color = CreateCompatibleBitmap(hdc_screen, size, size);
        let hbm_mask  = CreateBitmap(size, size, 1, 1, None);

        let old_obj = SelectObject(hdc_mem, hbm_color);

        // Deep‑navy background (#1A237E in BGR)
        let bg_brush = CreateSolidBrush(COLORREF(0x007E231A));
        let rc = RECT { left: 0, top: 0, right: size, bottom: size };
        FillRect(hdc_mem, &rc, bg_brush);
        let _ = DeleteObject(bg_brush);

        // Cyan targeting rectangle
        let pen = CreatePen(PS_SOLID, 2, COLORREF(0x00FFFF00));
        let old_pen   = SelectObject(hdc_mem, pen);
        let null_brush = GetStockObject(NULL_BRUSH);
        let old_brush  = SelectObject(hdc_mem, null_brush);
        let _ = Rectangle(hdc_mem, 2, 2, size - 2, size - 2);

        // White QR‑finder pattern blocks
        let white_brush = CreateSolidBrush(COLORREF(0x00FFFFFF));
        let qr_blocks = [
            RECT { left:  6, top:  6, right: 13, bottom: 13 },
            RECT { left: 19, top:  6, right: 26, bottom: 13 },
            RECT { left:  6, top: 19, right: 13, bottom: 26 },
            RECT { left: 18, top: 18, right: 23, bottom: 23 },
            RECT { left: 24, top: 22, right: 26, bottom: 26 },
        ];
        for b in &qr_blocks {
            FillRect(hdc_mem, b, white_brush);
        }
        let _ = DeleteObject(white_brush);

        // Bright‑red laser line through the middle (#FF3D00 in BGR)
        let laser_pen = CreatePen(PS_SOLID, 2, COLORREF(0x00003DFF));
        let _ = SelectObject(hdc_mem, laser_pen);
        let _ = MoveToEx(hdc_mem, 4, size / 2, None);
        let _ = LineTo(hdc_mem, size - 4, size / 2);

        // Restore GDI state
        SelectObject(hdc_mem, old_pen);
        SelectObject(hdc_mem, old_brush);
        let _ = DeleteObject(pen);
        let _ = DeleteObject(laser_pen);

        SelectObject(hdc_mem, old_obj);
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        // Create the HICON from the colour + mask bitmaps
        let icon_info = ICONINFO {
            fIcon:    BOOL(1),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask:  hbm_mask,
            hbmColor: hbm_color,
        };

        let hicon = CreateIconIndirect(&icon_info).unwrap();
        let _ = DeleteObject(hbm_color);
        let _ = DeleteObject(hbm_mask);
        hicon
    }
}
