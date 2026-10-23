//! Markdown parsing, AST representation and layout estimation for the Markdown Editor.
//!
//! Provides strict CommonMark + GitHub Flavored Markdown (GFM) support:
//! tables, task lists, strikethrough, footnotes, tabs expansion, code blocks, and blockquotes.

pub mod export;
pub mod layout;
pub mod parser;
pub mod types;
pub mod utils;

#[allow(unused_imports)]
pub use export::{markdown_to_html, markdown_to_plain_text};
#[allow(unused_imports)]
pub use layout::{
    calculate_footnote_offsets, calculate_footnote_offsets_for_width,
    compute_table_column_widths, compute_table_layout_metrics,
    estimate_block_height, estimate_block_height_for_width,
    estimate_preview_height, estimate_preview_height_for_width,
    measure_cell_min_word_width, TableColumnMetrics, TableLayoutMetrics,
};
pub use parser::parse_markdown;
#[allow(unused_imports)]
pub use types::{BlockType, MarkdownBlock, TableRow};
pub use utils::expand_tabs;

