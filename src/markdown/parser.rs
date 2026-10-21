//! Parser state machine converting CommonMark and GFM into structured preview blocks.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::types::{BlockType, ItemState, ListInfo, MarkdownBlock, TableRow};
use super::utils::expand_tabs;

/// State machine tracking Markdown AST construction from pulldown_cmark events.
#[derive(Default)]
pub struct MarkdownParser {
    blocks: Vec<MarkdownBlock>,
    current_block_type: Option<String>,
    current_level: i32,
    current_extra: String,
    current_text: String,

    list_stack: Vec<ListInfo>,
    item_stack: Vec<ItemState>,

    in_blockquote: usize,
    blockquote_text: String,

    in_footnote: bool,
    footnote_labels: Vec<String>,
    footnote_blocks: Vec<MarkdownBlock>,

    current_rows: Vec<TableRow>,
    current_row_cells: Vec<String>,
    current_cell_text: String,
    current_table_alignments: Vec<i32>,
    in_cell: bool,

    current_link_url: String,
    current_link_text: String,
    in_link: bool,
}

/// Converts an integer to Unicode superscript characters (e.g. 1 -> "¹", 2 -> "²", 12 -> "¹²").
pub fn to_superscript(mut n: usize) -> String {
    if n == 0 {
        return "⁰".to_string();
    }
    let mut digits = Vec::new();
    while n > 0 {
        let d = match n % 10 {
            0 => '⁰',
            1 => '¹',
            2 => '²',
            3 => '³',
            4 => '⁴',
            5 => '⁵',
            6 => '⁶',
            7 => '⁷',
            8 => '⁸',
            9 => '⁹',
            _ => unreachable!(),
        };
        digits.push(d);
        n /= 10;
    }
    digits.into_iter().rev().collect()
}

impl MarkdownParser {
    /// Creates a new uninitialized parser state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends formatted inline text to the active target buffer based on context.
    fn append_inline(&mut self, text: &str) {
        if self.in_link {
            self.current_link_text.push_str(text);
        } else if self.in_cell {
            self.current_cell_text.push_str(text);
        } else if let Some(item) = self.item_stack.last_mut() {
            item.text.push_str(text);
        } else if self.in_blockquote > 0 {
            self.blockquote_text.push_str(text);
        } else {
            self.current_text.push_str(text);
        }
    }

    /// Emits any pending text on the parent list item before a nested list begins.
    fn flush_parent_item_if_needed(&mut self) {
        if let Some(parent_item) = self.item_stack.last_mut() {
            let trimmed = parent_item.text.trim().to_string();
            if !trimmed.is_empty() {
                if parent_item.is_task {
                    let extra = if parent_item.is_ordered {
                        format!("{}. ", parent_item.index)
                    } else {
                        String::new()
                    };
                    let mut block = MarkdownBlock::with_type(BlockType::Task);
                    block.level = parent_item.depth as i32;
                    block.content = trimmed;
                    block.extra = extra;
                    block.is_checked = parent_item.is_checked;
                    self.blocks.push(block);
                } else {
                    let prefix = if parent_item.is_ordered {
                        format!("{}. ", parent_item.index)
                    } else {
                        match parent_item.depth {
                            0 => "• ".to_string(),
                            1 => "◦ ".to_string(),
                            _ => "▪ ".to_string(),
                        }
                    };
                    let mut block = MarkdownBlock::with_type(BlockType::Paragraph);
                    block.level = parent_item.depth as i32;
                    block.content = format!("{}{}", prefix, trimmed);
                    self.blocks.push(block);
                }
                parent_item.text.clear();
            }
        }
    }

    /// Handles opening tags.
    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::List(first_num) => {
                self.flush_parent_item_if_needed();
                self.list_stack.push(ListInfo {
                    is_ordered: first_num.is_some(),
                    next_index: first_num.unwrap_or(1),
                });
            }
            Tag::Item => {
                let depth = self.list_stack.len().saturating_sub(1);
                let (is_ordered, index) = if let Some(last) = self.list_stack.last_mut() {
                    let idx = last.next_index;
                    last.next_index += 1;
                    (last.is_ordered, idx)
                } else {
                    (false, 0)
                };
                self.item_stack.push(ItemState {
                    depth,
                    is_ordered,
                    index,
                    is_task: false,
                    is_checked: false,
                    text: String::new(),
                });
            }
            Tag::Heading { level, .. } => {
                self.current_block_type = Some("heading".to_string());
                self.current_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                self.current_text.clear();
            }
            Tag::Paragraph => {
                if self.item_stack.is_empty() && self.in_blockquote == 0 && !self.in_footnote {
                    self.current_block_type = Some("paragraph".to_string());
                    self.current_text.clear();
                } else if self.in_blockquote > 0 {
                    if !self.blockquote_text.is_empty() && !self.blockquote_text.ends_with("\n\n") {
                        self.blockquote_text.push_str("\n\n");
                    }
                } else if self.in_footnote {
                    if !self.current_text.is_empty() && !self.current_text.ends_with("\n\n") {
                        self.current_text.push_str("\n\n");
                    }
                } else if let Some(item) = self.item_stack.last_mut()
                    && !item.text.is_empty()
                    && !item.text.ends_with('\n')
                {
                    item.text.push('\n');
                }
            }
            Tag::CodeBlock(kind) => {
                self.current_block_type = Some("code".to_string());
                self.current_extra = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                self.current_text.clear();
            }
            Tag::BlockQuote(_) => {
                self.in_blockquote += 1;
                if self.in_blockquote == 1 {
                    self.blockquote_text.clear();
                }
            }
            Tag::FootnoteDefinition(label) => {
                self.in_footnote = true;
                self.current_extra = label.to_string();
                self.current_text.clear();
            }
            Tag::Table(alignments) => {
                self.current_rows.clear();
                self.current_table_alignments = alignments
                    .into_iter()
                    .map(|a| match a {
                        pulldown_cmark::Alignment::Center => 1,
                        pulldown_cmark::Alignment::Right => 2,
                        _ => 0,
                    })
                    .collect();
            }
            Tag::TableHead | Tag::TableRow => {
                self.current_row_cells.clear();
            }
            Tag::TableCell => {
                self.in_cell = true;
                self.current_cell_text.clear();
            }
            Tag::Emphasis => self.append_inline("*"),
            Tag::Strong => self.append_inline("**"),
            Tag::Strikethrough => self.append_inline("~~"),
            Tag::Link { dest_url, .. } => {
                self.in_link = true;
                self.current_link_url = dest_url.to_string();
                self.current_link_text.clear();
            }
            Tag::Image { dest_url, .. } => {
                self.in_link = true;
                self.current_link_url = dest_url.to_string();
                self.current_link_text.clear();
            }
            _ => {}
        }
    }

    /// Handles closing tags.
    fn handle_end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::List(_) => {
                self.list_stack.pop();
            }
            TagEnd::Item => {
                if let Some(item) = self.item_stack.pop() {
                    let trimmed = item.text.trim().to_string();
                    if !trimmed.is_empty() {
                        if item.is_task {
                            let extra = if item.is_ordered {
                                format!("{}. ", item.index)
                            } else {
                                String::new()
                            };
                            let mut block = MarkdownBlock::with_type(BlockType::Task);
                            block.level = item.depth as i32;
                            block.content = trimmed;
                            block.extra = extra;
                            block.is_checked = item.is_checked;
                            self.blocks.push(block);
                        } else {
                            let prefix = if item.is_ordered {
                                format!("{}. ", item.index)
                            } else {
                                match item.depth {
                                    0 => "• ".to_string(),
                                    1 => "◦ ".to_string(),
                                    _ => "▪ ".to_string(),
                                }
                            };
                            let mut block = MarkdownBlock::with_type(BlockType::Paragraph);
                            block.level = item.depth as i32;
                            block.content = format!("{}{}", prefix, trimmed);
                            self.blocks.push(block);
                        }
                    }
                }
            }
            TagEnd::Heading(_) => {
                if let Some(bt) = self.current_block_type.take() {
                    let mut block = MarkdownBlock::new(bt);
                    block.level = self.current_level;
                    block.content = self.current_text.trim().to_string();
                    self.blocks.push(block);
                }
            }
            TagEnd::Paragraph => {
                if self.item_stack.is_empty()
                    && self.in_blockquote == 0
                    && !self.in_footnote
                    && let Some(bt) = self.current_block_type.take()
                {
                    let content = self.current_text.trim().to_string();
                    if !content.is_empty() {
                        let mut block = MarkdownBlock::new(bt);
                        block.content = content;
                        self.blocks.push(block);
                    }
                }
            }
            TagEnd::CodeBlock => {
                if let Some(bt) = self.current_block_type.take() {
                    let unexpanded = self.current_text.trim_end_matches('\n');
                    let content = expand_tabs(unexpanded, 4);
                    let mut block = MarkdownBlock::new(bt);
                    block.content = content;
                    block.extra = std::mem::take(&mut self.current_extra);
                    self.blocks.push(block);
                }
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = self.in_blockquote.saturating_sub(1);
                if self.in_blockquote == 0 {
                    let trimmed = self.blockquote_text.trim().to_string();
                    if !trimmed.is_empty() {
                        let mut block = MarkdownBlock::with_type(BlockType::Quote);
                        block.content = trimmed;
                        self.blocks.push(block);
                    }
                    self.blockquote_text.clear();
                }
            }
            TagEnd::FootnoteDefinition => {
                self.in_footnote = false;
                let mut block = MarkdownBlock::with_type(BlockType::Footnote);
                block.content = self.current_text.trim().to_string();
                let label = std::mem::take(&mut self.current_extra);
                block.extra = label.clone();
                block.footnote_label = label;
                self.footnote_blocks.push(block);
                self.current_text.clear();
            }
            TagEnd::Table => {
                if !self.current_rows.is_empty() {
                    let mut block = MarkdownBlock::with_type(BlockType::Table);
                    let mut table_str = String::new();
                    for row in &self.current_rows {
                        table_str.push_str("| ");
                        table_str.push_str(&row.cells.join(" | "));
                        table_str.push_str(" |\n");
                        if row.is_header {
                            table_str.push_str("| ");
                            let sep: Vec<String> = (0..row.cells.len())
                                .map(|col_idx| {
                                    match self
                                        .current_table_alignments
                                        .get(col_idx)
                                        .copied()
                                        .unwrap_or(0)
                                    {
                                        1 => ":---:".to_string(),
                                        2 => "---:".to_string(),
                                        _ => ":---".to_string(),
                                    }
                                })
                                .collect();
                            table_str.push_str(&sep.join(" | "));
                            table_str.push_str(" |\n");
                        }
                    }
                    block.content = table_str.trim_end().to_string();
                    block.rows = std::mem::take(&mut self.current_rows);
                    block.table_alignments = std::mem::take(&mut self.current_table_alignments);
                    self.blocks.push(block);
                }
            }
            TagEnd::TableHead => {
                self.current_rows.push(TableRow {
                    cells: std::mem::take(&mut self.current_row_cells),
                    is_header: true,
                });
            }
            TagEnd::TableRow => {
                self.current_rows.push(TableRow {
                    cells: std::mem::take(&mut self.current_row_cells),
                    is_header: false,
                });
            }
            TagEnd::TableCell => {
                self.in_cell = false;
                self.current_row_cells
                    .push(self.current_cell_text.trim().to_string());
                self.current_cell_text.clear();
            }
            TagEnd::Emphasis => self.append_inline("*"),
            TagEnd::Strong => self.append_inline("**"),
            TagEnd::Strikethrough => self.append_inline("~~"),
            TagEnd::Link => {
                self.in_link = false;
                let formatted = format!("[{}]({})", self.current_link_text, self.current_link_url);
                self.append_inline(&formatted);
            }
            TagEnd::Image => {
                self.in_link = false;
                let formatted = format!("![{}]({})", self.current_link_text, self.current_link_url);
                self.append_inline(&formatted);
            }
            _ => {}
        }
    }

    /// Consumes the parser and processes all tokens in the input.
    pub fn parse(mut self, input: &str) -> Vec<MarkdownBlock> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(input, options);

        for event in parser {
            match event {
                Event::Start(tag) => self.handle_start_tag(tag),
                Event::End(tag_end) => self.handle_end_tag(tag_end),
                Event::TaskListMarker(checked) => {
                    if let Some(item) = self.item_stack.last_mut() {
                        item.is_task = true;
                        item.is_checked = checked;
                    }
                }
                Event::Rule => {
                    self.blocks.push(MarkdownBlock::with_type(BlockType::Separator));
                }
                Event::Code(code) => {
                    // Slint 1.18.1 Parley itemizer splits inline code spans across script boundaries
                    // when leading symbols (Script::Common) inherit preceding Cyrillic context before Latin characters,
                    // causing Slint to render split capsule backgrounds (e.g. `[-]` and `[warnaserror]`).
                    // Prepending an invisible zero-advance Latin script anchor (`\u{02b2}`) forces Parley to keep
                    // leading punctuation and following Latin characters within a single unified glyph run.
                    let needs_latin_anchor = code
                        .chars()
                        .next()
                        .is_some_and(|c| !c.is_ascii_alphabetic())
                        && code.chars().any(|c| c.is_ascii_alphabetic());
                    let formatted = if needs_latin_anchor {
                        format!("<font color=\"transparent\">\u{02b2}</font>`{}`", code)
                    } else {
                        format!("`{}`", code)
                    };
                    self.append_inline(&formatted);
                }
                Event::FootnoteReference(label) => {
                    let index = if let Some(pos) =
                        self.footnote_labels.iter().position(|l| l == label.as_ref())
                    {
                        pos + 1
                    } else {
                        self.footnote_labels.push(label.to_string());
                        self.footnote_labels.len()
                    };
                    let sup = to_superscript(index);
                    let formatted = format!("[[{sup}]](#fn-{label})");
                    self.append_inline(&formatted);
                }
                Event::Text(t) => self.append_inline(&t),
                Event::SoftBreak => self.append_inline(" "),
                Event::HardBreak => self.append_inline("\n"),
                _ => {}
            }
        }

        if !self.footnote_blocks.is_empty() {
            // Sort footnote definitions according to their appearance order in footnote_labels.
            // Any unreferenced footnote definitions are placed at the end.
            self.footnote_blocks.sort_by_key(|b| {
                self.footnote_labels
                    .iter()
                    .position(|l| l == &b.footnote_label)
                    .unwrap_or(usize::MAX)
            });

            // Re-number footnote blocks to match the in-text reference indices
            // and attach a return link (↩) to navigate back.
            let mut next_unref_idx = self.footnote_labels.len() + 1;
            for block in &mut self.footnote_blocks {
                let (idx, orig_label) = if let Some(pos) =
                    self.footnote_labels.iter().position(|l| l == &block.footnote_label)
                {
                    (pos + 1, block.footnote_label.clone())
                } else {
                    let i = next_unref_idx;
                    next_unref_idx += 1;
                    (i, block.footnote_label.clone())
                };

                block.extra = idx.to_string();
                block.footnote_label = orig_label.clone();
                if !block.content.is_empty() {
                    block.content = format!("{} [↩](#fnref-{})", block.content, orig_label);
                }
            }

            // Ensure a separator divider precedes the footnotes section if needed
            if !self.blocks.is_empty()
                && self.blocks.last().is_some_and(|b| b.block_type != "separator")
            {
                self.blocks.push(MarkdownBlock::with_type(BlockType::Separator));
            }

            self.blocks.append(&mut self.footnote_blocks);
        }

        self.blocks
    }
}

/// Parses CommonMark + GFM into a list of structured Markdown blocks.
pub fn parse_markdown(input: &str) -> Vec<MarkdownBlock> {
    MarkdownParser::new().parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headings() {
        let md = "# Title 1\n## Subtitle 2\n### Section 3";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type, "heading");
        assert_eq!(blocks[0].level, 1);
        assert_eq!(blocks[0].content, "Title 1");

        assert_eq!(blocks[1].level, 2);
        assert_eq!(blocks[1].content, "Subtitle 2");

        assert_eq!(blocks[2].level, 3);
        assert_eq!(blocks[2].content, "Section 3");
    }

    #[test]
    fn test_parse_gfm_tasklists() {
        let md = "- [ ] Task uncompleted\n- [x] Task completed";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type, "task");
        assert!(!blocks[0].is_checked);
        assert_eq!(blocks[0].content, "Task uncompleted");

        assert_eq!(blocks[1].block_type, "task");
        assert!(blocks[1].is_checked);
        assert_eq!(blocks[1].content, "Task completed");
    }

    #[test]
    fn test_parse_ordered_lists() {
        let md = "1. First\n2. Second\n3. Third";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content, "1. First");
        assert_eq!(blocks[1].content, "2. Second");
        assert_eq!(blocks[2].content, "3. Third");
    }

    #[test]
    fn test_parse_ordered_starting_number() {
        let md = "5. Fifth\n6. Sixth";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "5. Fifth");
        assert_eq!(blocks[1].content, "6. Sixth");
    }

    #[test]
    fn test_parse_nested_lists() {
        let md = "1. Parent\n   - Child 1\n   - Child 2\n2. Parent 2";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].content, "1. Parent");
        assert_eq!(blocks[0].level, 0);

        assert_eq!(blocks[1].content, "◦ Child 1");
        assert_eq!(blocks[1].level, 1);

        assert_eq!(blocks[2].content, "◦ Child 2");
        assert_eq!(blocks[2].level, 1);

        assert_eq!(blocks[3].content, "2. Parent 2");
        assert_eq!(blocks[3].level, 0);
    }

    #[test]
    fn test_parse_ordered_task_lists() {
        let md = "1. [ ] Ordered task 1\n2. [x] Ordered task 2";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].block_type, "task");
        assert_eq!(blocks[0].extra, "1. ");
        assert_eq!(blocks[0].content, "Ordered task 1");
        assert!(!blocks[0].is_checked);

        assert_eq!(blocks[1].block_type, "task");
        assert_eq!(blocks[1].extra, "2. ");
        assert_eq!(blocks[1].content, "Ordered task 2");
        assert!(blocks[1].is_checked);
    }

    #[test]
    fn test_parse_blockquotes() {
        let md = "> Single quote line\n>\n> Second quote line";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "quote");
        assert_eq!(blocks[0].content, "Single quote line\n\nSecond quote line");
    }

    #[test]
    fn test_parse_gfm_tables() {
        let md = "| A | B | C |\n| :--- | :---: | ---: |\n| 1 | 2 | 3 |";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "table");
        assert_eq!(blocks[0].rows.len(), 2);
        assert!(blocks[0].rows[0].is_header);
        assert_eq!(blocks[0].rows[0].cells, vec!["A", "B", "C"]);
        assert_eq!(blocks[0].table_alignments, vec![0, 1, 2]);
        assert!(!blocks[0].rows[1].is_header);
        assert_eq!(blocks[0].rows[1].cells, vec!["1", "2", "3"]);
        assert_eq!(
            blocks[0].content,
            "| A | B | C |\n| :--- | :---: | ---: |\n| 1 | 2 | 3 |"
        );
    }

    #[test]
    fn test_parse_gfm_strikethrough_and_code() {
        let md = "This is ~~deleted~~ and `code`";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "paragraph");
        assert_eq!(blocks[0].content, "This is ~~deleted~~ and `code`");
    }

    #[test]
    fn test_parse_inline_code_symbols_anchor() {
        let md = "Флаги: `-warnaserror` и `--flag` и `[Regex]` и `обычный`.";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "paragraph");
        assert!(blocks[0].content.contains("<font color=\"transparent\">\u{02b2}</font>`-warnaserror`"));
        assert!(blocks[0].content.contains("<font color=\"transparent\">\u{02b2}</font>`--flag`"));
        assert!(blocks[0].content.contains("<font color=\"transparent\">\u{02b2}</font>`[Regex]`"));
        assert!(blocks[0].content.contains("`обычный`"));
    }

    #[test]
    fn test_parse_code_block_with_tabs() {
        let md = "```rust\nfn main() {\n\tprintln!(\"hi\");\n}\n```";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "code");
        assert_eq!(blocks[0].extra, "rust");
        assert_eq!(blocks[0].content, "fn main() {\n    println!(\"hi\");\n}");
    }

    #[test]
    fn test_parse_complex_document() {
        let content = include_str!("../sample.md");
        let blocks = parse_markdown(content);
        assert!(!blocks.is_empty());
        assert!(blocks.iter().any(|b| b.block_type == "heading"));
        assert!(blocks.iter().any(|b| b.block_type == "paragraph"));
        assert!(blocks.iter().any(|b| b.block_type == "code"));
        assert!(blocks.iter().any(|b| b.block_type == "quote"));
        assert!(blocks.iter().any(|b| b.block_type == "task"));
        assert!(blocks.iter().any(|b| b.block_type == "table"));
        assert!(blocks.iter().any(|b| b.block_type == "separator"));
        let footnote = blocks.iter().find(|b| b.block_type == "footnote");
        assert!(footnote.is_some());
        let fn_b = footnote.unwrap();
        assert_eq!(fn_b.extra, "1");
        assert!(fn_b.content.contains("Это текст сноски"));
        assert!(fn_b.content.contains("[↩](#fnref-note)"));
    }

    #[test]
    fn test_footnote_parsing_behavior() {
        let md = "Text with [^note] and [^orphan].\n\n[^note]: This is the note.";
        let blocks = parse_markdown(md);
        assert_eq!(blocks[0].block_type, "paragraph");
        assert_eq!(blocks[0].content, "Text with [[¹]](#fn-note) and [^orphan].");

        let fn_b = blocks.iter().find(|b| b.block_type == "footnote").unwrap();
        assert_eq!(fn_b.extra, "1");
        assert_eq!(fn_b.content, "This is the note. [↩](#fnref-note)");
    }

    #[test]
    fn test_multiple_footnotes_ordering() {
        let md = "First [^b] and second [^a].\n\n[^a]: Definition A\n[^b]: Definition B";
        let blocks = parse_markdown(md);
        assert_eq!(blocks[0].content, "First [[¹]](#fn-b) and second [[²]](#fn-a).");

        let fn_blocks: Vec<_> = blocks.iter().filter(|b| b.block_type == "footnote").collect();
        assert_eq!(fn_blocks.len(), 2);
        assert_eq!(fn_blocks[0].extra, "1");
        assert_eq!(fn_blocks[0].content, "Definition B [↩](#fnref-b)");
        assert_eq!(fn_blocks[1].extra, "2");
        assert_eq!(fn_blocks[1].content, "Definition A [↩](#fnref-a)");
    }

    #[test]
    fn test_to_superscript() {
        assert_eq!(to_superscript(1), "¹");
        assert_eq!(to_superscript(2), "²");
        assert_eq!(to_superscript(3), "³");
        assert_eq!(to_superscript(10), "¹⁰");
        assert_eq!(to_superscript(42), "⁴²");
    }
}
