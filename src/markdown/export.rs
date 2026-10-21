//! Conversion of Markdown to HTML and plain text for clipboard export.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Converts Markdown text into a self-contained HTML fragment.
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html = String::with_capacity(markdown.len() * 2);
    let mut in_table_head = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => html.push_str("<p>"),
                Tag::Heading { level, .. } => {
                    let h = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    html.push_str(&format!("<h{}>", h));
                }
                Tag::BlockQuote(_) => html.push_str("<blockquote>"),
                Tag::CodeBlock(kind) => match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                        html.push_str(&format!(
                            "<pre><code class=\"language-{}\">",
                            html_escape(&lang)
                        ));
                    }
                    _ => html.push_str("<pre><code>"),
                },
                Tag::List(Some(start)) => html.push_str(&format!("<ol start=\"{}\">", start)),
                Tag::List(None) => html.push_str("<ul>"),
                Tag::Item => html.push_str("<li>"),
                Tag::FootnoteDefinition(label) => {
                    html.push_str(&format!(
                        "<div class=\"footnote\" id=\"fn-{}\"><p><sup>[{}]</sup> ",
                        html_escape(&label),
                        html_escape(&label)
                    ));
                }
                Tag::Table(_) => html.push_str("<table>\n"),
                Tag::TableHead => {
                    in_table_head = true;
                    html.push_str("<thead><tr>");
                }
                Tag::TableRow => html.push_str("<tr>"),
                Tag::TableCell => {
                    if in_table_head {
                        html.push_str("<th>");
                    } else {
                        html.push_str("<td>");
                    }
                }
                Tag::Emphasis => html.push_str("<em>"),
                Tag::Strong => html.push_str("<strong>"),
                Tag::Strikethrough => html.push_str("<del>"),
                Tag::Link {
                    dest_url, title, ..
                } => {
                    if title.is_empty() {
                        html.push_str(&format!("<a href=\"{}\">", html_escape(&dest_url)));
                    } else {
                        html.push_str(&format!(
                            "<a href=\"{}\" title=\"{}\">",
                            html_escape(&dest_url),
                            html_escape(&title)
                        ));
                    }
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    if title.is_empty() {
                        html.push_str(&format!("<img src=\"{}\" alt=\"", html_escape(&dest_url)));
                    } else {
                        html.push_str(&format!(
                            "<img src=\"{}\" title=\"{}\" alt=\"",
                            html_escape(&dest_url),
                            html_escape(&title)
                        ));
                    }
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => html.push_str("</p>\n"),
                TagEnd::Heading(level) => {
                    let h = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    html.push_str(&format!("</h{}>\n", h));
                }
                TagEnd::BlockQuote(_) => html.push_str("</blockquote>\n"),
                TagEnd::CodeBlock => html.push_str("</code></pre>\n"),
                TagEnd::List(true) => html.push_str("</ol>\n"),
                TagEnd::List(false) => html.push_str("</ul>\n"),
                TagEnd::Item => html.push_str("</li>\n"),
                TagEnd::FootnoteDefinition => html.push_str("</p></div>\n"),
                TagEnd::Table => html.push_str("</table>\n"),
                TagEnd::TableHead => {
                    in_table_head = false;
                    html.push_str("</tr></thead>\n<tbody>\n");
                }
                TagEnd::TableRow => html.push_str("</tr>\n"),
                TagEnd::TableCell => {
                    if in_table_head {
                        html.push_str("</th>");
                    } else {
                        html.push_str("</td>");
                    }
                }
                TagEnd::Emphasis => html.push_str("</em>"),
                TagEnd::Strong => html.push_str("</strong>"),
                TagEnd::Strikethrough => html.push_str("</del>"),
                TagEnd::Link => html.push_str("</a>"),
                TagEnd::Image => html.push_str("\">"),
                _ => {}
            },
            Event::Text(t) => html.push_str(&html_escape(&t)),
            Event::Code(c) => {
                html.push_str(&format!("<code>{}</code>", html_escape(&c)));
            }
            Event::Html(h) | Event::InlineHtml(h) => html.push_str(&h),
            Event::FootnoteReference(label) => {
                html.push_str(&format!(
                    "<sup><a href=\"#fn-{}\">[{}]</a></sup>",
                    html_escape(&label),
                    html_escape(&label)
                ));
            }
            Event::SoftBreak => html.push('\n'),
            Event::HardBreak => html.push_str("<br>\n"),
            Event::Rule => html.push_str("<hr>\n"),
            Event::TaskListMarker(checked) => {
                if checked {
                    html.push_str("<input type=\"checkbox\" checked disabled> ");
                } else {
                    html.push_str("<input type=\"checkbox\" disabled> ");
                }
            }
            Event::InlineMath(m) => {
                html.push_str(&format!("${}$", html_escape(&m)));
            }
            Event::DisplayMath(m) => {
                html.push_str(&format!("<div class=\"math display\">$${}$$</div>\n", html_escape(&m)));
            }
        }
    }

    if html.contains("<tbody>")
        && !html.ends_with("</tbody>\n</table>\n")
        && html.contains("</table>")
    {
        html = html.replace("</table>", "</tbody>\n</table>");
    }

    html
}

/// Converts Markdown text into stripped plain text.
pub fn markdown_to_plain_text(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut text = String::with_capacity(markdown.len());
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    for event in parser {
        match event {
            Event::Text(t) => {
                if t.contains('\u{02b2}') {
                    text.push_str(&t.replace('\u{02b2}', ""));
                } else {
                    text.push_str(&t);
                }
            }
            Event::Code(c) => {
                if c.contains('\u{02b2}') {
                    text.push_str(&c.replace('\u{02b2}', ""));
                } else {
                    text.push_str(&c);
                }
            }
            Event::Start(Tag::Paragraph) => {
                if !text.is_empty() && !text.ends_with("\n\n") && !text.ends_with(": ") {
                    if text.ends_with('\n') {
                        text.push('\n');
                    } else {
                        text.push_str("\n\n");
                    }
                }
            }
            Event::End(TagEnd::Paragraph) => {
                text.push('\n');
            }
            Event::Start(Tag::Heading { .. }) => {
                if !text.is_empty() && !text.ends_with("\n\n") {
                    text.push_str("\n\n");
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                text.push('\n');
            }
            Event::Start(Tag::BlockQuote(_)) => {
                if !text.is_empty() && !text.ends_with("\n\n") {
                    if text.ends_with('\n') {
                        text.push('\n');
                    } else {
                        text.push_str("\n\n");
                    }
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::Start(Tag::Table(_)) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::End(TagEnd::Table) => {
                text.push('\n');
            }
            Event::Start(Tag::TableRow) | Event::Start(Tag::TableHead) => {
                text.push_str("| ");
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                if text.ends_with(' ') {
                    text.pop();
                }
                text.push('\n');
            }
            Event::End(TagEnd::TableCell) => {
                text.push_str(" | ");
            }
            Event::Start(Tag::List(first_num)) => {
                list_stack.push(first_num);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                if let Some(Some(num)) = list_stack.last_mut() {
                    text.push_str(&format!("{}. ", num));
                    *num += 1;
                } else {
                    text.push_str("• ");
                }
            }
            Event::TaskListMarker(checked) => {
                if text.ends_with("• ") {
                    text.truncate(text.len() - "• ".len());
                }
                if checked {
                    text.push_str("[x] ");
                } else {
                    text.push_str("[ ] ");
                }
            }
            Event::End(TagEnd::Item) => {
                text.push('\n');
            }
            Event::Start(Tag::CodeBlock(_)) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                text.push('\n');
            }
            Event::Rule => {
                text.push_str("\n---\n");
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
            }
            Event::Start(Tag::FootnoteDefinition(label)) => {
                if !text.is_empty() && !text.ends_with("\n\n") {
                    if text.ends_with('\n') {
                        text.push('\n');
                    } else {
                        text.push_str("\n\n");
                    }
                }
                text.push_str(&format!("[{}]: ", label));
            }
            Event::FootnoteReference(label) => {
                text.push_str(&format!("[{}]", label));
            }
            _ => {}
        }
    }

    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html_formatting() {
        let md = "# Title\n\nThis is **bold** and *italic* and `code`.\n\n```rust\nfn main() {}\n```";
        let html = markdown_to_html(md);
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<pre><code class=\"language-rust\">"));
        assert!(html.contains("fn main() {}"));
        assert!(html.contains("</code></pre>"));
    }

    #[test]
    fn test_markdown_to_html_gfm_tables_and_tasks() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n\n- [ ] Task 1\n- [x] Task 2";
        let html = markdown_to_html(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>A</th>"));
        assert!(html.contains("<td>1</td>"));
        assert!(html.contains("<input type=\"checkbox\" disabled>"));
        assert!(html.contains("<input type=\"checkbox\" checked disabled>"));
    }

    #[test]
    fn test_markdown_to_plain_text() {
        let md = "# Heading 1\n\nParagraph with **bold** text and [link](https://example.com).\n\n- [x] Done";
        let plain = markdown_to_plain_text(md);
        assert!(plain.contains("Heading 1"));
        assert!(plain.contains("Paragraph with bold text"));
        assert!(plain.contains("[x] Done"));
        assert!(!plain.contains("# Heading"));
        assert!(!plain.contains("**bold**"));

        // Single block cases:
        let p = markdown_to_plain_text("Paragraph with **bold**, *italic*, ~~strikethrough~~ and `code`");
        assert_eq!(p, "Paragraph with bold, italic, strikethrough and code");

        let h = markdown_to_plain_text("Heading with **bold** and [link](url)");
        assert_eq!(h, "Heading with bold and link");

        let t = markdown_to_plain_text("- [x] Done task with **bold**");
        assert_eq!(t, "[x] Done task with bold");

        let fn_note = markdown_to_plain_text("[^note]: Footnote text with **bold**");
        assert_eq!(fn_note, "[note]: Footnote text with bold");

        let tbl = markdown_to_plain_text("| H1 | H2 |\n| :--- | :--- |\n| C1 | C2 |");
        assert_eq!(tbl, "| H1 | H2 |\n| C1 | C2 |");

        let anchor = markdown_to_plain_text("<font color=\"transparent\">\u{02b2}</font>`-warnaserror`");
        assert_eq!(anchor, "-warnaserror");

        let ordered = markdown_to_plain_text("1. First with **bold**\n2. Second");
        assert_eq!(ordered, "1. First with bold\n2. Second");
    }
}

