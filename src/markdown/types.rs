//! Data structures representing parsed Markdown elements and blocks.

/// Represents a row in a Markdown table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<String>,
    pub is_header: bool,
}

/// Enumeration of strongly typed Markdown block variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockType {
    Heading,
    Paragraph,
    Code,
    Quote,
    Task,
    Table,
    Separator,
    Footnote,
}

impl BlockType {
    /// Returns the string identifier used by UI preview bindings.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Heading => "heading",
            Self::Paragraph => "paragraph",
            Self::Code => "code",
            Self::Quote => "quote",
            Self::Task => "task",
            Self::Table => "table",
            Self::Separator => "separator",
            Self::Footnote => "footnote",
        }
    }

    /// Converts a string name into a strongly typed `BlockType` variant.
    pub fn from_str_name(name: &str) -> Option<Self> {
        match name {
            "heading" => Some(Self::Heading),
            "paragraph" => Some(Self::Paragraph),
            "code" => Some(Self::Code),
            "quote" => Some(Self::Quote),
            "task" => Some(Self::Task),
            "table" => Some(Self::Table),
            "separator" => Some(Self::Separator),
            "footnote" => Some(Self::Footnote),
            _ => None,
        }
    }
}

/// Represents a parsed block element formatted for UI preview rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownBlock {
    pub block_type: String, // String representation for Slint ModelRc compatibility
    pub level: i32,
    pub content: String,
    pub extra: String,
    pub footnote_label: String,
    pub is_checked: bool,
    pub rows: Vec<TableRow>,
    pub table_alignments: Vec<i32>,
}

impl MarkdownBlock {
    /// Creates a new MarkdownBlock with default empty values.
    pub fn new(block_type: impl Into<String>) -> Self {
        Self {
            block_type: block_type.into(),
            level: 0,
            content: String::new(),
            extra: String::new(),
            footnote_label: String::new(),
            is_checked: false,
            rows: Vec::new(),
            table_alignments: Vec::new(),
        }
    }

    /// Creates a new MarkdownBlock with a strongly typed BlockType.
    pub fn with_type(block_type: BlockType) -> Self {
        Self::new(block_type.as_str())
    }

    /// Returns the strongly typed BlockType variant if recognized.
    pub fn typed(&self) -> Option<BlockType> {
        BlockType::from_str_name(&self.block_type)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ListInfo {
    pub is_ordered: bool,
    pub next_index: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ItemState {
    pub depth: usize,
    pub is_ordered: bool,
    pub index: u64,
    pub is_task: bool,
    pub is_checked: bool,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_type_roundtrip() {
        let types = [
            BlockType::Heading,
            BlockType::Paragraph,
            BlockType::Code,
            BlockType::Quote,
            BlockType::Task,
            BlockType::Table,
            BlockType::Separator,
            BlockType::Footnote,
        ];
        for bt in types {
            assert_eq!(BlockType::from_str_name(bt.as_str()), Some(bt));
            let block = MarkdownBlock::with_type(bt);
            assert_eq!(block.typed(), Some(bt));
        }
        assert_eq!(BlockType::from_str_name("unknown"), None);
    }
}
