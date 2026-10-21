//! Calculation and estimation of preview layout geometry for Slint UI.

use super::types::{BlockType, MarkdownBlock};

/// Estimates the rendered height of a single markdown block in logical pixels.
pub fn estimate_block_height(b: &MarkdownBlock, word_wrap: bool) -> f32 {
    match b.typed() {
        Some(BlockType::Heading) => {
            let pad = match b.level {
                1 => 12.0 + 4.0,
                2 => 8.0 + 4.0,
                _ => 4.0 + 4.0,
            };
            let font_size = match b.level {
                1 => 24.0,
                2 => 20.0,
                3 => 17.0,
                _ => 15.0,
            };
            let line_h = font_size * 1.35;
            let char_count = b.content.chars().count() as f32;
            let cpl = match b.level {
                1 => 36.0,
                2 => 44.0,
                3 => 52.0,
                _ => 60.0,
            };
            let lines = if word_wrap {
                (char_count / cpl).ceil().max(1.0)
            } else {
                1.0
            };
            pad + lines * line_h + 12.0
        }
        Some(BlockType::Code) => {
            let explicit_lines = b.content.lines().count().max(1) as f32;
            let lang_header = if !b.extra.is_empty() { 20.0 } else { 0.0 };
            let line_h = 19.0;
            let lines = if word_wrap {
                let mut wrapped_lines = 0.0;
                for line in b.content.lines() {
                    let len = line.chars().count() as f32;
                    wrapped_lines += (len / 85.0).ceil().max(1.0);
                }
                wrapped_lines.max(explicit_lines)
            } else {
                explicit_lines
            };
            24.0 + lang_header + lines * line_h + 12.0
        }
        Some(BlockType::Table) => {
            let row_count = b.rows.len() as f32;
            row_count * 32.0 + 12.0 + 12.0
        }
        Some(BlockType::Separator) => 1.0 + 12.0,
        Some(BlockType::Task) => {
            let char_count = b.content.chars().count() as f32;
            let lines = if word_wrap {
                (char_count / 85.0).ceil().max(1.0)
            } else {
                1.0
            };
            (lines * 19.0).max(24.0) + 12.0
        }
        Some(BlockType::Quote) => {
            let char_count = b.content.chars().count() as f32;
            let explicit_lines = b.content.lines().count().max(1) as f32;
            let lines = if word_wrap {
                (char_count / 85.0).ceil().max(explicit_lines)
            } else {
                explicit_lines
            };
            12.0 + lines * 19.0 + 12.0
        }
        Some(BlockType::Footnote) => {
            let char_count = b.content.chars().count() as f32;
            let lines = if word_wrap {
                (char_count / 85.0).ceil().max(1.0)
            } else {
                1.0
            };
            12.0 + lines * 19.0 + 12.0
        }
        _ => {
            // paragraph
            let char_count = b.content.chars().count() as f32;
            let explicit_lines = b.content.lines().count().max(1) as f32;
            let lines = if word_wrap {
                (char_count / 85.0).ceil().max(explicit_lines)
            } else {
                explicit_lines
            };
            lines * 19.0 + 12.0
        }
    }
}

/// Estimates the total rendered height of the preview blocks in logical pixels.
pub fn estimate_preview_height(blocks: &[MarkdownBlock], word_wrap: bool) -> f32 {
    let mut total: f32 = 12.0 + 48.0; // padding top (12px) + padding bottom (48px)
    for b in blocks {
        total += estimate_block_height(b, word_wrap);
    }
    total
}

/// Calculates the estimated vertical Y offset in logical pixels from the top of the preview
/// for each footnote label referenced in the document.
pub fn calculate_footnote_offsets(
    blocks: &[MarkdownBlock],
    word_wrap: bool,
) -> std::collections::HashMap<String, f32> {
    let mut offsets = std::collections::HashMap::new();
    let mut current_y: f32 = 12.0;
    for b in blocks {
        let block_h = estimate_block_height(b, word_wrap);
        let mut search_str = b.content.as_str();
        while let Some(pos) = search_str.find("#fn-") {
            let rest = &search_str[pos + 4..];
            let end = rest.find(')').unwrap_or(rest.len());
            let label = &rest[..end];
            if !label.is_empty() {
                // Keep the topmost reference occurrence for each footnote label
                offsets.entry(label.to_string()).or_insert(current_y);
            }
            search_str = &rest[end..];
        }
        current_y += block_h;
    }
    offsets
}

/// Strips inline Markdown and HTML tags from a table cell to estimate its plain text measurement width.
pub fn strip_markdown_for_measurement(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch == '>' {
            in_tag = false;
            continue;
        }
        if in_tag {
            continue;
        }
        if ch == '\u{02b2}' {
            // Invisible Latin script anchor
            continue;
        }
        if ch == '[' {
            continue;
        }
        if ch == ']' {
            if chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                for inner in chars.by_ref() {
                    if inner == ')' {
                        break;
                    }
                }
            }
            continue;
        }
        if ch == '*' || ch == '_' || ch == '~' || ch == '`' {
            continue;
        }
        out.push(ch);
    }
    out
}

/// Estimates character display width for logical layout sizing at font-size 13px.
fn char_display_width(ch: char) -> f32 {
    if ch.is_ascii() {
        if ch == ' ' || ch == '.' || ch == ',' || ch == ':' || ch == ';' || ch == '!' || ch == '|' || ch == 'i' || ch == 'l' || ch == '\'' || ch == '`' {
            4.5
        } else if ch == 'm' || ch == 'w' || ch == 'M' || ch == 'W' || ch == '@' || ch == '#' || ch == '%' {
            10.5
        } else if ch.is_ascii_uppercase() {
            9.0
        } else {
            7.8
        }
    } else {
        match ch as u32 {
            // Cyrillic
            0x0400..=0x04FF => {
                match ch {
                    'ж' | 'ш' | 'щ' | 'ю' | 'м' | 'ф' | 'Ж' | 'Ш' | 'Щ' | 'Ю' | 'М' | 'Ф' => 10.5,
                    'і' | 'ї' | 'ь' | 'ъ' => 6.5,
                    _ if ch.is_uppercase() => 9.5,
                    _ => 8.2,
                }
            }
            // CJK
            0x4E00..=0x9FFF | 0x3000..=0x303F => 13.0,
            _ => 8.0,
        }
    }
}

/// Measures plain text width of a table cell in logical pixels (accounting for multiline breaks).
pub fn measure_cell_text_width(text: &str) -> f32 {
    let normalized = text
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let mut max_line_w: f32 = 0.0;
    for line in normalized.lines() {
        let clean = strip_markdown_for_measurement(line);
        let w: f32 = clean.chars().map(char_display_width).sum();
        if w > max_line_w {
            max_line_w = w;
        }
    }
    max_line_w
}

/// Measures plain text width of the longest unbreakable word or segment in a table cell in logical pixels.
pub fn measure_cell_min_word_width(text: &str) -> f32 {
    let normalized = text
        .replace("<br>", " ")
        .replace("<br/>", " ")
        .replace("<br />", " ");
    let clean = strip_markdown_for_measurement(&normalized);

    let mut max_segment_w: f32 = 0.0;
    let mut current_segment_w: f32 = 0.0;

    for ch in clean.chars() {
        if ch.is_whitespace() {
            if current_segment_w > max_segment_w {
                max_segment_w = current_segment_w;
            }
            current_segment_w = 0.0;
        } else {
            let code = ch as u32;
            let is_cjk = matches!(code, 0x4E00..=0x9FFF | 0x3000..=0x303F);
            if is_cjk {
                if current_segment_w > max_segment_w {
                    max_segment_w = current_segment_w;
                }
                current_segment_w = 0.0;
                let cjk_w = char_display_width(ch);
                if cjk_w > max_segment_w {
                    max_segment_w = cjk_w;
                }
            } else {
                current_segment_w += char_display_width(ch);
            }
        }
    }
    if current_segment_w > max_segment_w {
        max_segment_w = current_segment_w;
    }

    max_segment_w
}

/// Layout metrics for an individual column in a Markdown table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableColumnMetrics {
    /// Natural column width with all text on a single line + paddings.
    pub natural_width: f32,
    /// Balanced intermediate column width where wide columns have absorbed primary compression.
    pub mid_width: f32,
    /// Minimum column width based on the longest unbreakable word + paddings.
    pub min_width: f32,
    /// Proportional share of compressible space among all columns in the table.
    pub flex_ratio: f32,
}

/// Aggregated layout metrics for a Markdown table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableLayoutMetrics {
    /// Layout metrics for each column.
    pub columns: Vec<TableColumnMetrics>,
    /// Total natural width of the table.
    pub table_natural_width: f32,
    /// Balanced intermediate width of the table.
    pub table_mid_width: f32,
    /// Total minimum width of the table required for unbreakable words.
    pub table_min_width: f32,
}

/// Computes synchronized column widths, minimum unbreakable widths, intermediate balanced widths,
/// and flex ratios for a Markdown table across all its rows to support adaptive word wrapping.
pub fn compute_table_layout_metrics(rows: &[super::types::TableRow]) -> TableLayoutMetrics {
    if rows.is_empty() {
        return TableLayoutMetrics {
            columns: Vec::new(),
            table_natural_width: 0.0,
            table_mid_width: 0.0,
            table_min_width: 0.0,
        };
    }
    let num_cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return TableLayoutMetrics {
            columns: Vec::new(),
            table_natural_width: 0.0,
            table_mid_width: 0.0,
            table_min_width: 0.0,
        };
    }

    let cell_padding = 24.0; // 12px left + 12px right padding
    let mut cols_raw = Vec::with_capacity(num_cols);

    for col_idx in 0..num_cols {
        let mut max_cell_w: f32 = 0.0;
        let mut max_word_w: f32 = 0.0;
        for r in rows {
            if let Some(cell_text) = r.cells.get(col_idx) {
                let factor = if r.is_header { 1.15 } else { 1.0 };
                let full_w = measure_cell_text_width(cell_text) * factor;
                if full_w > max_cell_w {
                    max_cell_w = full_w;
                }
                let word_w = measure_cell_min_word_width(cell_text) * factor;
                if word_w > max_word_w {
                    max_word_w = word_w;
                }
            }
        }
        let col_min_w = (max_word_w + cell_padding).max(36.0).ceil();
        let col_natural_w = (max_cell_w + cell_padding).max(col_min_w).ceil();
        cols_raw.push((col_natural_w, col_min_w));
    }

    let table_natural_width: f32 = cols_raw.iter().map(|(nat, _)| *nat).sum();
    let table_min_width: f32 = cols_raw.iter().map(|(_, min)| *min).sum();

    // Average natural width per column
    let avg_natural_w = if num_cols > 0 {
        table_natural_width / num_cols as f32
    } else {
        0.0
    };
    // Columns wider than 1.15x average are considered wide and absorb primary compression
    let wide_threshold = avg_natural_w * 1.15;

    let mut cols_with_mid = Vec::with_capacity(num_cols);
    for &(natural_w, min_w) in &cols_raw {
        let mid_w = if natural_w > wide_threshold && natural_w > min_w {
            wide_threshold.max(min_w).min(natural_w).ceil()
        } else {
            natural_w
        };
        cols_with_mid.push((natural_w, mid_w, min_w));
    }

    let table_mid_width: f32 = cols_with_mid.iter().map(|(_, mid, _)| *mid).sum();

    // Compressible space per column: natural_width - min_width
    let total_compressible: f32 = cols_with_mid.iter().map(|(nat, _, min)| (nat - min).max(0.0)).sum();

    let columns = cols_with_mid
        .into_iter()
        .map(|(natural_width, mid_width, min_width)| {
            let compressible = (natural_width - min_width).max(0.0);
            let flex_ratio = if total_compressible > 0.0 {
                compressible / total_compressible
            } else if table_natural_width > 0.0 {
                natural_width / table_natural_width
            } else if num_cols > 0 {
                1.0 / num_cols as f32
            } else {
                1.0
            };
            TableColumnMetrics {
                natural_width,
                mid_width,
                min_width,
                flex_ratio,
            }
        })
        .collect();

    TableLayoutMetrics {
        columns,
        table_natural_width,
        table_mid_width,
        table_min_width,
    }
}

/// Computes synchronized column widths for a Markdown table across all its rows.
#[allow(dead_code)]
pub fn compute_table_column_widths(rows: &[super::types::TableRow]) -> Vec<f32> {
    compute_table_layout_metrics(rows)
        .columns
        .into_iter()
        .map(|c| c.natural_width)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parser::parse_markdown;

    #[test]
    fn test_estimate_preview_height() {
        let md = "# Title\n\nSome paragraph.\n\n```rust\nfn main() {}\n```";
        let blocks = parse_markdown(md);
        let h_wrap = estimate_preview_height(&blocks, true);
        let h_no_wrap = estimate_preview_height(&blocks, false);
        assert!(h_wrap > 100.0);
        assert!(h_no_wrap > 100.0);

        let sample_blocks = parse_markdown(crate::system::SAMPLE_MARKDOWN);
        let sample_wrap = estimate_preview_height(&sample_blocks, true);
        let sample_no_wrap = estimate_preview_height(&sample_blocks, false);
        assert!((3000.0..=4200.0).contains(&sample_wrap));
        assert!((2500.0..=3500.0).contains(&sample_no_wrap));
        assert!(sample_wrap >= sample_no_wrap);
    }

    #[test]
    fn test_calculate_footnote_offsets() {
        let md = "# Heading 1\n\nFirst paragraph with [^note1] reference.\n\nSecond paragraph with [^note2] reference.\n\n[^note1]: First footnote.\n[^note2]: Second footnote.";
        let blocks = parse_markdown(md);
        let offsets = calculate_footnote_offsets(&blocks, true);
        assert!(offsets.contains_key("note1"));
        assert!(offsets.contains_key("note2"));
        let y1 = offsets["note1"];
        let y2 = offsets["note2"];
        assert!(y1 > 0.0);
        assert!(y2 > y1);
    }

    #[test]
    fn test_strip_markdown_for_measurement() {
        assert_eq!(strip_markdown_for_measurement("**bold**"), "bold");
        assert_eq!(strip_markdown_for_measurement("*italic*"), "italic");
        assert_eq!(strip_markdown_for_measurement("~~strikethrough~~"), "strikethrough");
        assert_eq!(strip_markdown_for_measurement("`code`"), "code");
        assert_eq!(strip_markdown_for_measurement("[Link Text](https://example.com)"), "Link Text");
        assert_eq!(
            strip_markdown_for_measurement("<font color=\"transparent\">\u{02b2}</font>`TextInput`"),
            "TextInput"
        );
    }

    #[test]
    fn test_compute_table_column_widths() {
        let md = "| Short | Longer Column Header | C |\n|---|---|---|\n| 1 | Some description text here | X |";
        let blocks = parse_markdown(md);
        assert_eq!(blocks[0].block_type, "table");
        let widths = compute_table_column_widths(&blocks[0].rows);
        assert_eq!(widths.len(), 3);
        // Column 1 has "Longer Column Header" and "Some description text here" which should be wider than Column 0 and Column 2
        assert!(widths[1] > widths[0]);
        assert!(widths[1] > widths[2]);
        assert!(widths[0] >= 36.0);
        assert!(widths[2] >= 36.0);
    }

    #[test]
    fn test_measure_cell_min_word_width() {
        // Words separated by space
        let w_space = measure_cell_min_word_width("Hello world");
        let w_hello = measure_cell_text_width("Hello");
        let w_world = measure_cell_text_width("world");
        assert_eq!(w_space, w_hello.max(w_world));

        // Single long word
        let w_single = measure_cell_min_word_width("SuperLongWordWithoutSpaces");
        assert_eq!(w_single, measure_cell_text_width("SuperLongWordWithoutSpaces"));

        // HTML break tags
        let w_br = measure_cell_min_word_width("First<br>Second");
        assert_eq!(w_br, measure_cell_text_width("Second"));

        // CJK characters break individually
        let w_cjk = measure_cell_min_word_width("中文测试");
        assert_eq!(w_cjk, 13.0);
    }

    #[test]
    fn test_compute_table_layout_metrics() {
        let md = "| ID | Very Long Column Description With Many Many Words |\n|---|---|\n| 1 | Another sentence that can easily wrap across multiple lines |";
        let blocks = parse_markdown(md);
        assert_eq!(blocks[0].block_type, "table");
        let metrics = compute_table_layout_metrics(&blocks[0].rows);

        assert_eq!(metrics.columns.len(), 2);
        assert!(metrics.table_natural_width >= metrics.table_mid_width);
        assert!(metrics.table_mid_width >= metrics.table_min_width);

        // Column 0 has only short single words ("ID", "1")
        // Column 1 has a long sentence with many words that can wrap
        let col0 = &metrics.columns[0];
        let col1 = &metrics.columns[1];

        // Column 1 should have much more compressible capacity and higher flex_ratio than Column 0
        assert!(col1.flex_ratio > col0.flex_ratio);
        assert!((col0.flex_ratio + col1.flex_ratio - 1.0).abs() < 1e-4);

        // Compact column 0 does not shrink in phase 1 (mid == natural)
        assert_eq!(col0.mid_width, col0.natural_width);
        // Wide column 1 shrinks in phase 1 (mid < natural)
        assert!(col1.mid_width < col1.natural_width);

        // Column 0 min_width should be sufficient for "ID"
        assert!(col0.min_width >= 36.0);
        assert!(col0.natural_width >= col0.min_width);
    }
}
