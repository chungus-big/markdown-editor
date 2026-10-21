# AGENTS.md — Development Guidelines and Project Overview

## About the Project: Markdown Editor

A desktop Markdown editor written in **Rust** using the **Slint** GUI framework.

---

## Tech Stack & Tools

* **Rust**: version 1.99+, edition 2024.
* **Slint GUI**: version 1.18.1 (`slint`, `slint-build`).
  * UI files: `ui/` directory (`.slint`).
  * Application logic and integration: `src/` (`main.rs`, modules).
* **Cross-platform clipboard `arboard`**: integration of `arboard = "3.6.1"` (Wayland / X11 / Windows / macOS) in `src/system.rs` with clear separation: "Copy HTML" places valid HTML into the clipboard (with MIME `text/html` for rich-text apps and raw HTML for plain-text editors), while "Copy Text" copies clean, readable text without tags or syntax markers; includes automatic fallback to `wl-copy` and `xclip`.
* **OS theme detection**: `dark-light` 3.0.0 (auto-detects system theme on launch via `dark_light::detect()` and reactive background theme change tracking via `dark_light::subscribe()`).

---

## Project Structure

```text
markdown-editor/
├── .agents/              # Skills and agents
│   └── skills/slint/     # Slint instructions and recipes
├── lang/                 # Gettext localization directories (bundled translations)
│   └── ru/LC_MESSAGES/   # PO translation files (markdown-editor.po)
├── src/                  # Rust source code
│   ├── main.rs           # Application entry point, UI initialization, and Slint callbacks
│   ├── app_state.rs      # State management (AppState), pending actions, and save/load
│   ├── dialogs.rs        # System open/save dialogs and HelpDialogManager (Zenity/Kdialog)
│   ├── file_browser.rs   # Directory scanning module for directory trees with .md files
│   ├── formatting.rs     # Formatting planner (FormattingPlan), table generator (generate_markdown_table), Undo/Redo coordinator, and tests
│   ├── shortcuts.rs      # Unified keyboard shortcuts registry (CommandRegistry) with layout normalization
│   ├── markdown/         # CommonMark + GFM parsing and data models
│   │   ├── mod.rs        # Public facade and re-exports
│   │   ├── types.rs      # TableRow, MarkdownBlock, and BlockType enum
│   │   ├── parser.rs     # MarkdownParser with pulldown-cmark state machine
│   │   ├── layout.rs     # Block height estimation for Slint ScrollView (estimate_preview_height)
│   │   └── utils.rs      # Tab expansion (expand_tabs) and text utilities
│   ├── preview.rs        # Slint model adapters (convert_blocks_to_slint) and document statistics
│   ├── sample.md         # Embedded reference and test Markdown document
│   └── system.rs         # System theme (dark-light) and interface language detection
├── ui/                   # Slint UI layout
│   ├── fonts/            # Bundled fonts (Hack, Ubuntu, Ubuntu Mono, Ubuntu Sans, Symbols Nerd Font)
│   └── app-window.slint  # Main window: MenuBar, seamless workspace with borders (file tree, editor, preview), status bar
├── AGENTS.md             # This guideline
├── build.rs              # Slint build script (bundled translations + DefaultTranslationContext::None)
├── Cargo.toml            # Package manifest and dependencies
├── favicon.svg           # Application vector icon (starter template)
├── markdown-editor.desktop # Linux FreeDesktop entry (starter template)
├── LICENSE               # GNU General Public License v3.0 (GPL-3.0-or-later)
└── THIRD_PARTY_LICENSES.md # Legal notices for third-party fonts, icons, and libraries
```