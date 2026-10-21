//! Markdown editor text formatting planner, dispatching, and atomic undo/redo action tracking.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::markdown;
use crate::AppWindow;
use slint::ComponentHandle;

/// Concrete plan for inserting or wrapping Markdown syntax around editor selections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormattingPlan {
    /// Wraps existing text by inserting suffix at `end` and prefix at `start`.
    /// When suffix is non-empty, creates 2 insert steps in Slint's undo stack with NO word removal.
    /// When suffix is empty (e.g. headings or single-line quotes), creates 1 insert step at `start`.
    Wrap {
        start: usize,
        end: usize,
        prefix: String,
        suffix: String,
        original_anchor: usize,
        original_cursor: usize,
    },
    /// Inserts text at pos (when there is no selection). Creates 1 insert step in Slint.
    Insert {
        pos: usize,
        text: String,
        target_cursor: Option<usize>,
        original_anchor: usize,
        original_cursor: usize,
    },
    /// Replaces the selection `start..end` with `text` (e.g. multi-line blockquote/tasks).
    /// Creates 2 steps in Slint: TextRemove followed by TextInsert.
    Replace {
        start: usize,
        end: usize,
        text: String,
        original_anchor: usize,
        original_cursor: usize,
    },
}

/// Records the history of programmatic formatting steps for atomic undo and redo operations.
#[derive(Clone, Debug)]
pub struct UndoAction {
    pub steps: usize,
    pub restore_anchor: usize,
    pub restore_cursor: usize,
    pub redo_anchor: usize,
    pub redo_cursor: usize,
}

/// Analyzes cursor position or text selection and computes the exact non-destructive `FormattingPlan`.
pub fn plan_formatting(
    text: &str,
    anchor: usize,
    cursor: usize,
    kind: &str,
) -> FormattingPlan {
    let text_len = text.len();
    let mut anchor_clamped = anchor.min(text_len);
    let mut cursor_clamped = cursor.min(text_len);

    while !text.is_char_boundary(anchor_clamped) && anchor_clamped > 0 {
        anchor_clamped -= 1;
    }
    while !text.is_char_boundary(cursor_clamped) && cursor_clamped > 0 {
        cursor_clamped -= 1;
    }

    let mut start = anchor_clamped.min(cursor_clamped);
    let mut end = anchor_clamped.max(cursor_clamped);

    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

    // If no explicit selection, check if cursor is inside a word for inline styling (bold, italic, strikethrough)
    if start == end && !text.is_empty() && matches!(kind, "bold" | "italic" | "strikethrough") {
        let prev_char = text[..start].chars().next_back();
        let next_char = text[start..].chars().next();

        let inside_word = match (prev_char, next_char) {
            (Some(p), Some(n)) => is_word_char(p) && is_word_char(n),
            _ => false,
        };

        if inside_word {
            let mut word_start = start;
            while word_start > 0 {
                let prev_idx = text[..word_start]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let c = text[prev_idx..word_start].chars().next().unwrap();
                if is_word_char(c) {
                    word_start = prev_idx;
                } else {
                    break;
                }
            }
            let mut word_end = end;
            while word_end < text_len {
                let c = text[word_end..].chars().next().unwrap();
                if is_word_char(c) {
                    word_end += c.len_utf8();
                } else {
                    break;
                }
            }
            start = word_start;
            end = word_end;
        }
    }

    let has_selection = start < end;
    let selected_text = if has_selection { &text[start..end] } else { "" };

    match kind {
        "bold" => {
            if has_selection {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix: "**".to_string(),
                    suffix: "**".to_string(),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                FormattingPlan::Insert {
                    pos: start,
                    text: "****".to_string(),
                    target_cursor: Some(start + 2),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "italic" => {
            if has_selection {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix: "*".to_string(),
                    suffix: "*".to_string(),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                FormattingPlan::Insert {
                    pos: start,
                    text: "**".to_string(),
                    target_cursor: Some(start + 1),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "strikethrough" => {
            if has_selection {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix: "~~".to_string(),
                    suffix: "~~".to_string(),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                FormattingPlan::Insert {
                    pos: start,
                    text: "~~~~".to_string(),
                    target_cursor: Some(start + 2),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "code_block" => {
            if has_selection {
                let mut prefix = String::new();
                if start > 0 && !text[..start].ends_with('\n') {
                    prefix.push('\n');
                }
                prefix.push_str("```\n");

                let mut suffix = String::new();
                if !selected_text.ends_with('\n') {
                    suffix.push('\n');
                }
                suffix.push_str("```\n");

                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix,
                    suffix,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                let mut prefix = String::new();
                if start > 0 && !text[..start].ends_with('\n') {
                    prefix.push('\n');
                }
                let cursor_offset = start + prefix.len() + 4; // cursor inside code block on empty line
                prefix.push_str("```\n\n```\n");
                FormattingPlan::Insert {
                    pos: start,
                    text: prefix,
                    target_cursor: Some(cursor_offset),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "h1" => {
            let mut prefix = String::new();
            if start > 0 && !text[..start].ends_with('\n') {
                prefix.push('\n');
            }
            prefix.push_str("# ");
            if has_selection {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix,
                    suffix: String::new(),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                FormattingPlan::Insert {
                    pos: start,
                    text: prefix,
                    target_cursor: None,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "h2" => {
            let mut prefix = String::new();
            if start > 0 && !text[..start].ends_with('\n') {
                prefix.push('\n');
            }
            prefix.push_str("## ");
            if has_selection {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix,
                    suffix: String::new(),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                FormattingPlan::Insert {
                    pos: start,
                    text: prefix,
                    target_cursor: None,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "blockquote" => {
            if has_selection {
                if !selected_text.contains('\n') {
                    let mut prefix = String::new();
                    if start > 0 && !text[..start].ends_with('\n') {
                        prefix.push('\n');
                    }
                    prefix.push_str("> ");
                    FormattingPlan::Wrap {
                        start,
                        end,
                        prefix,
                        suffix: String::new(),
                        original_anchor: anchor_clamped,
                        original_cursor: cursor_clamped,
                    }
                } else {
                    let formatted = selected_text
                        .lines()
                        .map(|l| format!("> {l}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    FormattingPlan::Replace {
                        start,
                        end,
                        text: formatted,
                        original_anchor: anchor_clamped,
                        original_cursor: cursor_clamped,
                    }
                }
            } else {
                let mut prefix = String::new();
                if start > 0 && !text[..start].ends_with('\n') {
                    prefix.push('\n');
                }
                prefix.push_str("> ");
                FormattingPlan::Insert {
                    pos: start,
                    text: prefix,
                    target_cursor: None,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "task_list" => {
            if has_selection {
                if !selected_text.contains('\n') {
                    let mut prefix = String::new();
                    if start > 0 && !text[..start].ends_with('\n') {
                        prefix.push('\n');
                    }
                    prefix.push_str("- [ ] ");
                    FormattingPlan::Wrap {
                        start,
                        end,
                        prefix,
                        suffix: String::new(),
                        original_anchor: anchor_clamped,
                        original_cursor: cursor_clamped,
                    }
                } else {
                    let formatted = selected_text
                        .lines()
                        .map(|l| format!("- [ ] {l}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    FormattingPlan::Replace {
                        start,
                        end,
                        text: formatted,
                        original_anchor: anchor_clamped,
                        original_cursor: cursor_clamped,
                    }
                }
            } else {
                let mut prefix = String::new();
                if start > 0 && !text[..start].ends_with('\n') {
                    prefix.push('\n');
                }
                prefix.push_str("- [ ] Task\n");
                FormattingPlan::Insert {
                    pos: start,
                    text: prefix,
                    target_cursor: None,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        "footnote" => {
            if has_selection {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix: "[^1]: ".to_string(),
                    suffix: "\n".to_string(),
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                let mut prefix = String::new();
                if start > 0 && !text[..start].ends_with('\n') {
                    prefix.push('\n');
                }
                prefix.push_str("[^1]: Footnote description\n");
                FormattingPlan::Insert {
                    pos: start,
                    text: prefix,
                    target_cursor: None,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
        kind if kind.starts_with("table:") => {
            let parts: Vec<&str> = kind.split(':').collect();
            let cols = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
            let rows = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
            let mut prefix = String::new();
            if start > 0 && !text[..start].ends_with('\n') {
                prefix.push('\n');
            }
            prefix.push_str(&generate_markdown_table(cols, rows));
            FormattingPlan::Insert {
                pos: start,
                text: prefix,
                target_cursor: None,
                original_anchor: anchor_clamped,
                original_cursor: cursor_clamped,
            }
        }
        "table" => {
            let mut prefix = String::new();
            if start > 0 && !text[..start].ends_with('\n') {
                prefix.push('\n');
            }
            prefix.push_str(&generate_markdown_table(2, 1));
            FormattingPlan::Insert {
                pos: start,
                text: prefix,
                target_cursor: None,
                original_anchor: anchor_clamped,
                original_cursor: cursor_clamped,
            }
        }
        raw_snippet => {
            let clean = markdown::expand_tabs(raw_snippet, 4);
            if has_selection {
                FormattingPlan::Replace {
                    start,
                    end,
                    text: clean,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            } else {
                FormattingPlan::Insert {
                    pos: start,
                    text: clean,
                    target_cursor: None,
                    original_anchor: anchor_clamped,
                    original_cursor: cursor_clamped,
                }
            }
        }
    }
}

/// Generates a GFM Markdown table with custom number of columns and data rows.
pub fn generate_markdown_table(cols: usize, rows: usize) -> String {
    let cols = cols.clamp(1, 50);
    let rows = rows.clamp(1, 100);
    let mut s = String::new();

    // Header row
    s.push('|');
    for c in 1..=cols {
        s.push_str(&format!(" Header {} |", c));
    }
    s.push('\n');

    // Separator row
    s.push('|');
    for _ in 1..=cols {
        s.push_str("---|");
    }
    s.push('\n');

    // Data rows
    let mut cell_idx = 1;
    for _ in 1..=rows {
        s.push('|');
        for _ in 1..=cols {
            s.push_str(&format!(" Cell {} |", cell_idx));
            cell_idx += 1;
        }
        s.push('\n');
    }
    s.push('\n');
    s
}

/// Prepares formatting text and selection range based on current document text, anchor, cursor, and kind.
/// Returns (selection_start, selection_end, text_to_insert, optional_target_cursor).
#[allow(dead_code)]
pub fn prepare_formatting(
    text: &str,
    anchor: usize,
    cursor: usize,
    kind: &str,
) -> (usize, usize, String, Option<usize>) {
    match plan_formatting(text, anchor, cursor, kind) {
        FormattingPlan::Wrap {
            start,
            end,
            prefix,
            suffix,
            ..
        } => {
            let selected = if start < end && end <= text.len() {
                &text[start..end]
            } else {
                ""
            };
            (start, end, format!("{prefix}{selected}{suffix}"), None)
        }
        FormattingPlan::Insert {
            pos,
            text: ins_text,
            target_cursor,
            ..
        } => (pos, pos, ins_text, target_cursor),
        FormattingPlan::Replace {
            start,
            end,
            text: rep_text,
            ..
        } => (start, end, rep_text, None),
    }
}

/// Clears any active keyboard modifier states (Control, Shift, Alt, Meta) in the Slint window.
///
/// When shortcuts (like Ctrl+B, Ctrl+I, Ctrl+K, Ctrl+1, Ctrl+2, etc.) trigger formatting,
/// physical modifier keys (e.g. Control) are held down by the user. If programmatic `KeyPressed`
/// events for syntax insertion are dispatched while `modifiers.control` is active, Slint's
/// `TextInput` treats them as shortcut events rather than printable text input, silently ignoring
/// text insertion (see `i-slint-core::items::text::TextInput::key_event`).
/// Releasing modifiers ensures the editor treats subsequent synthetic key events as printable text.
pub fn clear_active_modifiers(window: &slint::Window) {
    use slint::platform::Key;
    for key in [
        Key::Control,
        Key::ControlR,
        Key::Shift,
        Key::ShiftR,
        Key::Alt,
        Key::AltGr,
        Key::Meta,
        Key::MetaR,
    ] {
        window.dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: key.into(),
        });
    }
}

/// Helper function to safely dispatch text insertion to TextInput with cleared modifiers.
fn dispatch_text_insert(window: &slint::Window, text: &str) {
    clear_active_modifiers(window);
    window.dispatch_event(slint::platform::WindowEvent::KeyPressed {
        text: text.into(),
    });
    window.dispatch_event(slint::platform::WindowEvent::KeyReleased {
        text: text.into(),
    });
}

/// Dispatches a formatting action to the Slint UI editor with atomic undo/redo management.
pub fn apply_formatting(
    ui: &AppWindow,
    kind: &str,
    undo_stack: &Arc<Mutex<Vec<UndoAction>>>,
    redo_stack: &Arc<Mutex<Vec<UndoAction>>>,
    is_programmatic: &Arc<AtomicBool>,
) {
    let current_text = ui.get_markdown_text();
    let anchor = ui.get_editor_anchor_pos() as usize;
    let cursor = ui.get_editor_cursor_pos() as usize;

    let plan = plan_formatting(current_text.as_str(), anchor, cursor, kind);

    let ui_weak = ui.as_weak();
    let undo_stack_clone = Arc::clone(undo_stack);
    let redo_stack_clone = Arc::clone(redo_stack);
    let is_prog_clone = Arc::clone(is_programmatic);

    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_focus_editor();

            is_prog_clone.store(true, Ordering::SeqCst);

            let action = match plan {
                FormattingPlan::Wrap {
                    start,
                    end,
                    prefix,
                    suffix,
                    original_anchor,
                    original_cursor,
                } => {
                    let clean_prefix = markdown::expand_tabs(&prefix, 4);
                    let clean_suffix = markdown::expand_tabs(&suffix, 4);

                    let steps = if !clean_suffix.is_empty() {
                        // 1. Move cursor to end (no selection)
                        ui.invoke_set_editor_selection(end as i32, end as i32);
                        // 2. Dispatch suffix
                        dispatch_text_insert(ui.window(), clean_suffix.as_str());
                        2
                    } else {
                        1
                    };

                    // Move cursor to start (no selection)
                    ui.invoke_set_editor_selection(start as i32, start as i32);
                    // Dispatch prefix
                    dispatch_text_insert(ui.window(), clean_prefix.as_str());

                    // Select inner wrapped text
                    let inner_start = start + clean_prefix.len();
                    let inner_end = end + clean_prefix.len();
                    let (sel_anchor, sel_cursor) = if original_anchor <= original_cursor {
                        (inner_start, inner_end)
                    } else {
                        (inner_end, inner_start)
                    };
                    ui.invoke_set_editor_selection(sel_anchor as i32, sel_cursor as i32);

                    UndoAction {
                        steps,
                        restore_anchor: original_anchor,
                        restore_cursor: original_cursor,
                        redo_anchor: sel_anchor,
                        redo_cursor: sel_cursor,
                    }
                }
                FormattingPlan::Insert {
                    pos,
                    text,
                    target_cursor,
                    original_anchor,
                    original_cursor,
                } => {
                    let clean_text = markdown::expand_tabs(&text, 4);
                    ui.invoke_set_editor_selection(pos as i32, pos as i32);
                    dispatch_text_insert(ui.window(), clean_text.as_str());

                    let final_cursor = target_cursor.unwrap_or(pos + clean_text.len());
                    ui.invoke_set_editor_selection(final_cursor as i32, final_cursor as i32);

                    UndoAction {
                        steps: 1,
                        restore_anchor: original_anchor,
                        restore_cursor: original_cursor,
                        redo_anchor: final_cursor,
                        redo_cursor: final_cursor,
                    }
                }
                FormattingPlan::Replace {
                    start,
                    end,
                    text,
                    original_anchor,
                    original_cursor,
                } => {
                    let clean_text = markdown::expand_tabs(&text, 4);
                    ui.invoke_set_editor_selection(start as i32, end as i32);
                    dispatch_text_insert(ui.window(), clean_text.as_str());

                    let final_anchor = start;
                    let final_cursor = start + clean_text.len();
                    ui.invoke_set_editor_selection(final_anchor as i32, final_cursor as i32);

                    UndoAction {
                        steps: 2,
                        restore_anchor: original_anchor,
                        restore_cursor: original_cursor,
                        redo_anchor: final_anchor,
                        redo_cursor: final_cursor,
                    }
                }
            };

            is_prog_clone.store(false, Ordering::SeqCst);

            if let Ok(mut stack) = undo_stack_clone.lock() {
                stack.push(action);
            }
            if let Ok(mut rstack) = redo_stack_clone.lock() {
                rstack.clear();
            }

            ui.set_is_modified(true);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_formatting_bold_selection() {
        let text = "Hello world!";
        let (start, end, insert, cursor) = prepare_formatting(text, 6, 11, "bold");
        assert_eq!(start, 6);
        assert_eq!(end, 11);
        assert_eq!(insert, "**world**");
        assert_eq!(cursor, None);
    }

    #[test]
    fn test_prepare_formatting_bold_inside_word() {
        let text = "Hello world!";
        let (start, end, insert, cursor) = prepare_formatting(text, 8, 8, "bold");
        assert_eq!(start, 6);
        assert_eq!(end, 11);
        assert_eq!(insert, "**world**");
        assert_eq!(cursor, None);
    }

    #[test]
    fn test_prepare_formatting_bold_inside_cyrillic_word() {
        let text = "Привет мир!";
        // "Привет" is 12 bytes in UTF-8. Offset 4 is between 'р' (2 bytes) and 'и' (2 bytes)
        let (start, end, insert, cursor) = prepare_formatting(text, 4, 4, "bold");
        assert_eq!(start, 0);
        assert_eq!(end, 12);
        assert_eq!(insert, "**Привет**");
        assert_eq!(cursor, None);
    }

    #[test]
    fn test_prepare_formatting_bold_no_selection_empty() {
        let text = "";
        let (start, end, insert, cursor) = prepare_formatting(text, 0, 0, "bold");
        assert_eq!(start, 0);
        assert_eq!(end, 0);
        assert_eq!(insert, "****");
        assert_eq!(cursor, Some(2));
    }

    #[test]
    fn test_prepare_formatting_bold_no_selection_at_space() {
        let text = "Hello world";
        let (start, end, insert, cursor) = prepare_formatting(text, 5, 5, "bold");
        assert_eq!(start, 5);
        assert_eq!(end, 5);
        assert_eq!(insert, "****");
        assert_eq!(cursor, Some(7));
    }

    #[test]
    fn test_prepare_formatting_italic_and_strikethrough() {
        let text = "test word";
        let (s, e, ins, cur) = prepare_formatting(text, 5, 9, "italic");
        assert_eq!((s, e, ins.as_str(), cur), (5, 9, "*word*", None));

        let (s, e, ins, cur) = prepare_formatting(text, 0, 0, "italic");
        assert_eq!((s, e, ins.as_str(), cur), (0, 0, "**", Some(1)));

        let (s, e, ins, cur) = prepare_formatting(text, 5, 9, "strikethrough");
        assert_eq!((s, e, ins.as_str(), cur), (5, 9, "~~word~~", None));

        let (s, e, ins, cur) = prepare_formatting(text, 0, 0, "strikethrough");
        assert_eq!((s, e, ins.as_str(), cur), (0, 0, "~~~~", Some(2)));
    }

    #[test]
    fn test_prepare_formatting_headings() {
        let text = "My Title";
        let (s, e, ins, cur) = prepare_formatting(text, 0, 8, "h1");
        assert_eq!((s, e, ins.as_str(), cur), (0, 8, "# My Title", None));

        let (s, e, ins, cur) = prepare_formatting(text, 0, 8, "h2");
        assert_eq!((s, e, ins.as_str(), cur), (0, 8, "## My Title", None));

        let text_mid = "prefix";
        let (s, e, ins, cur) = prepare_formatting(text_mid, 6, 6, "h1");
        assert_eq!((s, e, ins.as_str(), cur), (6, 6, "\n# ", None));
    }

    #[test]
    fn test_prepare_formatting_code_block() {
        let code = "fn main() {\n    println!();\n}";
        let (_s, _e, ins, cur) = prepare_formatting(code, 0, code.len(), "code_block");
        assert_eq!(ins, format!("```\n{code}\n```\n"));
        assert_eq!(cur, None);

        let (_s, _e, ins, cur) = prepare_formatting("", 0, 0, "code_block");
        assert_eq!(ins, "```\n\n```\n");
        assert_eq!(cur, Some(4));
    }

    #[test]
    fn test_prepare_formatting_multiline_lists_and_quotes() {
        let lines = "line 1\nline 2";
        let (_s, _e, ins, _cur) = prepare_formatting(lines, 0, lines.len(), "blockquote");
        assert_eq!(ins, "> line 1\n> line 2");

        let (_s, _e, ins, _cur) = prepare_formatting(lines, 0, lines.len(), "task_list");
        assert_eq!(ins, "- [ ] line 1\n- [ ] line 2");
    }

    #[test]
    fn test_prepare_formatting_table_and_footnote() {
        let text = "intro";
        let (_s, _e, ins, _cur) = prepare_formatting(text, 5, 5, "table");
        assert!(ins.starts_with("\n| Header 1 | Header 2 |"));

        let (_s, _e, ins, _cur) = prepare_formatting(text, 0, 5, "footnote");
        assert_eq!(ins, "[^1]: intro\n");
    }

    #[test]
    fn test_generate_markdown_table_custom_dimensions() {
        let table_1x1 = generate_markdown_table(1, 1);
        assert_eq!(table_1x1, "| Header 1 |\n|---|\n| Cell 1 |\n\n");

        let table_3x2 = generate_markdown_table(3, 2);
        assert_eq!(
            table_3x2,
            "| Header 1 | Header 2 | Header 3 |\n|---|---|---|\n| Cell 1 | Cell 2 | Cell 3 |\n| Cell 4 | Cell 5 | Cell 6 |\n\n"
        );

        let (_s, _e, ins, _cur) = prepare_formatting("text", 4, 4, "table:3:2");
        assert!(ins.starts_with("\n| Header 1 | Header 2 | Header 3 |"));
        assert!(ins.contains("| Cell 6 |"));
    }

    #[test]
    fn test_prepare_formatting_utf8_boundary_safety() {
        let text = "Привет";
        // Byte offset 1 is inside 2-byte 'П' (0..2), safe clamping moves it down to 0 (start of string)
        let (s, e, ins, cur) = prepare_formatting(text, 1, 1, "bold");
        assert_eq!(s, 0);
        assert_eq!(e, 0);
        assert_eq!(ins, "****");
        assert_eq!(cur, Some(2));

        // Byte offset 3 is inside 2-byte 'р' (2..4), safe clamping moves it down to 2 (between 'П' and 'р')
        // Here cursor is inside the word, so it expands to whole word "Привет" (0..12)
        let (s2, e2, ins2, cur2) = prepare_formatting(text, 3, 3, "bold");
        assert_eq!(s2, 0);
        assert_eq!(e2, 12);
        assert_eq!(ins2, "**Привет**");
        assert_eq!(cur2, None);
    }

    #[test]
    fn test_plan_formatting_wrap_selection() {
        let text = "Hello world!";
        // Selecting "world" (6..11)
        let plan = plan_formatting(text, 6, 11, "bold");
        assert_eq!(
            plan,
            FormattingPlan::Wrap {
                start: 6,
                end: 11,
                prefix: "**".into(),
                suffix: "**".into(),
                original_anchor: 6,
                original_cursor: 11,
            }
        );

        // Reverse selection (11..6)
        let plan_rev = plan_formatting(text, 11, 6, "italic");
        assert_eq!(
            plan_rev,
            FormattingPlan::Wrap {
                start: 6,
                end: 11,
                prefix: "*".into(),
                suffix: "*".into(),
                original_anchor: 11,
                original_cursor: 6,
            }
        );

        // Strikethrough
        let plan_strike = plan_formatting(text, 6, 11, "strikethrough");
        assert_eq!(
            plan_strike,
            FormattingPlan::Wrap {
                start: 6,
                end: 11,
                prefix: "~~".into(),
                suffix: "~~".into(),
                original_anchor: 6,
                original_cursor: 11,
            }
        );
    }

    #[test]
    fn test_plan_formatting_headings_and_quotes() {
        let text = "My Title";
        let plan_h1 = plan_formatting(text, 0, 8, "h1");
        assert_eq!(
            plan_h1,
            FormattingPlan::Wrap {
                start: 0,
                end: 8,
                prefix: "# ".into(),
                suffix: "".into(),
                original_anchor: 0,
                original_cursor: 8,
            }
        );

        let plan_single_quote = plan_formatting("single line", 0, 11, "blockquote");
        assert_eq!(
            plan_single_quote,
            FormattingPlan::Wrap {
                start: 0,
                end: 11,
                prefix: "> ".into(),
                suffix: "".into(),
                original_anchor: 0,
                original_cursor: 11,
            }
        );

        let plan_multi_quote = plan_formatting("line 1\nline 2", 0, 13, "blockquote");
        assert_eq!(
            plan_multi_quote,
            FormattingPlan::Replace {
                start: 0,
                end: 13,
                text: "> line 1\n> line 2".into(),
                original_anchor: 0,
                original_cursor: 13,
            }
        );
    }

    #[test]
    fn test_plan_formatting_insert_without_selection() {
        let plan_empty_bold = plan_formatting("", 0, 0, "bold");
        assert_eq!(
            plan_empty_bold,
            FormattingPlan::Insert {
                pos: 0,
                text: "****".into(),
                target_cursor: Some(2),
                original_anchor: 0,
                original_cursor: 0,
            }
        );

        let plan_empty_code = plan_formatting("", 0, 0, "code_block");
        assert_eq!(
            plan_empty_code,
            FormattingPlan::Insert {
                pos: 0,
                text: "```\n\n```\n".into(),
                target_cursor: Some(4),
                original_anchor: 0,
                original_cursor: 0,
            }
        );
    }

    #[test]
    fn test_modifier_keys_conversion() {
        use slint::platform::Key;
        let keys = [
            Key::Control,
            Key::ControlR,
            Key::Shift,
            Key::ShiftR,
            Key::Alt,
            Key::AltGr,
            Key::Meta,
            Key::MetaR,
        ];
        for k in keys {
            let ch: char = k.into();
            assert!(ch.is_control());
            let s: slint::SharedString = k.into();
            assert_eq!(s.len(), 1);
        }
    }
}


