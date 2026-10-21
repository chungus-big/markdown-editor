//! Preview adapters, statistic calculations, and Slint Model converters.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::{file_browser, markdown};
use crate::{AppWindow, FileTreeItemData, MarkdownBlockData, TableCellData, TableRowData};

/// Converts parsed AST `MarkdownBlock` elements into a Slint UI `ModelRc<MarkdownBlockData>`.
pub fn convert_blocks_to_slint(
    blocks: Vec<markdown::MarkdownBlock>,
) -> slint::ModelRc<MarkdownBlockData> {
    let slint_blocks: Vec<MarkdownBlockData> = blocks
        .into_iter()
        .map(|b| {
            let b_typed = b.typed();
            let styled_content = if b_typed == Some(markdown::BlockType::Heading) {
                let bold = format!("**{}**", b.content);
                slint::StyledText::from_markdown(&bold)
                    .unwrap_or_else(|_| slint::StyledText::from_plain_text(&b.content))
            } else {
                slint::StyledText::from_markdown(&b.content)
                    .unwrap_or_else(|_| slint::StyledText::from_plain_text(&b.content))
            };

            let table_metrics = markdown::compute_table_layout_metrics(&b.rows);
            let num_cols = table_metrics.columns.len();
            let total_natural_width = table_metrics.table_natural_width;
            let total_min_width = table_metrics.table_min_width;

            let rows: Vec<TableRowData> = b
                .rows
                .into_iter()
                .map(|r| {
                    let mut cells: Vec<TableCellData> = Vec::with_capacity(num_cols);
                    for col_idx in 0..num_cols {
                        let raw_cell = r.cells.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                        let styled_text = if r.is_header {
                            let bold = format!("**{}**", raw_cell);
                            slint::StyledText::from_markdown(&bold)
                                .unwrap_or_else(|_| slint::StyledText::from_plain_text(raw_cell))
                        } else {
                            slint::StyledText::from_markdown(raw_cell)
                                .unwrap_or_else(|_| slint::StyledText::from_plain_text(raw_cell))
                        };
                        let col_metric = table_metrics.columns.get(col_idx);
                        let width = col_metric.map(|m| m.natural_width).unwrap_or(80.0);
                        let mid_width = col_metric.map(|m| m.mid_width).unwrap_or(width);
                        let min_width = col_metric.map(|m| m.min_width).unwrap_or(40.0);
                        let flex_ratio = col_metric.map(|m| m.flex_ratio).unwrap_or(0.0);
                        let ratio = if total_natural_width > 0.0 {
                            width / total_natural_width
                        } else if num_cols > 0 {
                            1.0 / num_cols as f32
                        } else {
                            1.0
                        };
                        let align = b.table_alignments.get(col_idx).copied().unwrap_or(0);
                        cells.push(TableCellData {
                            text: styled_text,
                            width,
                            mid_width,
                            min_width,
                            flex_ratio,
                            ratio,
                            align,
                        });
                    }
                    TableRowData {
                        cells: Rc::new(slint::VecModel::from(cells)).into(),
                        is_header: r.is_header,
                    }
                })
                .collect();

            let clipboard_content = match b_typed {
                Some(markdown::BlockType::Task) => {
                    let mark = if b.is_checked { "[x]" } else { "[ ]" };
                    if b.extra.is_empty() {
                        format!("- {} {}", mark, b.content)
                    } else {
                        format!("{}{} {}", b.extra, mark, b.content)
                    }
                }
                Some(markdown::BlockType::Footnote) => {
                    let label = if !b.footnote_label.is_empty() {
                        &b.footnote_label
                    } else {
                        &b.extra
                    };
                    let clean_content = if let Some(idx) = b.content.rfind(" [↩](#fnref-") {
                        &b.content[..idx]
                    } else {
                        &b.content
                    };
                    format!("[^{}]: {}", label, clean_content)
                }
                _ => b.content.clone(),
            };

            let has_links = b.content.contains("](")
                || b.content.contains("[[")
                || b.content.contains("http://")
                || b.content.contains("https://")
                || b.content.contains("mailto:");

            MarkdownBlockData {
                block_type: b.block_type.into(),
                level: b.level,
                content: clipboard_content.into(),
                styled_content,
                extra: b.extra.into(),
                footnote_label: b.footnote_label.into(),
                is_checked: b.is_checked,
                rows: Rc::new(slint::VecModel::from(rows)).into(),
                table_natural_width: total_natural_width,
                table_mid_width: table_metrics.table_mid_width,
                table_min_width: total_min_width,
                has_links,
            }
        })
        .collect();

    Rc::new(slint::VecModel::from(slint_blocks)).into()
}

/// Tracks footnote return scroll positions and estimated offsets for bidirectional navigation.
#[derive(Default, Debug, Clone)]
pub struct FootnoteNavigationState {
    /// Exact recorded scroll positions (content-y in px) when clicking footnote references in preview.
    pub recorded_positions: std::collections::HashMap<String, f32>,
    /// Estimated Y offsets from preview top with word wrap enabled.
    pub estimated_offsets_wrap: std::collections::HashMap<String, f32>,
    /// Estimated Y offsets from preview top with word wrap disabled.
    pub estimated_offsets_no_wrap: std::collections::HashMap<String, f32>,
    /// Maps display index ("1", "2") to footnote label ("note", etc.).
    pub index_to_label: std::collections::HashMap<String, String>,
}

impl FootnoteNavigationState {
    /// Updates estimated offsets and index mapping based on current parsed blocks.
    pub fn update(&mut self, blocks: &[markdown::MarkdownBlock]) {
        self.estimated_offsets_wrap = markdown::calculate_footnote_offsets(blocks, true);
        self.estimated_offsets_no_wrap = markdown::calculate_footnote_offsets(blocks, false);
        self.index_to_label.clear();
        for b in blocks {
            if b.typed() == Some(markdown::BlockType::Footnote)
                && !b.extra.is_empty()
                && !b.footnote_label.is_empty()
            {
                self.index_to_label
                    .insert(b.extra.clone(), b.footnote_label.clone());
            }
        }
    }

    /// Records the current scroll position for the clicked footnote label.
    pub fn save_pos(&mut self, url: &str, y: f32) {
        let label = url.strip_prefix("#fn-").unwrap_or(url);
        self.recorded_positions.insert(label.to_string(), y);
    }

    /// Clears any recorded scroll positions (e.g. on loading a new document).
    pub fn clear_recorded(&mut self) {
        self.recorded_positions.clear();
    }

    /// Resolves target scroll position for returning from a footnote.
    pub fn get_pos(&self, url: &str, word_wrap: bool) -> Option<f32> {
        let label = url.strip_prefix("#fnref-").unwrap_or(url);
        // 1. Check exact recorded position for label
        if let Some(&y) = self.recorded_positions.get(label) {
            return Some(y);
        }
        // 2. Check if label is a numeric index mapped to the original label
        if let Some(mapped_label) = self.index_to_label.get(label)
            && let Some(&y) = self.recorded_positions.get(mapped_label)
        {
            return Some(y);
        }
        // 3. Fallback to estimated block offset
        let offsets = if word_wrap {
            &self.estimated_offsets_wrap
        } else {
            &self.estimated_offsets_no_wrap
        };
        if let Some(&offset) = offsets.get(label) {
            return Some(-offset);
        }
        if let Some(mapped_label) = self.index_to_label.get(label)
            && let Some(&offset) = offsets.get(mapped_label)
        {
            return Some(-offset);
        }
        None
    }
}

/// Calculates line, word, and character counts for the given markdown text.
pub fn calculate_stats(text: &str) -> (i32, i32, i32) {
    let lines = text.lines().count() as i32;
    let words = text.split_whitespace().count() as i32;
    let chars = text.chars().count() as i32;
    (lines, words, chars)
}

/// Re-parses the markdown text, calculates preview height, and updates the preview UI panel.
pub fn update_preview(
    ui: &AppWindow,
    text: &str,
    footnote_nav: &mut FootnoteNavigationState,
) {
    let blocks = markdown::parse_markdown(text);
    let h_wrap = markdown::estimate_preview_height(&blocks, true);
    let h_no_wrap = markdown::estimate_preview_height(&blocks, false);
    ui.set_estimated_preview_height(h_wrap);
    ui.set_estimated_preview_height_no_wrap(h_no_wrap);

    footnote_nav.update(&blocks);

    let model = convert_blocks_to_slint(blocks);
    ui.set_preview_blocks(model);

    let (lines, words, chars) = calculate_stats(text);
    ui.set_line_count(lines);
    ui.set_word_count(words);
    ui.set_char_count(chars);
}

/// Scans the file tree and updates the sidebar file list model in Slint UI.
pub fn update_file_tree(
    ui: &AppWindow,
    dir: &Path,
    active_path: Option<&Path>,
    expanded_dirs: &HashSet<PathBuf>,
) {
    let (tree_items, total_count) =
        file_browser::scan_directory_tree(dir, active_path, expanded_dirs);

    let items: Vec<FileTreeItemData> = tree_items
        .into_iter()
        .map(|item| FileTreeItemData {
            name: item.name.into(),
            path: item.path.into(),
            is_dir: item.is_dir,
            is_expanded: item.is_expanded,
            depth: item.depth,
            is_active: item.is_active,
        })
        .collect();

    ui.set_file_tree(Rc::new(slint::VecModel::from(items)).into());
    ui.set_file_count(total_count);
    ui.set_current_directory_name(file_browser::get_directory_name(dir).into());
    ui.set_current_directory_path(dir.to_string_lossy().to_string().into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::Model;

    #[test]
    fn test_calculate_stats() {
        let (lines, words, chars) = calculate_stats("Hello world\nSecond line");
        assert_eq!(lines, 2);
        assert_eq!(words, 4);
        assert_eq!(chars, 23);
    }

    #[test]
    fn test_update_preview_sample_document() {
        let content = crate::system::SAMPLE_MARKDOWN;
        let blocks = markdown::parse_markdown(content);
        let model = convert_blocks_to_slint(blocks.clone());
        assert!(model.row_count() > 0);

        let quotes: Vec<_> = blocks.iter().filter(|b| b.block_type == "quote").collect();
        assert!(quotes.len() >= 3);
        assert!(quotes
            .iter()
            .any(|q| q.content.contains("Simplicity is prerequisite for reliability.")));
    }

    #[test]
    fn test_table_preview_model() {
        let content = crate::system::SAMPLE_MARKDOWN;
        let blocks = markdown::parse_markdown(content);
        let table_block = blocks
            .iter()
            .find(|b| b.block_type == "table")
            .expect("sample.md should have a table");
        assert_eq!(table_block.rows.len(), 5);
        assert_eq!(table_block.rows[0].cells.len(), 5);

        let model = convert_blocks_to_slint(vec![table_block.clone()]);
        let block_data = model.row_data(0).unwrap();
        assert_eq!(block_data.rows.row_count(), 5);

        let header_row = block_data.rows.row_data(0).unwrap();
        assert!(header_row.is_header);
        assert_eq!(header_row.cells.row_count(), 5);

        let row_1 = block_data.rows.row_data(1).unwrap();
        assert!(!row_1.is_header);
        assert_eq!(row_1.cells.row_count(), 5);

        assert!(block_data.table_natural_width > 0.0);
        assert!(block_data.table_mid_width > 0.0);
        assert!(block_data.table_min_width > 0.0);
        assert!(block_data.table_natural_width >= block_data.table_mid_width);
        assert!(block_data.table_mid_width >= block_data.table_min_width);

        // Every column should have the exact same width, mid_width, min_width, flex_ratio and ratio across rows!
        let mut total_ratio = 0.0;
        let mut total_flex_ratio = 0.0;
        for col in 0..5 {
            let header_cell = header_row.cells.row_data(col).unwrap();
            let row1_cell = row_1.cells.row_data(col).unwrap();
            assert_eq!(
                header_cell.width, row1_cell.width,
                "Column {col} width mismatch between header and data row!"
            );
            assert_eq!(
                header_cell.mid_width, row1_cell.mid_width,
                "Column {col} mid_width mismatch between header and data row!"
            );
            assert_eq!(
                header_cell.min_width, row1_cell.min_width,
                "Column {col} min_width mismatch between header and data row!"
            );
            assert!(
                (header_cell.flex_ratio - row1_cell.flex_ratio).abs() < 1e-4,
                "Column {col} flex_ratio mismatch between header and data row!"
            );
            assert!(
                (header_cell.ratio - row1_cell.ratio).abs() < 1e-4,
                "Column {col} ratio mismatch between header and data row!"
            );
            assert!(header_cell.ratio > 0.0);
            total_ratio += header_cell.ratio;
            total_flex_ratio += header_cell.flex_ratio;
        }
        assert!(
            (total_ratio - 1.0).abs() < 1e-3,
            "Sum of column ratios should be ~1.0, got {total_ratio}"
        );
        assert!(
            (total_flex_ratio - 1.0).abs() < 1e-3,
            "Sum of column flex_ratios should be ~1.0, got {total_flex_ratio}"
        );
    }

    #[test]
    fn test_footnote_navigation_state() {
        let md = "# Title\n\nText with [^first] and [^second].\n\n[^first]: Def 1\n[^second]: Def 2";
        let blocks = markdown::parse_markdown(md);
        let mut state = FootnoteNavigationState::default();
        state.update(&blocks);

        // Before any click, fallback to estimated block offset
        let fallback_first = state.get_pos("#fnref-first", true);
        assert!(fallback_first.is_some());
        assert!(fallback_first.unwrap() < 0.0);

        // Also works with numeric index "1"
        let fallback_idx1 = state.get_pos("#fnref-1", true);
        assert_eq!(fallback_idx1, fallback_first);

        // When user clicked the footnote reference, exact position is recorded
        state.save_pos("#fn-first", -250.0);
        assert_eq!(state.get_pos("#fnref-first", true), Some(-250.0));
        assert_eq!(state.get_pos("#fnref-1", true), Some(-250.0));

        // Other footnote still uses its estimated offset
        let fallback_second = state.get_pos("#fnref-second", true);
        assert!(fallback_second.is_some());

        // Clear resets recorded positions
        state.clear_recorded();
        assert_eq!(state.get_pos("#fnref-first", true), fallback_first);
    }

    #[test]
    fn test_preview_clipboard_formatting() {
        let md = "- [x] Done task\n- [ ] Pending task\n\n[^note]: Footnote text\n\n> Quote line\n\n| H1 | H2 |\n| --- | --- |\n| C1 | C2 |";
        let blocks = markdown::parse_markdown(md);
        let model = convert_blocks_to_slint(blocks);

        // Task 1: [x] Done task -> "- [x] Done task"
        let t1 = model.row_data(0).unwrap();
        assert_eq!(t1.block_type, "task");
        assert_eq!(t1.content, "- [x] Done task");

        // Task 2: [ ] Pending task -> "- [ ] Pending task"
        let t2 = model.row_data(1).unwrap();
        assert_eq!(t2.block_type, "task");
        assert_eq!(t2.content, "- [ ] Pending task");

        // Quote
        let q = model.row_data(2).unwrap();
        assert_eq!(q.block_type, "quote");
        assert_eq!(q.content, "Quote line");

        // Table
        let tbl = model.row_data(3).unwrap();
        assert_eq!(tbl.block_type, "table");
        assert!(tbl.content.contains("| H1 | H2 |"));
        assert!(tbl.content.contains("| C1 | C2 |"));

        // Separator between body and footnotes
        let sep = model.row_data(4).unwrap();
        assert_eq!(sep.block_type, "separator");

        // Footnote
        let fn_b = model.row_data(5).unwrap();
        assert_eq!(fn_b.block_type, "footnote");
        assert_eq!(fn_b.content, "[^note]: Footnote text");

        // Plain task without links has has_links == false
        assert!(!t1.has_links);
        assert!(!t2.has_links);
        assert!(!q.has_links);

        // Document with links vs without links
        let doc = "Plain text without any link.\n\nParagraph with [Google](https://google.com) link.\n\nParagraph with footnote ref[^note].\n\n[^note]: Note def";
        let doc_blocks = markdown::parse_markdown(doc);
        let doc_model = convert_blocks_to_slint(doc_blocks);
        let p_plain = doc_model.row_data(0).unwrap();
        assert_eq!(p_plain.block_type, "paragraph");
        assert!(!p_plain.has_links);

        let p_link = doc_model.row_data(1).unwrap();
        assert_eq!(p_link.block_type, "paragraph");
        assert!(p_link.has_links);

        let p_fn = doc_model.row_data(2).unwrap();
        assert_eq!(p_fn.block_type, "paragraph");
        assert!(p_fn.has_links);
    }
}
