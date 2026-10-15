//! Configuration types and persistence.
//!
//! Stores the global shortcut key bindings and history preferences.
//! Settings are persisted as JSON next to the executable.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
};

// ────────────────────────────────────────────────────────────
// Modifier + Key lookup tables
// ────────────────────────────────────────────────────────────

/// Available modifier‑key combinations.
///
/// Each entry is `(display_name, win32_modifiers)`.
/// `MOD_NOREPEAT` is always OR‑ed in to prevent repeated WM_HOTKEY messages
/// while the keys are held down.
pub const MOD_OPTIONS: &[(&str, u32)] = &[
    ("Ctrl + Alt",   MOD_CONTROL.0 | MOD_ALT.0   | MOD_NOREPEAT.0),
    ("Ctrl + Shift", MOD_CONTROL.0 | MOD_SHIFT.0  | MOD_NOREPEAT.0),
    ("Alt + Shift",  MOD_ALT.0     | MOD_SHIFT.0  | MOD_NOREPEAT.0),
    ("Win + Alt",    MOD_WIN.0     | MOD_ALT.0    | MOD_NOREPEAT.0),
];

/// Available virtual‑key choices.
///
/// Each entry is `(display_name, VK_code)`.
pub const KEY_OPTIONS: &[(&str, u32)] = &[
    ("A", 0x41), ("B", 0x42), ("C", 0x43), ("D", 0x44), ("E", 0x45),
    ("F", 0x46), ("G", 0x47), ("H", 0x48), ("I", 0x49), ("J", 0x4A),
    ("K", 0x4B), ("L", 0x4C), ("M", 0x4D), ("N", 0x4E), ("O", 0x4F),
    ("P", 0x50), ("Q", 0x51), ("R", 0x52), ("S", 0x53), ("T", 0x54),
    ("U", 0x55), ("V", 0x56), ("W", 0x57), ("X", 0x58), ("Y", 0x59),
    ("Z", 0x5A), ("F1", 0x70), ("F2", 0x71), ("F3", 0x72), ("F4", 0x73),
    ("F7", 0x76), ("F8", 0x77), ("F9", 0x78), ("F10", 0x79), ("F11", 0x7A), ("F12", 0x7B),
];

/// Maximum‑history‑record presets shown in the Options dialog.
pub const HIST_LIMITS: &[usize] = &[10, 20, 50, 100, 200];

// ────────────────────────────────────────────────────────────
// Data types
// ────────────────────────────────────────────────────────────

/// Identifies one shortcut key: a modifier‑combo index + a key index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyConfig {
    /// Index into [`MOD_OPTIONS`].
    pub mod_index: usize,
    /// Index into [`KEY_OPTIONS`].
    pub key_index: usize,
}

/// Persistent application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub fullscreen_hotkey: KeyConfig,
    pub region_hotkey: KeyConfig,
    pub record_history: bool,
    pub max_history: usize,
}

impl AppConfig {
    /// Human‑readable label for the fullscreen‑scan shortcut (e.g. `"Ctrl + Alt + S"`).
    pub fn format_fullscreen_hotkey(&self) -> String {
        let m = MOD_OPTIONS.get(self.fullscreen_hotkey.mod_index).map(|m| m.0).unwrap_or("Ctrl + Alt");
        let k = KEY_OPTIONS.get(self.fullscreen_hotkey.key_index).map(|k| k.0).unwrap_or("S");
        format!("{} + {}", m, k)
    }

    /// Human‑readable label for the region‑scan shortcut (e.g. `"Ctrl + Alt + R"`).
    pub fn format_region_hotkey(&self) -> String {
        let m = MOD_OPTIONS.get(self.region_hotkey.mod_index).map(|m| m.0).unwrap_or("Ctrl + Alt");
        let k = KEY_OPTIONS.get(self.region_hotkey.key_index).map(|k| k.0).unwrap_or("R");
        format!("{} + {}", m, k)
    }
}

impl Default for AppConfig {
    /// Returns the factory‑default configuration:
    /// * Fullscreen scan: `Ctrl + Alt + S`
    /// * Region scan:     `Ctrl + Alt + R`
    /// * History enabled, 50 records max
    fn default() -> Self {
        Self {
            fullscreen_hotkey: KeyConfig { mod_index: 0, key_index: 18 }, // Ctrl+Alt+S
            region_hotkey:     KeyConfig { mod_index: 0, key_index: 17 }, // Ctrl+Alt+R
            record_history: true,
            max_history: 50,
        }
    }
}

// ────────────────────────────────────────────────────────────
// Persistence helpers
// ────────────────────────────────────────────────────────────

/// Path to the JSON config file (next to the executable).
pub fn get_config_path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop();
    p.push("scanner_config.json");
    p
}

/// Load configuration from disk, or create the default config file if missing / invalid.
pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str(&content) {
            return cfg;
        }
    }
    let default_cfg = AppConfig::default();
    let _ = fs::write(&path, serde_json::to_string_pretty(&default_cfg).unwrap_or_default());
    default_cfg
}

/// Persist configuration to disk.
pub fn save_config(cfg: &AppConfig) {
    let path = get_config_path();
    let _ = fs::write(&path, serde_json::to_string_pretty(cfg).unwrap_or_default());
}

// ────────────────────────────────────────────────────────────
// Unit tests
// ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_shortcuts() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.format_fullscreen_hotkey(), "Ctrl + Alt + S");
        assert_eq!(cfg.format_region_hotkey(), "Ctrl + Alt + R");
    }

    #[test]
    fn default_config_has_history_enabled() {
        let cfg = AppConfig::default();
        assert!(cfg.record_history);
        assert_eq!(cfg.max_history, 50);
    }

    #[test]
    fn config_round_trips_through_json() {
        let cfg = AppConfig {
            fullscreen_hotkey: KeyConfig { mod_index: 2, key_index: 5 },
            region_hotkey: KeyConfig { mod_index: 1, key_index: 10 },
            record_history: false,
            max_history: 100,
        };

        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(cfg, restored);
    }

    #[test]
    fn format_hotkey_graceful_on_out_of_bounds() {
        let cfg = AppConfig {
            fullscreen_hotkey: KeyConfig { mod_index: 999, key_index: 999 },
            region_hotkey: KeyConfig { mod_index: 999, key_index: 999 },
            ..Default::default()
        };
        // Should fall back to the defaults without panicking
        assert_eq!(cfg.format_fullscreen_hotkey(), "Ctrl + Alt + S");
        assert_eq!(cfg.format_region_hotkey(), "Ctrl + Alt + R");
    }

    #[test]
    fn mod_options_table_not_empty() {
        assert!(!MOD_OPTIONS.is_empty());
    }

    #[test]
    fn key_options_table_not_empty() {
        assert!(!KEY_OPTIONS.is_empty());
    }

    #[test]
    fn hist_limits_sorted_ascending() {
        for pair in HIST_LIMITS.windows(2) {
            assert!(pair[0] < pair[1], "HIST_LIMITS must be sorted");
        }
    }
}
