# FocusLock

A cross-platform focus timer that helps you maintain a healthy work-rest rhythm. When it's time to rest, FocusLock forces a fullscreen overlay to block your screen until the break ends.

## Features

- **Custom Stage Cycles**: Configure work/rest intervals to match your flow (e.g., 45min work → 15min rest).
- **Fullscreen Overlay**: When break time arrives, a semi-transparent (or full-black/dark) overlay blocks your screen—soft enforcement to actually rest.
- **Sound Notifications**: Get audio cues when work ends (enter rest) and when rest ends (return to work). Supports built-in beeps and custom audio files.
- **System Tray**: Stay in the background with a tray icon that shows your current status and remaining time.
- **Skip Shortcut**: Press `Ctrl+Shift+F2` (Windows) or `Cmd+Shift+F2` (macOS) to skip a break if needed.
- **Auto-Reset**: If you're away for too long (default: 30 min), FocusLock resets the timer to keep things sane.
- **Check for Updates**: Automatically checks GitHub Releases for new versions.
- **i18n**: Supports Chinese and English (more languages welcome!).

## Installation

### Windows

Download the latest `FocusLock_vX.X.X_x64-setup.exe` from [GitHub Releases](https://github.com/weifeng-work/FocusLock/releases) and run the installer.

### macOS / Linux

Currently Windows-only. macOS and Linux support is planned.

## Usage

1. **First Launch**: FocusLock starts minimized in the system tray. Click the tray icon to open settings.
2. **Configure Stages**: Set up your work/rest cycles in the settings panel.
3. **Start Focusing**: The timer starts automatically. When work ends, a fullscreen overlay appears for your break.
4. **Customize**: Change overlay style, rest message, sound notifications, and more in settings.

## Configuration

Settings are stored in:
- **Windows**: `%APPDATA%\FocusLock\config.json`
- **macOS**: `~/Library/Application Support/FocusLock/config.json`

You can also place custom sound files in the `sounds/` subdirectory of the data directory.

## Building from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri 2.x](https://tauri.app/)

### Steps

```bash
# Clone the repo
git clone https://github.com/weifeng-work/FocusLock.git
cd FocusLock

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build release
npm run tauri build
```

## Tech Stack

- **Framework**: Tauri 2.x
- **Backend**: Rust
- **Frontend**: Vue 3 + TypeScript + Vite
- **State Management**: Pinia

## Contributing

Contributions are welcome! Please feel free to submit a PR or open an issue.

## License

MIT

## Author

weifeng-work

## Links

- **GitHub**: https://github.com/weifeng-work/FocusLock
- **Releases**: https://github.com/weifeng-work/FocusLock/releases
- **WeChat Support Group**: Scan the QR code in the settings panel.
