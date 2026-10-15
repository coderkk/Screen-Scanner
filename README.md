# Screen Scanner (Rust)

A native Windows desktop tool built in pure Rust to scan and decode barcodes and QR codes on screen, running smoothly in your system tray.

---

## 🌟 Key Features

1. **Native Windows Executable & Icon**:
   - The `.exe` has an embedded high-resolution scanner icon in Windows Explorer.
   - Starts silently without any terminal/console window.

2. **System Tray Integration**:
   - Resides quietly in the Windows notification area (systray).
   - Right-click menu with live shortcut labels:
     - 📸 **Scan Whole Screen**: Captures all monitors and scans for barcodes.
     - ✂️ **Select Region to Scan**: Semi-transparent snipping overlay to drag & select.
     - 📜 **Scan History...**: Opens the history window with list, preview, and clipboard actions.
     - ⚙️ **Options (Configure Shortcuts & History)...**: Customizes hotkeys and history limits.
     - ℹ️ **About Screen Scanner...**: Shows version and barcode engine information.
     - 🚪 **Exit**: Clean exit.

3. **Toast Notifications on Scan**:
   - When a barcode/QR code is detected, Windows displays a toast notification with the detected format and a preview of the content, confirming it has been copied to your clipboard.

4. **Scan History & Configuration**:
   - **Options dialog**:
     - Customize hotkeys for full screen and region snip (<kbd>Ctrl</kbd>/<kbd>Alt</kbd>/<kbd>Shift</kbd>/<kbd>Win</kbd> + Keys).
     - Toggle: *"Record scanned items in history"*.
     - Choose maximum history entries (10, 20, 50, 100, 200).
     - Settings are stored in `scanner_config.json`.
   - **History window**:
     - Lists scanned codes with timestamp, format, and text preview.
     - **Copy to Clipboard** button (or double-click any item).
     - **Open in Browser** button (if URL).
     - **Clear All** button.
     - Persistent across restarts in `scanner_history.json`.

5. **Window Icons**:
   - Options and History dialog windows feature the custom scanner icon on their title bars and in the Windows task switcher (<kbd>Alt</kbd> + <kbd>Tab</kbd>).

---

## ⌨️ Default Shortcuts

| Shortcut | Action |
| :--- | :--- |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>S</kbd> | Scan whole active screen / monitors |
| <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>R</kbd> | Open region snip overlay (drag to select) |
| <kbd>ESC</kbd> | Cancel region selection |

---

## 🛠️ Prerequisites & Compilation

### Prerequisites
- **Rust toolchain** (1.70+ recommended): [rustup.rs](https://rustup.rs)
- **Windows GNU / MSVC toolchain**:
  - The build script (`build.rs`) uses `windres` (available via MinGW/GCC) or your Windows SDK to compile `resources.rc` and embed `app.ico` directly into the `.exe`.

### Building / Compiling

1. **Clone or open the project folder**:
   ```powershell
   cd screen_scanner
   ```

2. **Build Release Binary**:
   ```powershell
   cargo build --release
   ```
   The compiled `.exe` will be located at:
   ```text
   target\release\screen_scanner.exe
   ```

3. **Run Unit Tests**:
   ```powershell
   cargo test
   ```

4. **Run Directly during Development**:
   ```powershell
   cargo run --release
   ```

---

## 🚀 How to Run

Run the compiled executable:
```powershell
.\target\release\screen_scanner.exe
```
Or double-click `screen_scanner.exe` in Windows Explorer. It starts silently and places an icon in your system tray.

