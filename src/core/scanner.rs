//! Screen capture and barcode / QR‑code decoding engine.
//!
//! This module owns the pure‑logic side of scanning:
//! * Capturing the virtual desktop (stitching multiple monitors).
//! * Running the `rxing` decoder in both normal and inverted luminance passes.
//! * Convenience wrappers that trigger a full‑screen or region scan.

use std::collections::HashMap;

use image::{DynamicImage, RgbaImage};
use rxing::{
    common::HybridBinarizer, BarcodeFormat, BinaryBitmap, BufferedImageLuminanceSource,
    DecodeHintType, DecodeHintValue, LuminanceSource, MultiUseMultiFormatReader, Reader,
};
use xcap::Monitor;

// ────────────────────────────────────────────────────────────
// Data types
// ────────────────────────────────────────────────────────────

/// The result of a successful barcode / QR‑code decode.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// The symbology that was detected (e.g. `QR_CODE`, `CODE_128`).
    pub format: BarcodeFormat,
    /// The decoded text content.
    pub text: String,
}

// ────────────────────────────────────────────────────────────
// Decoding
// ────────────────────────────────────────────────────────────

/// Attempt to decode a barcode or QR code from the given image.
///
/// Two passes are made:
/// 1. **Normal** luminance – handles standard black‑on‑white codes.
/// 2. **Inverted** luminance – handles white‑on‑black / dark‑mode QR codes.
///
/// Returns `None` if no code is found after both passes.
pub fn decode_barcode(img: &DynamicImage) -> Option<ScanResult> {
    let mut reader = MultiUseMultiFormatReader::default();
    let mut hints = HashMap::new();
    hints.insert(DecodeHintType::TRY_HARDER, DecodeHintValue::TryHarder(true));

    // Pass 1: standard luminance
    let lum_source = BufferedImageLuminanceSource::new(img.clone());
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(lum_source));

    if let Ok(res) = reader.decode_with_hints(&mut bitmap, &hints) {
        return Some(ScanResult {
            format: res.getBarcodeFormat().clone(),
            text: res.getText().to_string(),
        });
    }

    // Pass 2: inverted luminance (white‑on‑black)
    let mut lum_source_inv = BufferedImageLuminanceSource::new(img.clone());
    lum_source_inv.invert();
    let mut inv_bitmap = BinaryBitmap::new(HybridBinarizer::new(lum_source_inv));
    if let Ok(res) = reader.decode_with_hints(&mut inv_bitmap, &hints) {
        return Some(ScanResult {
            format: res.getBarcodeFormat().clone(),
            text: res.getText().to_string(),
        });
    }

    None
}

// ────────────────────────────────────────────────────────────
// Screen capture
// ────────────────────────────────────────────────────────────

/// Capture the entire virtual desktop (all monitors stitched together).
///
/// Returns `(image, virt_x, virt_y, total_width, total_height)`.
/// `virt_x` / `virt_y` are the coordinates of the top‑left corner of the
/// virtual desktop (can be negative if a monitor extends to the left / above
/// the primary display).
pub fn capture_virtual_screen() -> Option<(DynamicImage, i32, i32, u32, u32)> {
    let monitors = Monitor::all().ok()?;
    if monitors.is_empty() {
        return None;
    }

    let min_x = monitors.iter().map(|m| m.x()).min().unwrap_or(0);
    let min_y = monitors.iter().map(|m| m.y()).min().unwrap_or(0);
    let max_x = monitors.iter().map(|m| m.x() + m.width() as i32).max().unwrap_or(1920);
    let max_y = monitors.iter().map(|m| m.y() + m.height() as i32).max().unwrap_or(1080);

    let total_w = (max_x - min_x).max(1) as u32;
    let total_h = (max_y - min_y).max(1) as u32;

    let mut stitched = RgbaImage::new(total_w, total_h);

    for m in monitors {
        if let Ok(cap) = m.capture_image() {
            let offset_x = (m.x() - min_x) as u32;
            let offset_y = (m.y() - min_y) as u32;
            for (x, y, pixel) in cap.enumerate_pixels() {
                let dest_x = offset_x + x;
                let dest_y = offset_y + y;
                if dest_x < total_w && dest_y < total_h {
                    stitched.put_pixel(dest_x, dest_y, *pixel);
                }
            }
        }
    }

    Some((DynamicImage::ImageRgba8(stitched), min_x, min_y, total_w, total_h))
}

// ────────────────────────────────────────────────────────────
// High‑level scan triggers
// ────────────────────────────────────────────────────────────

/// Capture the full virtual desktop and try to decode a barcode / QR code.
///
/// On success the result is handed to `handle_scan_result` (toast + dialog).
/// On failure a warning message box is shown.
pub fn trigger_fullscreen_scan() {
    if let Some((img, _, _, _, _)) = capture_virtual_screen() {
        if let Some(result) = decode_barcode(&img) {
            crate::notifications::handle_scan_result(&result);
        } else {
            crate::notifications::show_message("No barcode or QR code detected on active screen.");
        }
    } else {
        crate::notifications::show_message("Failed to capture screen monitors.");
    }
}

/// Show the snipping overlay so the user can drag‑select a region to scan.
pub fn trigger_region_snip(hwnd_overlay: windows::Win32::Foundation::HWND) {
    if let Some((img, vx, vy, vw, vh)) = capture_virtual_screen() {
        unsafe {
            crate::overlay::SNIP_STATE = Some(crate::overlay::SnippingState {
                is_selecting: false,
                start_pt: windows::Win32::Foundation::POINT::default(),
                current_pt: windows::Win32::Foundation::POINT::default(),
                vx,
                vy,
                vw: vw as i32,
                vh: vh as i32,
                captured_screen: Some(img),
            });

            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::Graphics::Gdi::InvalidateRect;
            SetWindowPos(hwnd_overlay, HWND_TOPMOST, vx, vy, vw as i32, vh as i32, SWP_SHOWWINDOW)
                .unwrap_or_default();
            let _ = ShowWindow(hwnd_overlay, SW_SHOW);
            let _ = InvalidateRect(hwnd_overlay, None, true);
        }
    } else {
        crate::notifications::show_message("Failed to capture screen monitors.");
    }
}

// ────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Luma, DynamicImage};

    #[test]
    fn decode_barcode_returns_none_for_blank_image() {
        let img = DynamicImage::ImageLuma8(image::ImageBuffer::<Luma<u8>, Vec<u8>>::new(100, 100));
        assert!(decode_barcode(&img).is_none(), "Blank image should produce no result");
    }

    #[test]
    fn scan_result_debug_format() {
        let r = ScanResult {
            format: BarcodeFormat::QR_CODE,
            text: "hello".into(),
        };
        let dbg = format!("{:?}", r);
        assert!(dbg.contains("QR_CODE"));
        assert!(dbg.contains("hello"));
    }
}
