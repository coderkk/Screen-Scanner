//! Binary entry‑point for Screen Scanner.
//!
//! All logic lives in the library crate; this file simply boots the
//! application by calling [`screen_scanner::app_state::run_app`].

#![windows_subsystem = "windows"]

fn main() {
    screen_scanner::app_state::run_app();
}

