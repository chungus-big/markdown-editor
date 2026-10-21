//! System environment detection, OS theme and localization helpers.

/// Embedded sample Markdown document containing reference features and test cases.
pub const SAMPLE_MARKDOWN: &str = include_str!("sample.md");

/// Detects if the current desktop environment is using a dark theme.
pub fn detect_system_dark() -> bool {
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}

/// Detects the system language from standard environment variables (LC_ALL, LC_MESSAGES, LANG).
pub fn detect_system_language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let lower = val.to_lowercase();
            if lower.starts_with("ru") {
                return "ru";
            }
        }
    }
    "en"
}

/// Returns true if Russian language is active (mode 2) or auto-detected (mode 0 with system "ru").
pub fn is_russian(language_mode: u8) -> bool {
    match language_mode {
        1 => false,
        2 => true,
        _ => detect_system_language() == "ru",
    }
}

/// Returns the embedded sample markdown document.
pub fn get_sample_markdown(_is_ru: bool) -> &'static str {
    SAMPLE_MARKDOWN
}

/// Returns a localized default document name when saving untitled files.
pub fn get_document_fallback_name(is_ru: bool) -> &'static str {
    if is_ru {
        "Документ"
    } else {
        "Document"
    }
}

/// Copies plain text to the system clipboard using arboard with CLI tool fallbacks.
pub fn copy_text_to_clipboard(text: &str) -> bool {
    if let Ok(mut clip) = arboard::Clipboard::new()
        && clip.set_text(text).is_ok()
    {
        return true;
    }
    // Fallback to external tools on Linux if arboard is unavailable
    if let Ok(mut child) = std::process::Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        if child.wait().map(|s| s.success()).unwrap_or(false) {
            return true;
        }
    }
    if let Ok(mut child) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        if child.wait().map(|s| s.success()).unwrap_or(false) {
            return true;
        }
    }
    false
}

/// Copies HTML to the system clipboard (providing text/html for rich text and HTML markup for plain text).
pub fn copy_html_to_clipboard(html: &str) -> bool {
    let mut success = false;
    if let Ok(mut clip) = arboard::Clipboard::new()
        && (clip.set_html(html, Some(html)).is_ok() || clip.set_text(html).is_ok())
    {
        success = true;
    }
    // Also broadcast to wl-copy with text/html mime-type on Wayland
    if let Ok(mut child) = std::process::Command::new("wl-copy")
        .arg("-t")
        .arg("text/html")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(html.as_bytes());
        }
        let _ = child.wait();
        success = true;
    }
    if !success {
        copy_text_to_clipboard(html);
    }
    true
}

/// Safely extracts the selected text slice from a string given byte offset positions.
///
/// Ensures valid UTF-8 character boundary alignment.
/// If `anchor == cursor` or if the selection is empty/invalid, returns the full text.
pub fn extract_text_selection(text: &str, anchor: i32, cursor: i32) -> &str {
    let raw_start = anchor.min(cursor).max(0) as usize;
    let raw_end = anchor.max(cursor).max(0) as usize;
    if raw_start == raw_end || text.is_empty() {
        return text;
    }

    let mut start = raw_start.min(text.len());
    let mut end = raw_end.min(text.len());

    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }

    if start < end {
        &text[start..end]
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_selection_basic() {
        let text = "fn hello_world() {\n    println!(\"Hello\");\n}";
        // Extract "hello_world"
        assert_eq!(extract_text_selection(text, 3, 14), "hello_world");
        // Inverted cursor and anchor
        assert_eq!(extract_text_selection(text, 14, 3), "hello_world");
    }

    #[test]
    fn test_extract_text_selection_empty_returns_full() {
        let text = "println!(\"Test\");";
        assert_eq!(extract_text_selection(text, 5, 5), text);
        assert_eq!(extract_text_selection(text, 0, 0), text);
        assert_eq!(extract_text_selection(text, -1, -1), text);
    }

    #[test]
    fn test_extract_text_selection_multibyte_utf8() {
        let text = "Привет, мир! 🚀 Код";
        // "Привет" in UTF-8 is 12 bytes (2 bytes per Cyrillic char)
        assert_eq!(extract_text_selection(text, 0, 12), "Привет");
        // Safe adjustment when byte index lands inside a multi-byte character boundary
        assert_eq!(extract_text_selection(text, 1, 11), "Привет");
    }

    #[test]
    fn test_extract_text_selection_clamped_bounds() {
        let text = "short";
        assert_eq!(extract_text_selection(text, 0, 100), "short");
        assert_eq!(extract_text_selection(text, -50, 3), "sho");
    }
}
