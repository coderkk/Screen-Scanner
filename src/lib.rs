//! Screen Scanner library root.

pub mod app_state;
pub mod core;
pub mod storage;
pub mod ui;
pub mod utils;

// Re-exports for convenience and backwards-compatibility
pub use core::config;
pub use core::scanner;
pub use storage::history;
pub use ui::icons;
pub use ui::notifications;
pub use ui::options;
pub use ui::overlay;
pub use ui::tray;
