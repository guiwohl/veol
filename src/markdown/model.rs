#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub spans: Vec<Span>,
    pub task: Option<bool>,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub cells: Vec<Vec<Span>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontmatterKind {
    Yaml,
    Toml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Frontmatter {
        kind: FrontmatterKind,
        pairs: Vec<(String, String)>,
    },
    Heading {
        level: u8,
        spans: Vec<Span>,
        anchor: String,
        source_line: Option<usize>,
    },
    Paragraph {
        spans: Vec<Span>,
    },
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    MermaidBlock {
        source: String,
    },
    Quote {
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Table {
        headers: TableRow,
        alignments: Vec<Alignment>,
        rows: Vec<TableRow>,
    },
    Rule,
    Footnote {
        label: String,
        blocks: Vec<Block>,
    },
}

pub fn heading_anchor(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_dash = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for low in ch.to_lowercase() {
                out.push(low);
            }
            prev_dash = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_simple_lowercase() {
        assert_eq!(heading_anchor("Hello"), "hello");
    }

    #[test]
    fn anchor_spaces_become_hyphens() {
        assert_eq!(heading_anchor("Hello World"), "hello-world");
    }

    #[test]
    fn anchor_strips_punctuation() {
        assert_eq!(heading_anchor("What's up?!"), "whats-up");
    }

    #[test]
    fn anchor_collapses_repeats() {
        assert_eq!(heading_anchor("a   b__c"), "a-b-c");
    }

    #[test]
    fn anchor_trims_trailing_dash() {
        assert_eq!(heading_anchor("Trailing!!!"), "trailing");
    }

    #[test]
    fn anchor_empty_for_no_alphanumerics() {
        assert_eq!(heading_anchor("!!!"), "");
    }

    #[test]
    fn anchor_keeps_digits() {
        assert_eq!(heading_anchor("Section 2.1"), "section-21");
    }
}
