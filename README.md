# Markdown Editor

<div align="center">

<img src="favicon.svg" alt="Markdown Editor Logo" width="84" height="84" />

![Rust](https://img.shields.io/badge/Rust-1.99%2B-orange?logo=rust&logoColor=white)
![Edition](https://img.shields.io/badge/Edition-2024-blue?logo=rust)
![Slint GUI](https://img.shields.io/badge/GUI-Slint%201.18.1-7952b3?logo=rust)
![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)
![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey)

**A high-performance, native desktop Markdown editor with live preview, built with Rust and the Slint GUI toolkit.**

[Features](#features) • [Installation](#installation--building) • [Keybindings](#keyboard-shortcuts) • [Architecture](#architecture) • [License](#license)

</div>

---

## Overview

**Markdown Editor** is a fast, lightweight, and modern desktop application designed for writers, technical bloggers, and developers. By combining the safety and efficiency of **Rust** with the declarative reactive UI engine of **Slint**, Markdown Editor delivers instantaneous startup times, minimal memory consumption, and a fluid editing experience — completely free from Electron or web-browser overhead.

---

## Features

### 📝 CommonMark 0.31 & GitHub Flavored Markdown (GFM)
* **Headings & Typography**: Smooth rendering of H1–H6, bold, italic, and strikethrough text.
* **GFM Tables**: Full support for tables with column alignment, intelligent multi-phase auto-layout, column wrapping, and an interactive table insertion dialog.
* **Interactive Task Lists**: Checkbox support (`[ ]` and `[x]`) alongside ordered and unordered lists.
* **Footnotes**: Full GFM footnote syntax (`[^1]` and `[^1]: ...`) with bidirectional navigation between text references and definitions.
* **Code Blocks & Quotes**: Monospace code blocks with automatic tab expansion (`expand_tabs`) and styled blockquotes.

### ⚡ Split-View & Flexible Layouts
* **Split View**: Side-by-side editing and real-time live preview.
* **Editor-Only**: Distraction-free writing mode.
* **Preview-Only**: Clean, book-style reading and proofreading mode.

### 📁 Integrated File Tree Explorer
* Built-in file browser for browsing folders and subdirectories.
* Quick opening of `.md` and `.markdown` files.
* Single-click expand/collapse and instant directory synchronization (`F5`).

### 📋 Smart Clipboard & Rich Export
* **Copy HTML (`Ctrl+Shift+C`)**: Copies valid HTML into the system clipboard with `text/html` MIME type for direct pasting into word processors, rich text applications, or email clients (with automatic fallback to `wl-copy` and `xclip` on Linux).
* **Copy Plain Text**: Strips Markdown tags for clean sharing of pure prose.

### 🎨 Adaptive Themes & Typography
* **KDE Breeze-inspired Palette**: Carefully tuned colors for high contrast and comfortable long-form writing.
* **OS Theme Detection**: Automatic synchronization with OS light/dark modes on startup via `dark-light`, plus live reactive subscription to desktop theme changes.
* **Manual Override**: Toggle between Auto, Dark, and Light themes directly from the status bar.
* **Bundled Nerd Fonts**: Includes Hack, Ubuntu, Ubuntu Mono, Ubuntu Sans, and Symbols Nerd Font for consistent cross-platform glyph rendering.

### ⌨️ Layout-Agnostic Keyboard Shortcuts
* Global keyboard shortcut coordinator with automatic layout normalization: shortcuts (`Ctrl+B`, `Ctrl+I`, `Ctrl+S`, etc.) work seamlessly regardless of whether your active input layout is English (QWERTY), Cyrillic (ЙЦУКЕН), or Greek.

### 📊 Real-Time Document Statistics
* Live status bar counters tracking total lines, words, and characters.

### 🛡️ Single-Instance Native Dialogs
* Native system dialogs for file operations and Help windows (About, Licenses) with process tracking to prevent duplicate instances and memory leaks.

---

## Keyboard Shortcuts

| Shortcut | Command | Description |
| :--- | :--- | :--- |
| **File Operations** | | |
| `Ctrl + N` | `file.new` | Create a new document |
| `Ctrl + O` | `file.open` | Open an existing Markdown file |
| `Ctrl + S` | `file.save` | Save current document |
| `Ctrl + Shift + S` | `file.save_as` | Save document with a new name |
| `F5` | `file.reload_dir` | Refresh file tree explorer |
| `Ctrl + Q` | `file.exit` | Exit the editor |
| **Editing & Clipboard** | | |
| `Ctrl + Z` | `edit.undo` | Undo last action |
| `Ctrl + Shift + Z` / `Ctrl + Y` | `edit.redo` | Redo previously undone action |
| `Ctrl + X` | `edit.cut` | Cut selection |
| `Ctrl + C` | `edit.copy` | Copy selection |
| `Ctrl + V` | `edit.paste` | Paste from clipboard |
| `Ctrl + A` | `edit.select_all` | Select all text |
| `Ctrl + Shift + C` | `edit.copy_preview_html` | Copy rendered preview as HTML |
| **Markdown Formatting** | | |
| `Ctrl + 1` / `Ctrl + 2` | `format.h1` / `format.h2` | Apply Heading 1 / Heading 2 |
| `Ctrl + B` | `format.bold` | Toggle bold text formatting |
| `Ctrl + I` | `format.italic` | Toggle italic text formatting |
| `Ctrl + K` | `format.code_block` | Insert or wrap code block |
| `Ctrl + Shift + T` | `format.table` | Open table creation dialog |
| **View Modes** | | |
| `Ctrl + Shift + B` | `view.toggle_sidebar` | Toggle file browser sidebar |
| `Ctrl + Shift + 1` | `view.split` | Switch to Split View (Editor + Preview) |
| `Ctrl + Shift + 2` | `view.editor` | Switch to Editor-Only View |
| `Ctrl + Shift + 3` | `view.preview` | Switch to Preview-Only View |
| **Help & Dialogs** | | |
| `F1` | `help.guide` | Open Help Guide |
| `Ctrl + L` | `help.licenses` | Show Licenses dialog |
| `Escape` | `dialog.cancel` | Dismiss dialog or popup |
| `Enter` | `dialog.confirm` | Confirm dialog action |

---

## Installation & Building

### Prerequisites

Ensure you have **Rust 1.99+** (Edition 2024) installed via [rustup](https://rustup.rs/):

```bash
rustup update stable
```

#### Linux System Dependencies
On Debian/Ubuntu-based distributions:
```bash
sudo apt-get update
sudo apt-get install -y cmake fontconfig libfontconfig1-dev libxkbcommon-dev libwayland-dev libx11-dev zenity
```

On Fedora/RHEL-based distributions:
```bash
sudo dnf install -y cmake fontconfig-devel libxkbcommon-devel wayland-devel libX11-devel zenity
```

On Arch Linux:
```bash
sudo pacman -S cmake fontconfig libxkbcommon wayland libx11 zenity
```

### Build from Source

```bash
# 1. Clone the repository
git clone https://github.com/chungus-big/markdown-editor.git
cd markdown-editor

# 2. Build and run in debug mode
cargo run

# 3. Build optimized release binary
cargo build --release
```

The compiled executable will be located at `target/release/markdown-editor`.

### Running Tests

Run the full suite of unit and regression tests:
```bash
cargo test
```

### Desktop Integration (Linux)

Starter files are included for optional integration into Linux desktop application menus, docks, and file associations:

* [`markdown-editor.desktop`](./markdown-editor.desktop) — Standard FreeDesktop `.desktop` application launcher template.
* [`favicon.svg`](./favicon.svg) — Scalable vector icon.

To install them into your user profile (`~/.local/`):

```bash
# 1. Install release binary
mkdir -p ~/.local/bin
cp target/release/markdown-editor ~/.local/bin/

# 2. Install application icon
mkdir -p ~/.local/share/icons/hicolor/scalable/apps
cp favicon.svg ~/.local/share/icons/hicolor/scalable/apps/markdown-editor.svg

# 3. Install desktop shortcut
mkdir -p ~/.local/share/applications
cp markdown-editor.desktop ~/.local/share/applications/

# 4. Update desktop application cache (optional)
update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
gtk-update-icon-cache ~/.local/share/icons/hicolor/ 2>/dev/null || true
```

> **Note**: These files are starter templates. You can customize the `Exec` path, icon name, or categories in `markdown-editor.desktop` to suit your system setup or system-wide installations (`/usr/local/bin` and `/usr/share/`).

---

## Architecture

```text
markdown-editor/
├── .agents/              # AI coding agent skills & configurations
│   └── skills/slint/     # Slint UI skill instructions and references
├── lang/                 # Gettext localization bundles (e.g. Russian .po)
├── src/                  # Rust application source code
│   ├── main.rs           # Entry point and Slint UI callback bindings
│   ├── app_state.rs      # Application state (document, file path, pending actions)
│   ├── dialogs.rs        # Native file pickers and single-instance help dialogs
│   ├── file_browser.rs   # Recursive directory scanner and tree model
│   ├── formatting.rs     # Selection-aware Markdown syntax insertion engine
│   ├── shortcuts.rs      # Centralized keyboard chord registry with layout normalization
│   ├── markdown/         # CommonMark + GFM parser and layout metrics
│   │   ├── parser.rs     # pulldown-cmark state machine
│   │   ├── layout.rs     # Height and table column width calculations
│   │   ├── types.rs      # Markdown block data structures
│   │   ├── utils.rs      # Tab expansion and text utilities
│   │   └── export.rs     # HTML & plain text transformation
│   ├── preview.rs        # Slint model conversion and document statistics
│   ├── sample.md         # Embedded reference document
│   └── system.rs         # OS theme detection and clipboard utilities
├── ui/                   # Slint UI markup and assets
│   ├── fonts/            # Bundled Nerd Fonts
│   └── app-window.slint  # Main window layout, MenuBar, and custom components
├── build.rs              # Slint build script with bundled translations
├── Cargo.toml            # Rust manifest and dependencies
├── favicon.svg           # Application vector icon (starter template)
├── markdown-editor.desktop # Linux FreeDesktop entry (starter template)
├── LICENSE               # GNU General Public License v3.0
└── THIRD_PARTY_LICENSES.md # Third-party component legal attributions
```

---

## Localization

Markdown Editor supports multilingual interfaces via bundled Gettext translations:
* **English** (Default)
* **Russian** (`lang/ru/LC_MESSAGES/markdown-editor.po`)

The active language is automatically determined from system locale settings (`LANG`, `LC_ALL`, `LC_MESSAGES`).

---

## License

This project is licensed under the **GNU General Public License v3.0 or later** ([GPL-3.0-or-later](./LICENSE)).

### Third-Party Attributions
For detailed copyright and license information regarding bundled fonts (Hack, Ubuntu, Nerd Fonts), icon sets (Octicons, Devicons, Font Awesome), and Rust dependencies (Slint, pulldown-cmark, dark-light, arboard), please refer to [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).
