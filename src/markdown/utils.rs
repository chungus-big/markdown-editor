//! Utility functions for Markdown text processing.

/// Expands tab characters into spaces based on standard tab stops.
pub fn expand_tabs(s: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(s.len());
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            result.push('\n');
        }
        let mut col = 0;
        for ch in line.chars() {
            if ch == '\t' {
                let spaces = tab_width - (col % tab_width);
                for _ in 0..spaces {
                    result.push(' ');
                }
                col += spaces;
            } else {
                result.push(ch);
                col += 1;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tabs() {
        assert_eq!(expand_tabs("\tx", 4), "    x");
        assert_eq!(expand_tabs("a\tb", 4), "a   b");
        assert_eq!(expand_tabs("1234\t5", 4), "1234    5");
        assert_eq!(expand_tabs("line1\n\tline2", 4), "line1\n    line2");
    }
}
