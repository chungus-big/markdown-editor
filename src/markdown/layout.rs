//! Calculation and estimation of preview layout geometry for Slint UI.

use super::types::{BlockType, MarkdownBlock};

/// Estimates the rendered height of a single markdown block in logical pixels.
#[allow(dead_code)]
pub fn estimate_block_height(b: &MarkdownBlock, word_wrap: bool) -> f32 {
    estimate_block_height_for_width(b, word_wrap, 700.0)
}

/// Estimates the rendered height of a single markdown block in logical pixels given the available preview pane width.
pub fn estimate_block_height_for_width(b: &MarkdownBlock, word_wrap: bool, preview_width: f32) -> f32 {
    let preview_w = if preview_width < 50.0 { 700.0 } else { preview_width };
    let block_avail_w = (preview_w - 32.0).max(50.0);

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
            let inner_w = (block_avail_w - 12.0).max(30.0);
            let cpl = (inner_w / (font_size * 0.58)).max(1.0);
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
            let inner_w = (block_avail_w - 24.0).max(40.0);
            let cpl = (inner_w / 7.8).max(1.0);
            let lines = if word_wrap {
                let mut wrapped_lines = 0.0;
                for line in b.content.lines() {
                    let len = line.chars().count() as f32;
                    wrapped_lines += (len / cpl).ceil().max(1.0);
                }
                wrapped_lines.max(explicit_lines)
            } else {
                explicit_lines
            };
            24.0 + lang_header + lines * line_h + 12.0
        }
        Some(BlockType::Table) => {
            let metrics = compute_table_layout_metrics(&b.rows);
            let num_cols = metrics.columns.len();
            if num_cols == 0 || b.rows.is_empty() {
                return 24.0;
            }
            let actual_table_w = if word_wrap {
                metrics.table_natural_width.min(block_avail_w.max(100.0))
            } else {
                metrics.table_natural_width
            };

            let col_widths: Vec<f32> = metrics
                .columns
                .iter()
                .map(|col| {
                    if !word_wrap {
                        col.natural_width
                    } else if actual_table_w < metrics.table_min_width {
                        col.min_width * (actual_table_w / metrics.table_min_width.max(1.0))
                    } else if actual_table_w < metrics.table_mid_width {
                        let t = (actual_table_w - metrics.table_min_width)
                            / (metrics.table_mid_width - metrics.table_min_width).max(1.0);
                        col.min_width + (col.mid_width - col.min_width) * t
                    } else {
                        let t = ((actual_table_w - metrics.table_mid_width)
                            / (metrics.table_natural_width - metrics.table_mid_width).max(1.0))
                        .min(1.0);
                        col.min_width + (col.natural_width - col.mid_width) * t
                    }
                })
                .collect();

            let mut table_h: f32 = 0.0;
            for (row_idx, row) in b.rows.iter().enumerate() {
                if row_idx > 0 {
                    table_h += 1.0;
                }
                let mut row_h: f32 = 32.0;
                for (col_idx, cell) in row.cells.iter().enumerate() {
                    let col_w = col_widths.get(col_idx).copied().unwrap_or(50.0);
                    let cell_usable_w = (col_w - 24.0).max(20.0);
                    let text_w = measure_cell_text_width(cell);
                    let lines = if word_wrap && text_w > cell_usable_w {
                        (text_w / cell_usable_w).ceil().max(1.0)
                    } else {
                        1.0
                    };
                    let cell_h = 16.0 + lines * 18.0;
                    if cell_h > row_h {
                        row_h = cell_h;
                    }
                }
                table_h += row_h;
            }
            table_h + 12.0
        }
        Some(BlockType::Separator) => 1.0 + 12.0,
        Some(BlockType::Task) => {
            let pad_left = if b.level > 0 {
                b.level as f32 * 20.0 + 6.0
            } else {
                6.0
            };
            let prefix_w = 32.0 + if !b.extra.is_empty() { 32.0 } else { 0.0 };
            let inner_w = (block_avail_w - pad_left - 6.0 - prefix_w).max(40.0);
            let cpl = (inner_w / 8.0).max(1.0);
            let char_count = b.content.chars().count() as f32;
            let lines = if word_wrap {
                (char_count / cpl).ceil().max(1.0)
            } else {
                1.0
            };
            (lines * 19.0).max(24.0) + 12.0
        }
        Some(BlockType::Quote) => {
            let inner_w = (block_avail_w - 38.0).max(40.0);
            let cpl = (inner_w / 8.0).max(1.0);
            let explicit_lines = b.content.lines().count().max(1) as f32;
            let lines = if word_wrap {
                let mut total = 0.0;
                for line in b.content.lines() {
                    let len = line.chars().count() as f32;
                    total += (len / cpl).ceil().max(1.0);
                }
                total.max(explicit_lines)
            } else {
                explicit_lines
            };
            12.0 + lines * 19.0 + 12.0
        }
        Some(BlockType::Footnote) => {
            let inner_w = (block_avail_w - 60.0).max(40.0);
            let cpl = (inner_w / 8.0).max(1.0);
            let char_count = b.content.chars().count() as f32;
            let lines = if word_wrap {
                (char_count / cpl).ceil().max(1.0)
            } else {
                1.0
            };
            12.0 + lines * 19.0 + 12.0
        }
        _ => {
            // paragraph
            let pad_left = if b.level > 0 {
                b.level as f32 * 20.0 + 6.0
            } else {
                6.0
            };
            let inner_w = (block_avail_w - pad_left - 6.0).max(40.0);
            let cpl = (inner_w / 8.0).max(1.0);
            let explicit_lines = b.content.lines().count().max(1) as f32;
            let lines = if word_wrap {
                let mut total = 0.0;
                for line in b.content.lines() {
                    let len = line.chars().count() as f32;
                    total += (len / cpl).ceil().max(1.0);
                }
                total.max(explicit_lines)
            } else {
                explicit_lines
            };
            lines * 19.0 + 12.0
        }
    }
}

/// Estimates the total rendered height of the preview blocks in logical pixels.
pub fn estimate_preview_height(blocks: &[MarkdownBlock], word_wrap: bool) -> f32 {
    estimate_preview_height_for_width(blocks, word_wrap, 700.0)
}

/// Estimates the total rendered height of the preview blocks in logical pixels given the available preview pane width.
pub fn estimate_preview_height_for_width(blocks: &[MarkdownBlock], word_wrap: bool, width: f32) -> f32 {
    if blocks.is_empty() {
        return 0.0;
    }
    let eff_width = if width < 50.0 { 700.0 } else { width };
    let mut total: f32 = 12.0 + 48.0; // padding top (12px) + padding bottom (48px)
    for b in blocks {
        total += estimate_block_height_for_width(b, word_wrap, eff_width);
    }
    if word_wrap {
        total += 24.0; // safety buffer to ensure bottom content is never clipped on narrow wrapping
    }
    total
}

/// Calculates the estimated vertical Y offset in logical pixels from the top of the preview
/// for each footnote label referenced in the document.
pub fn calculate_footnote_offsets(
    blocks: &[MarkdownBlock],
    word_wrap: bool,
) -> std::collections::HashMap<String, f32> {
    calculate_footnote_offsets_for_width(blocks, word_wrap, 700.0)
}

/// Calculates the estimated vertical Y offset in logical pixels from the top of the preview
/// for each footnote label referenced in the document given the preview width.
pub fn calculate_footnote_offsets_for_width(
    blocks: &[MarkdownBlock],
    word_wrap: bool,
    width: f32,
) -> std::collections::HashMap<String, f32> {
    let mut offsets = std::collections::HashMap::new();
    let mut current_y: f32 = 12.0;
    let eff_width = if width < 50.0 { 700.0 } else { width };
    for b in blocks {
        let block_h = estimate_block_height_for_width(b, word_wrap, eff_width);
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

    #[test]
    fn test_estimate_preview_height_width_adaptation() {
        let sample_blocks = parse_markdown(crate::system::SAMPLE_MARKDOWN);
        let h_wide = estimate_preview_height_for_width(&sample_blocks, true, 800.0);
        let h_medium = estimate_preview_height_for_width(&sample_blocks, true, 500.0);
        let h_narrow = estimate_preview_height_for_width(&sample_blocks, true, 250.0);

        // Narrow preview must have significantly taller estimated height due to word wrapping
        assert!(h_narrow > h_medium, "narrow height ({h_narrow}) must be > medium height ({h_medium})");
        assert!(h_medium >= h_wide, "medium height ({h_medium}) must be >= wide height ({h_wide})");

        // When word_wrap is false, width should not cause wrapping expansions
        let h_no_wrap_narrow = estimate_preview_height_for_width(&sample_blocks, false, 250.0);
        let h_no_wrap_wide = estimate_preview_height_for_width(&sample_blocks, false, 800.0);
        assert_eq!(h_no_wrap_narrow, h_no_wrap_wide);
    }
}
