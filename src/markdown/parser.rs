use pulldown_cmark::{
    Alignment as PdAlignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
};

use crate::markdown::frontmatter;
use crate::markdown::model::{
    heading_anchor, Alignment, Block, ListItem, Span, SpanStyle, TableRow,
};

pub fn parse(source: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let body = if let Some((fm, rest)) = frontmatter::extract(source) {
        out.push(Block::Frontmatter {
            kind: fm.kind,
            pairs: fm.pairs,
        });
        rest
    } else {
        source
    };

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(body, opts);
    let events: Vec<Event<'_>> = parser.collect();
    let mut walker = Walker::new(&events);
    walker.run(&mut out);
    out
}

fn is_mermaid_info(info: &str) -> bool {
    let head = info.trim().split([':', ' ']).next().unwrap_or("");
    head.eq_ignore_ascii_case("mermaid")
        || head.eq_ignore_ascii_case("mmd")
        || head.eq_ignore_ascii_case("mermaid-js")
}

#[derive(Default)]
struct InlineCtx {
    bold: u32,
    italic: u32,
    strike: u32,
    link: Vec<String>,
}

impl InlineCtx {
    fn style(&self, code: bool) -> SpanStyle {
        SpanStyle {
            bold: self.bold > 0,
            italic: self.italic > 0,
            strike: self.strike > 0,
            code,
            link: self.link.last().cloned(),
        }
    }
}

struct Walker<'a> {
    events: &'a [Event<'a>],
    pos: usize,
}

impl<'a> Walker<'a> {
    fn new(events: &'a [Event<'a>]) -> Self {
        Self { events, pos: 0 }
    }

    fn run(&mut self, out: &mut Vec<Block>) {
        while self.pos < self.events.len() {
            if let Some(block) = self.next_block() {
                out.push(block);
            }
        }
    }

    fn next_block(&mut self) -> Option<Block> {
        let ev = self.events.get(self.pos)?.clone();
        match ev {
            Event::Start(tag) => {
                self.pos += 1;
                Some(self.consume_block(tag))
            }
            Event::Rule => {
                self.pos += 1;
                Some(Block::Rule)
            }
            _ => {
                self.pos += 1;
                None
            }
        }
    }

    fn consume_block(&mut self, tag: Tag<'a>) -> Block {
        match tag {
            Tag::Heading { level, .. } => {
                let mut ctx = InlineCtx::default();
                let spans = self.collect_inline_until(&mut ctx, TagEnd::Heading(level));
                let plain: String = spans.iter().map(|s| s.text.as_str()).collect();
                Block::Heading {
                    level: level as u8,
                    anchor: heading_anchor(&plain),
                    spans,
                    source_line: None,
                }
            }
            Tag::Paragraph => {
                let mut ctx = InlineCtx::default();
                let spans = self.collect_inline_until(&mut ctx, TagEnd::Paragraph);
                Block::Paragraph { spans }
            }
            Tag::CodeBlock(kind) => {
                let (lang, mermaid) = match &kind {
                    CodeBlockKind::Fenced(info) => {
                        let trimmed = info.trim();
                        if is_mermaid_info(trimmed) {
                            (None, true)
                        } else if trimmed.is_empty() {
                            (None, false)
                        } else {
                            (Some(trimmed.to_string()), false)
                        }
                    }
                    CodeBlockKind::Indented => (None, false),
                };
                let mut code = String::new();
                while let Some(ev) = self.events.get(self.pos).cloned() {
                    self.pos += 1;
                    match ev {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(t) => code.push_str(&t),
                        _ => {}
                    }
                }
                if mermaid {
                    let source = code.trim_end_matches('\n').to_string();
                    Block::MermaidBlock { source }
                } else {
                    let code = code.trim_end_matches('\n').to_string();
                    Block::CodeBlock { lang, code }
                }
            }
            Tag::BlockQuote(_) => {
                let mut inner = Vec::new();
                while let Some(ev) = self.events.get(self.pos).cloned() {
                    if matches!(ev, Event::End(TagEnd::BlockQuote(_))) {
                        self.pos += 1;
                        break;
                    }
                    if let Some(block) = self.next_block() {
                        inner.push(block);
                    }
                }
                Block::Quote { blocks: inner }
            }
            Tag::List(start) => {
                let ordered = start.is_some();
                let start_n = start.unwrap_or(1);
                let mut items = Vec::new();
                while let Some(ev) = self.events.get(self.pos).cloned() {
                    match ev {
                        Event::End(TagEnd::List(_)) => {
                            self.pos += 1;
                            break;
                        }
                        Event::Start(Tag::Item) => {
                            self.pos += 1;
                            items.push(self.consume_list_item());
                        }
                        _ => {
                            self.pos += 1;
                        }
                    }
                }
                Block::List {
                    ordered,
                    start: start_n,
                    items,
                }
            }
            Tag::Table(aligns) => {
                let alignments = aligns.into_iter().map(map_alignment).collect();
                let headers = self.consume_table_head();
                let mut rows = Vec::new();
                while let Some(ev) = self.events.get(self.pos).cloned() {
                    match ev {
                        Event::End(TagEnd::Table) => {
                            self.pos += 1;
                            break;
                        }
                        Event::Start(Tag::TableRow) => {
                            self.pos += 1;
                            rows.push(self.consume_table_row(TagEnd::TableRow));
                        }
                        _ => {
                            self.pos += 1;
                        }
                    }
                }
                Block::Table {
                    headers,
                    alignments,
                    rows,
                }
            }
            Tag::FootnoteDefinition(label) => {
                let label = label.into_string();
                let mut blocks = Vec::new();
                while let Some(ev) = self.events.get(self.pos).cloned() {
                    if matches!(ev, Event::End(TagEnd::FootnoteDefinition)) {
                        self.pos += 1;
                        break;
                    }
                    if let Some(block) = self.next_block() {
                        blocks.push(block);
                    }
                }
                Block::Footnote { label, blocks }
            }
            other => {
                let end = other.to_end();
                self.skip_until(end);
                Block::Paragraph { spans: Vec::new() }
            }
        }
    }

    fn consume_list_item(&mut self) -> ListItem {
        let mut spans = Vec::new();
        let mut task = None;
        let mut children = Vec::new();
        let mut ctx = InlineCtx::default();

        while let Some(ev) = self.events.get(self.pos).cloned() {
            match ev {
                Event::End(TagEnd::Item) => {
                    self.pos += 1;
                    break;
                }
                Event::TaskListMarker(done) => {
                    task = Some(done);
                    self.pos += 1;
                }
                Event::Start(Tag::Paragraph) => {
                    self.pos += 1;
                    let p_spans = self.collect_inline_until(&mut ctx, TagEnd::Paragraph);
                    spans.extend(p_spans);
                }
                Event::Start(inner_tag) => {
                    self.pos += 1;
                    children.push(self.consume_block(inner_tag));
                }
                Event::Text(_)
                | Event::Code(_)
                | Event::SoftBreak
                | Event::HardBreak
                | Event::Html(_)
                | Event::InlineHtml(_)
                | Event::FootnoteReference(_) => {
                    let span = self.take_inline_event(&mut ctx);
                    if let Some(s) = span {
                        spans.push(s);
                    }
                }
                Event::Rule => {
                    self.pos += 1;
                    children.push(Block::Rule);
                }
                _ => {
                    self.pos += 1;
                }
            }
        }

        ListItem {
            spans,
            task,
            children,
        }
    }

    fn consume_table_head(&mut self) -> TableRow {
        while let Some(ev) = self.events.get(self.pos).cloned() {
            self.pos += 1;
            if matches!(ev, Event::Start(Tag::TableHead)) {
                break;
            }
        }
        self.consume_table_row(TagEnd::TableHead)
    }

    fn consume_table_row(&mut self, end: TagEnd) -> TableRow {
        let mut cells = Vec::new();
        while let Some(ev) = self.events.get(self.pos).cloned() {
            match ev {
                e if e == Event::End(end) => {
                    self.pos += 1;
                    break;
                }
                Event::Start(Tag::TableCell) => {
                    self.pos += 1;
                    let mut ctx = InlineCtx::default();
                    let cell = self.collect_inline_until(&mut ctx, TagEnd::TableCell);
                    cells.push(cell);
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        TableRow { cells }
    }

    fn collect_inline_until(&mut self, ctx: &mut InlineCtx, end: TagEnd) -> Vec<Span> {
        let mut spans = Vec::new();
        while let Some(ev) = self.events.get(self.pos).cloned() {
            if matches!(&ev, Event::End(e) if *e == end) {
                self.pos += 1;
                break;
            }
            if let Some(span) = self.take_inline_event(ctx) {
                spans.push(span);
            }
        }
        spans
    }

    fn take_inline_event(&mut self, ctx: &mut InlineCtx) -> Option<Span> {
        let ev = self.events.get(self.pos).cloned()?;
        self.pos += 1;
        match ev {
            Event::Text(t) => Some(Span {
                text: t.into_string(),
                style: ctx.style(false),
            }),
            Event::Code(t) => Some(Span {
                text: t.into_string(),
                style: ctx.style(true),
            }),
            Event::Html(t) | Event::InlineHtml(t) => Some(Span {
                text: t.into_string(),
                style: ctx.style(false),
            }),
            Event::SoftBreak => Some(Span {
                text: " ".to_string(),
                style: ctx.style(false),
            }),
            Event::HardBreak => Some(Span {
                text: "\n".to_string(),
                style: ctx.style(false),
            }),
            Event::FootnoteReference(label) => Some(Span {
                text: format!("[^{}]", label.into_string()),
                style: ctx.style(false),
            }),
            Event::Start(Tag::Emphasis) => {
                ctx.italic += 1;
                None
            }
            Event::Start(Tag::Strong) => {
                ctx.bold += 1;
                None
            }
            Event::Start(Tag::Strikethrough) => {
                ctx.strike += 1;
                None
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                ctx.link.push(dest_url.into_string());
                None
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                ctx.link.push(dest_url.into_string());
                None
            }
            Event::End(TagEnd::Emphasis) => {
                if ctx.italic > 0 {
                    ctx.italic -= 1;
                }
                None
            }
            Event::End(TagEnd::Strong) => {
                if ctx.bold > 0 {
                    ctx.bold -= 1;
                }
                None
            }
            Event::End(TagEnd::Strikethrough) => {
                if ctx.strike > 0 {
                    ctx.strike -= 1;
                }
                None
            }
            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => {
                ctx.link.pop();
                None
            }
            _ => None,
        }
    }

    fn skip_until(&mut self, end: TagEnd) {
        let mut depth = 1usize;
        while self.pos < self.events.len() && depth > 0 {
            let ev = self.events[self.pos].clone();
            self.pos += 1;
            match ev {
                Event::Start(t) if t.to_end() == end => depth += 1,
                Event::End(e) if e == end => depth -= 1,
                _ => {}
            }
        }
    }
}

fn map_alignment(a: PdAlignment) -> Alignment {
    match a {
        PdAlignment::None => Alignment::None,
        PdAlignment::Left => Alignment::Left,
        PdAlignment::Center => Alignment::Center,
        PdAlignment::Right => Alignment::Right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::model::FrontmatterKind;

    #[test]
    fn empty_source_yields_no_blocks() {
        assert!(parse("").is_empty());
    }

    #[test]
    fn only_frontmatter_yields_single_frontmatter_block() {
        let blocks = parse("---\ntitle: Hi\n---\n");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Frontmatter { kind, pairs } => {
                assert_eq!(*kind, FrontmatterKind::Yaml);
                assert_eq!(pairs, &vec![("title".into(), "Hi".into())]);
            }
            other => panic!("expected Frontmatter, got {other:?}"),
        }
    }

    #[test]
    fn h1_heading_has_level_one_and_slug_anchor() {
        let blocks = parse("# Hello\n");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::Heading {
                level,
                spans,
                anchor,
                ..
            } => {
                assert_eq!(*level, 1);
                assert_eq!(anchor, "hello");
                assert_eq!(spans.len(), 1);
                assert_eq!(spans[0].text, "Hello");
            }
            other => panic!("expected Heading, got {other:?}"),
        }
    }

    #[test]
    fn paragraph_captures_bold_italic_strike() {
        let blocks = parse("**bold** *italic* ~~strike~~\n");
        let spans = match &blocks[0] {
            Block::Paragraph { spans } => spans,
            other => panic!("expected Paragraph, got {other:?}"),
        };
        let bold = spans.iter().find(|s| s.text == "bold").expect("bold span");
        assert!(bold.style.bold);
        let italic = spans
            .iter()
            .find(|s| s.text == "italic")
            .expect("italic span");
        assert!(italic.style.italic);
        let strike = spans
            .iter()
            .find(|s| s.text == "strike")
            .expect("strike span");
        assert!(strike.style.strike);
    }

    #[test]
    fn fenced_code_block_records_language() {
        let blocks = parse("```rust\nfn main() {}\n```\n");
        match &blocks[0] {
            Block::CodeBlock { lang, code } => {
                assert_eq!(lang.as_deref(), Some("rust"));
                assert_eq!(code, "fn main() {}");
            }
            other => panic!("expected CodeBlock, got {other:?}"),
        }
    }

    #[test]
    fn mermaid_fence_becomes_mermaid_block() {
        let blocks = parse("```mermaid\ngraph TD; A-->B;\n```\n");
        match &blocks[0] {
            Block::MermaidBlock { source } => {
                assert_eq!(source, "graph TD; A-->B;");
            }
            other => panic!("expected MermaidBlock, got {other:?}"),
        }
    }

    #[test]
    fn mermaid_alias_mmd_is_recognised() {
        let blocks = parse("```mmd\ngraph TD; A-->B;\n```\n");
        assert!(matches!(blocks[0], Block::MermaidBlock { .. }));
    }

    #[test]
    fn mermaid_alias_mermaid_js_is_recognised() {
        let blocks = parse("```mermaid-js\ngraph TD; A-->B;\n```\n");
        assert!(matches!(blocks[0], Block::MermaidBlock { .. }));
    }

    #[test]
    fn blockquote_wraps_inner_paragraph() {
        let blocks = parse("> quoted\n");
        match &blocks[0] {
            Block::Quote { blocks: inner } => {
                assert_eq!(inner.len(), 1);
                assert!(matches!(inner[0], Block::Paragraph { .. }));
            }
            other => panic!("expected Quote, got {other:?}"),
        }
    }

    #[test]
    fn unordered_list_two_items() {
        let blocks = parse("- one\n- two\n");
        match &blocks[0] {
            Block::List {
                ordered,
                start: _,
                items,
            } => {
                assert!(!ordered);
                assert_eq!(items.len(), 2);
                let texts: Vec<String> = items
                    .iter()
                    .map(|it| it.spans.iter().map(|s| s.text.as_str()).collect::<String>())
                    .collect();
                assert_eq!(texts, vec!["one".to_string(), "two".to_string()]);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn ordered_list_records_starting_index() {
        let blocks = parse("3. one\n4. two\n");
        match &blocks[0] {
            Block::List { ordered, start, .. } => {
                assert!(*ordered);
                assert_eq!(*start, 3);
            }
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn task_list_marks_done_and_pending() {
        let blocks = parse("- [x] done\n- [ ] todo\n");
        let items = match &blocks[0] {
            Block::List { items, .. } => items,
            other => panic!("expected List, got {other:?}"),
        };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
    }

    #[test]
    fn table_with_alignments_parsed() {
        let blocks = parse("| L | C | R |\n|:--|:-:|--:|\n| a | b | c |\n");
        match &blocks[0] {
            Block::Table {
                headers,
                alignments,
                rows,
            } => {
                assert_eq!(headers.cells.len(), 3);
                assert_eq!(
                    alignments,
                    &vec![Alignment::Left, Alignment::Center, Alignment::Right]
                );
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].cells.len(), 3);
            }
            other => panic!("expected Table, got {other:?}"),
        }
    }

    #[test]
    fn horizontal_rule_after_paragraph() {
        let blocks = parse("foo\n\n---\n\nbar\n");
        assert!(blocks.iter().any(|b| matches!(b, Block::Rule)));
    }

    #[test]
    fn footnote_reference_and_definition() {
        let blocks = parse("See note[^1].\n\n[^1]: the note body\n");
        let para_has_ref = blocks.iter().any(|b| match b {
            Block::Paragraph { spans } => spans.iter().any(|s| s.text.contains("[^1]")),
            _ => false,
        });
        assert!(para_has_ref, "paragraph missing footnote ref");
        let footnote = blocks
            .iter()
            .find_map(|b| match b {
                Block::Footnote { label, blocks } => Some((label.clone(), blocks.clone())),
                _ => None,
            })
            .expect("footnote block");
        assert_eq!(footnote.0, "1");
        assert!(!footnote.1.is_empty());
    }

    #[test]
    fn inline_link_records_url_on_span() {
        let blocks = parse("see [docs](https://example.com)\n");
        let spans = match &blocks[0] {
            Block::Paragraph { spans } => spans,
            other => panic!("expected Paragraph, got {other:?}"),
        };
        let docs = spans.iter().find(|s| s.text == "docs").expect("docs span");
        assert_eq!(docs.style.link.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn inline_code_span_marked_as_code() {
        let blocks = parse("run `cargo test` now\n");
        let spans = match &blocks[0] {
            Block::Paragraph { spans } => spans,
            other => panic!("expected Paragraph, got {other:?}"),
        };
        let code = spans
            .iter()
            .find(|s| s.text == "cargo test")
            .expect("code span");
        assert!(code.style.code);
    }
}
