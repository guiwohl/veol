use ratatui::style::{Color, Modifier};

use crate::markdown::model::{
    Alignment, Block, FrontmatterKind, ListItem, Span, SpanStyle, TableRow,
};
use crate::mermaid::ascii;
use crate::render::theme::Theme;

#[derive(Debug, Clone, PartialEq)]
pub struct StyledSpan {
    pub text: String,
    pub fg: Color,
    pub bg: Option<Color>,
    pub modifier: Modifier,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutLine {
    pub spans: Vec<StyledSpan>,
    pub block_index: usize,
    pub heading_anchor: Option<String>,
    pub mermaid_block_index: Option<usize>,
    pub indent_cols: u16,
}

const MAX_CONTENT_WIDTH: u16 = 100;
const MIN_TERMINAL_FOR_PADDING: u16 = 120;

pub fn layout(blocks: &[Block], width: u16, theme: &Theme) -> Vec<LayoutLine> {
    let raw_width = width.max(4);
    let (prose_width, prose_indent) = if raw_width >= MIN_TERMINAL_FOR_PADDING {
        let content = MAX_CONTENT_WIDTH.min(raw_width);
        let margin = (raw_width - content) / 2;
        (content as usize, margin)
    } else {
        (raw_width as usize, 0u16)
    };

    let mut ctx = LayoutCtx::new(theme, prose_width);
    let mut prev_paragraph_like = false;
    for (idx, block) in blocks.iter().enumerate() {
        ctx.block_index = idx;
        let needs_blank_before = !ctx.lines.is_empty()
            && match block {
                Block::Heading { .. } | Block::CodeBlock { .. } => true,
                Block::Paragraph { .. } => !prev_paragraph_like,
                _ => false,
            };
        let (block_width, block_indent) = (prose_width, prose_indent);
        if needs_blank_before {
            let saved = ctx.indent_cols;
            ctx.indent_cols = block_indent;
            ctx.push_blank();
            ctx.indent_cols = saved;
        }
        ctx.width = block_width;
        ctx.indent_cols = block_indent;
        match block {
            Block::Frontmatter { kind, pairs } => render_frontmatter(&mut ctx, *kind, pairs),
            Block::Heading {
                level,
                spans,
                anchor,
                ..
            } => render_heading(&mut ctx, *level, spans, anchor),
            Block::Paragraph { spans } => render_paragraph(&mut ctx, spans, block_width, ""),
            Block::CodeBlock { lang, code } => render_code_block(&mut ctx, lang.as_deref(), code),
            Block::MermaidBlock { source } => render_mermaid(&mut ctx, idx, source),
            Block::Quote { blocks: inner } => render_quote(&mut ctx, inner, 1),
            Block::List {
                ordered,
                start,
                items,
            } => render_list(&mut ctx, *ordered, *start, items, 0),
            Block::Table {
                headers,
                alignments,
                rows,
            } => render_table(&mut ctx, headers, alignments, rows),
            Block::Rule => render_rule(&mut ctx),
            Block::Footnote {
                label,
                blocks: inner,
            } => render_footnote(&mut ctx, label, inner),
        }
        prev_paragraph_like = matches!(block, Block::Paragraph { .. });
    }
    ctx.lines
}

struct LayoutCtx<'a> {
    theme: &'a Theme,
    width: usize,
    lines: Vec<LayoutLine>,
    block_index: usize,
    indent_cols: u16,
}

impl<'a> LayoutCtx<'a> {
    fn new(theme: &'a Theme, width: usize) -> Self {
        Self {
            theme,
            width,
            lines: Vec::new(),
            block_index: 0,
            indent_cols: 0,
        }
    }

    fn fg(&self) -> Color {
        self.theme.colors.fg.to_ratatui()
    }

    fn push_line(&mut self, spans: Vec<StyledSpan>) {
        self.lines.push(LayoutLine {
            spans,
            block_index: self.block_index,
            heading_anchor: None,
            mermaid_block_index: None,
            indent_cols: self.indent_cols,
        });
    }

    fn push_blank(&mut self) {
        self.push_line(vec![StyledSpan {
            text: String::new(),
            fg: self.fg(),
            bg: None,
            modifier: Modifier::empty(),
            link: None,
        }]);
    }
}

fn plain(text: &str, fg: Color) -> StyledSpan {
    StyledSpan {
        text: text.to_string(),
        fg,
        bg: None,
        modifier: Modifier::empty(),
        link: None,
    }
}

fn span_with_bg(text: &str, fg: Color, bg: Color) -> StyledSpan {
    StyledSpan {
        text: text.to_string(),
        fg,
        bg: Some(bg),
        modifier: Modifier::empty(),
        link: None,
    }
}

fn pad_to_width(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.chars().take(width).collect()
    } else {
        let mut out = s.to_string();
        out.extend(std::iter::repeat_n(' ', width - len));
        out
    }
}

fn align_cell(s: &str, width: usize, align: Alignment) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.chars().take(width).collect();
    }
    let pad = width - len;
    match align {
        Alignment::Right => format!("{}{}", " ".repeat(pad), s),
        Alignment::Center => {
            let l = pad / 2;
            let r = pad - l;
            format!("{}{}{}", " ".repeat(l), s, " ".repeat(r))
        }
        _ => format!("{}{}", s, " ".repeat(pad)),
    }
}

fn modifier_for(style: &SpanStyle) -> Modifier {
    let mut m = Modifier::empty();
    if style.bold {
        m |= Modifier::BOLD;
    }
    if style.italic {
        m |= Modifier::ITALIC;
    }
    if style.strike {
        m |= Modifier::CROSSED_OUT;
    }
    if style.link.is_some() {
        m |= Modifier::UNDERLINED;
    }
    m
}

fn style_for_span(style: &SpanStyle, theme: &Theme) -> (Color, Option<Color>, Modifier) {
    let modifier = modifier_for(style);
    let (fg, bg) = if style.code {
        (
            theme.colors.fg.to_ratatui(),
            Some(theme.colors.code_bg.to_ratatui()),
        )
    } else if style.link.is_some() {
        (theme.colors.link.to_ratatui(), None)
    } else if style.strike {
        (theme.colors.popup_dim.to_ratatui(), None)
    } else {
        (theme.colors.fg.to_ratatui(), None)
    };
    (fg, bg, modifier)
}

#[derive(Clone)]
struct WrapToken {
    text: String,
    fg: Color,
    bg: Option<Color>,
    modifier: Modifier,
    link: Option<String>,
}

fn spans_to_tokens(spans: &[Span], theme: &Theme) -> Vec<WrapToken> {
    let mut out: Vec<WrapToken> = Vec::new();
    for span in spans {
        let (fg, bg, modifier) = style_for_span(&span.style, theme);
        let link = span.style.link.clone();
        let mut chunk = String::new();
        let push_chunk = |chunk: &mut String, out: &mut Vec<WrapToken>| {
            if !chunk.is_empty() {
                out.push(WrapToken {
                    text: std::mem::take(chunk),
                    fg,
                    bg,
                    modifier,
                    link: link.clone(),
                });
            }
        };
        for ch in span.text.chars() {
            if ch == '\n' {
                push_chunk(&mut chunk, &mut out);
                out.push(WrapToken {
                    text: "\n".to_string(),
                    fg,
                    bg,
                    modifier,
                    link: link.clone(),
                });
            } else if ch == ' ' {
                chunk.push(ch);
                push_chunk(&mut chunk, &mut out);
            } else {
                chunk.push(ch);
            }
        }
        push_chunk(&mut chunk, &mut out);
    }
    out
}

fn wrap_tokens_into_lines(
    tokens: &[WrapToken],
    width: usize,
    continuation_indent: &str,
    indent_fg: Color,
) -> Vec<Vec<StyledSpan>> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines: Vec<Vec<StyledSpan>> = Vec::new();
    let mut current: Vec<StyledSpan> = Vec::new();
    let mut current_width = 0usize;
    let mut first_line = true;

    let prepare_line = |first: bool, current: &mut Vec<StyledSpan>, current_width: &mut usize| {
        if !first && !continuation_indent.is_empty() {
            current.push(plain(continuation_indent, indent_fg));
            *current_width += continuation_indent.chars().count();
        }
    };

    prepare_line(first_line, &mut current, &mut current_width);

    let push_styled = |current: &mut Vec<StyledSpan>, tok: &WrapToken, text: String| {
        let modifier = tok.modifier;
        if let Some(last) = current.last_mut() {
            if last.fg == tok.fg
                && last.bg == tok.bg
                && last.modifier == modifier
                && last.link == tok.link
            {
                last.text.push_str(&text);
                return;
            }
        }
        current.push(StyledSpan {
            text,
            fg: tok.fg,
            bg: tok.bg,
            modifier,
            link: tok.link.clone(),
        });
    };

    let avail = |first: bool| -> usize {
        if first {
            width
        } else {
            width.saturating_sub(continuation_indent.chars().count())
        }
    };

    for tok in tokens {
        if tok.text == "\n" {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            first_line = false;
            prepare_line(first_line, &mut current, &mut current_width);
            continue;
        }
        let tok_len = tok.text.chars().count();
        let line_avail = avail(first_line);
        let is_space = tok.text.chars().all(|c| c == ' ');
        if current_width + tok_len <= width {
            let at_line_start = current_width == 0
                || (!first_line && current_width == continuation_indent.chars().count());
            if !(is_space && at_line_start) {
                push_styled(&mut current, tok, tok.text.clone());
                current_width += tok_len;
            }
        } else if is_space {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            first_line = false;
            prepare_line(first_line, &mut current, &mut current_width);
        } else if tok_len > line_avail {
            let mut remaining: String = tok.text.clone();
            while !remaining.is_empty() {
                let cap = avail(first_line).saturating_sub(current_width);
                if cap == 0 {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                    first_line = false;
                    prepare_line(first_line, &mut current, &mut current_width);
                    continue;
                }
                let take: String = remaining.chars().take(cap).collect();
                let take_len = take.chars().count();
                remaining = remaining.chars().skip(cap).collect();
                push_styled(&mut current, tok, take);
                current_width += take_len;
                if !remaining.is_empty() {
                    lines.push(std::mem::take(&mut current));
                    current_width = 0;
                    first_line = false;
                    prepare_line(first_line, &mut current, &mut current_width);
                }
            }
        } else {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            first_line = false;
            prepare_line(first_line, &mut current, &mut current_width);
            push_styled(&mut current, tok, tok.text.clone());
            current_width += tok_len;
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

fn render_paragraph(ctx: &mut LayoutCtx<'_>, spans: &[Span], width: usize, prefix: &str) {
    let tokens = spans_to_tokens(spans, ctx.theme);
    let prefix_width = prefix.chars().count();
    let inner_width = width.saturating_sub(prefix_width).max(1);
    let lines = wrap_tokens_into_lines(&tokens, inner_width, "", ctx.fg());
    let prefix_fg = ctx.fg();
    for line in lines {
        let mut styled = Vec::new();
        if !prefix.is_empty() {
            styled.push(plain(prefix, prefix_fg));
        }
        styled.extend(line);
        ctx.push_line(styled);
    }
}

fn render_heading(ctx: &mut LayoutCtx<'_>, level: u8, spans: &[Span], anchor: &str) {
    let heading_fg = match level {
        1 => ctx.theme.colors.heading_1,
        2 => ctx.theme.colors.heading_2,
        3 => ctx.theme.colors.heading_3,
        4 => ctx.theme.colors.heading_4,
        5 => ctx.theme.colors.heading_5,
        _ => ctx.theme.colors.heading_6,
    }
    .to_ratatui();
    let mut tokens = spans_to_tokens(spans, ctx.theme);
    for tok in tokens.iter_mut() {
        tok.fg = heading_fg;
        tok.modifier |= Modifier::BOLD;
    }
    let lines = wrap_tokens_into_lines(&tokens, ctx.width, "", heading_fg);
    let first_line_idx = ctx.lines.len();
    for line in lines {
        ctx.push_line(line);
    }
    if let Some(first) = ctx.lines.get_mut(first_line_idx) {
        first.heading_anchor = Some(anchor.to_string());
    }
}

fn render_code_block(ctx: &mut LayoutCtx<'_>, lang: Option<&str>, code: &str) {
    let bg = ctx.theme.colors.code_bg.to_ratatui();
    let fg = ctx.theme.colors.fg.to_ratatui();
    let lang_fg = ctx.theme.colors.code_border.to_ratatui();

    if let Some(l) = lang {
        if !l.is_empty() {
            let label = format!(" {} ", l);
            let pad_width = ctx.width.saturating_sub(label.chars().count());
            let pad = " ".repeat(pad_width);
            ctx.push_line(vec![
                span_with_bg(&label, lang_fg, bg),
                span_with_bg(&pad, fg, bg),
            ]);
        }
    }

    let highlighted = crate::render::code::HIGHLIGHTER.highlight_lines(code, lang, ctx.theme);
    for line_spans in highlighted {
        emit_code_line(ctx, &line_spans, fg, bg);
    }

    ctx.push_blank();
}

fn emit_code_line(ctx: &mut LayoutCtx<'_>, line_spans: &[StyledSpan], fg: Color, bg: Color) {
    let width = ctx.width;
    if line_spans.is_empty() {
        ctx.push_line(vec![span_with_bg(&" ".repeat(width), fg, bg)]);
        return;
    }

    let mut current: Vec<StyledSpan> = Vec::new();
    let mut current_width = 0usize;
    let mut first = true;

    let prepare = |first: bool, cur: &mut Vec<StyledSpan>, w: &mut usize| {
        if !first {
            cur.push(span_with_bg("  ", fg, bg));
            *w += 2;
        }
    };
    prepare(first, &mut current, &mut current_width);

    for s in line_spans {
        for ch in s.text.chars() {
            if current_width >= width {
                let pad = width.saturating_sub(current_width);
                if pad > 0 {
                    current.push(span_with_bg(&" ".repeat(pad), fg, bg));
                }
                ctx.push_line(std::mem::take(&mut current));
                current_width = 0;
                first = false;
                prepare(first, &mut current, &mut current_width);
            }
            let piece = ch.to_string();
            let span_bg = s.bg.unwrap_or(bg);
            if let Some(last) = current.last_mut() {
                if last.fg == s.fg
                    && last.bg == Some(span_bg)
                    && last.modifier == s.modifier
                    && last.link == s.link
                {
                    last.text.push(ch);
                    current_width += 1;
                    continue;
                }
            }
            current.push(StyledSpan {
                text: piece,
                fg: s.fg,
                bg: Some(span_bg),
                modifier: s.modifier,
                link: s.link.clone(),
            });
            current_width += 1;
        }
    }

    let pad = width.saturating_sub(current_width);
    if pad > 0 {
        current.push(span_with_bg(&" ".repeat(pad), fg, bg));
    }
    ctx.push_line(current);
}

fn render_mermaid(ctx: &mut LayoutCtx<'_>, block_idx: usize, source: &str) {
    let default_fg = ctx.theme.colors.mermaid_caption.to_ratatui();
    let rows = ascii::render_mermaid_styled(source, ctx.width as u16);
    for row in rows {
        let spans: Vec<StyledSpan> = if row.is_empty() {
            vec![StyledSpan {
                text: String::new(),
                fg: default_fg,
                bg: None,
                modifier: Modifier::empty(),
                link: None,
            }]
        } else {
            row.into_iter()
                .map(|run| StyledSpan {
                    text: run.text,
                    fg: run.color.unwrap_or(default_fg),
                    bg: None,
                    modifier: Modifier::empty(),
                    link: None,
                })
                .collect()
        };
        let line = LayoutLine {
            spans,
            block_index: ctx.block_index,
            heading_anchor: None,
            mermaid_block_index: Some(block_idx),
            indent_cols: ctx.indent_cols,
        };
        ctx.lines.push(line);
    }
}

fn render_quote(ctx: &mut LayoutCtx<'_>, inner: &[Block], depth: usize) {
    let marker_fg = ctx.theme.colors.quote_marker.to_ratatui();
    let text_fg = ctx.theme.colors.quote_text.to_ratatui();
    let prefix_unit = "│ ";
    let prefix: String = prefix_unit.repeat(depth);
    let prefix_width = prefix.chars().count();
    let inner_width = ctx.width.saturating_sub(prefix_width).max(1);

    for block in inner {
        match block {
            Block::Paragraph { spans } => {
                let mut tokens = spans_to_tokens(spans, ctx.theme);
                for tok in tokens.iter_mut() {
                    if tok.bg.is_none() {
                        tok.fg = text_fg;
                    }
                }
                let lines = wrap_tokens_into_lines(&tokens, inner_width, "", text_fg);
                for line in lines {
                    let mut row = vec![plain(&prefix, marker_fg)];
                    row.extend(line);
                    ctx.push_line(row);
                }
            }
            Block::Quote { blocks: nested } => {
                render_quote(ctx, nested, depth + 1);
            }
            other => {
                let saved_width = ctx.width;
                ctx.width = inner_width;
                let start = ctx.lines.len();
                match other {
                    Block::Heading {
                        level,
                        spans,
                        anchor,
                        ..
                    } => {
                        render_heading(ctx, *level, spans, anchor);
                    }
                    Block::CodeBlock { lang, code } => {
                        render_code_block(ctx, lang.as_deref(), code);
                    }
                    Block::List {
                        ordered,
                        start,
                        items,
                    } => {
                        render_list(ctx, *ordered, *start, items, 0);
                    }
                    Block::Rule => render_rule(ctx),
                    _ => {}
                }
                ctx.width = saved_width;
                for i in start..ctx.lines.len() {
                    let line = &mut ctx.lines[i];
                    let mut prefixed = vec![plain(&prefix, marker_fg)];
                    prefixed.append(&mut line.spans);
                    line.spans = prefixed;
                }
            }
        }
    }
}

fn render_list(
    ctx: &mut LayoutCtx<'_>,
    ordered: bool,
    start: u64,
    items: &[ListItem],
    depth: usize,
) {
    let indent: String = " ".repeat(depth * 2);
    let fg = ctx.fg();
    let done_fg = ctx.theme.colors.task_done.to_ratatui();
    let pending_fg = ctx.theme.colors.task_pending.to_ratatui();

    for (i, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}. ", start + i as u64)
        } else {
            "• ".to_string()
        };
        let task_prefix = match item.task {
            Some(true) => Some(("[x] ".to_string(), done_fg, Modifier::BOLD)),
            Some(false) => Some(("[ ] ".to_string(), pending_fg, Modifier::empty())),
            None => None,
        };
        let marker_width = indent.chars().count()
            + marker.chars().count()
            + task_prefix
                .as_ref()
                .map(|t| t.0.chars().count())
                .unwrap_or(0);
        let continuation = " ".repeat(marker_width);

        let tokens = spans_to_tokens(&item.spans, ctx.theme);
        let inner_width = ctx.width.saturating_sub(marker_width).max(1);
        let lines = wrap_tokens_into_lines(&tokens, inner_width, "", fg);

        for (j, line) in lines.into_iter().enumerate() {
            let mut row = Vec::new();
            if j == 0 {
                if !indent.is_empty() {
                    row.push(plain(&indent, fg));
                }
                row.push(plain(&marker, fg));
                if let Some((text, color, modi)) = &task_prefix {
                    row.push(StyledSpan {
                        text: text.clone(),
                        fg: *color,
                        bg: None,
                        modifier: *modi,
                        link: None,
                    });
                }
            } else {
                row.push(plain(&continuation, fg));
            }
            row.extend(line);
            ctx.push_line(row);
        }

        for child in &item.children {
            match child {
                Block::List {
                    ordered,
                    start,
                    items,
                } => {
                    render_list(ctx, *ordered, *start, items, depth + 1);
                }
                Block::Paragraph { spans } => {
                    let tokens = spans_to_tokens(spans, ctx.theme);
                    let inner_width = ctx.width.saturating_sub(marker_width).max(1);
                    let lines = wrap_tokens_into_lines(&tokens, inner_width, "", fg);
                    for line in lines {
                        let mut row = vec![plain(&continuation, fg)];
                        row.extend(line);
                        ctx.push_line(row);
                    }
                }
                _ => {}
            }
        }
    }
}

fn cell_plain_text(cell: &[Span]) -> String {
    cell.iter().map(|s| s.text.as_str()).collect()
}

fn longest_word(s: &str) -> usize {
    s.split_whitespace()
        .map(|w| w.chars().count())
        .max()
        .unwrap_or(0)
}

fn wrap_plain_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    for source_line in text.split('\n') {
        let mut current = String::new();
        for word in source_line.split(' ') {
            if word.is_empty() {
                if current.is_empty() || current.chars().count() < width {
                    current.push(' ');
                }
                continue;
            }
            let wlen = word.chars().count();
            if current.is_empty() {
                if wlen <= width {
                    current.push_str(word);
                } else {
                    let mut rest = word.to_string();
                    while !rest.is_empty() {
                        let take: String = rest.chars().take(width).collect();
                        rest = rest.chars().skip(width).collect();
                        if rest.is_empty() {
                            current = take;
                        } else {
                            lines.push(take);
                        }
                    }
                }
            } else if current.chars().count() + 1 + wlen <= width {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(std::mem::take(&mut current));
                if wlen <= width {
                    current.push_str(word);
                } else {
                    let mut rest = word.to_string();
                    while !rest.is_empty() {
                        let take: String = rest.chars().take(width).collect();
                        rest = rest.chars().skip(width).collect();
                        if rest.is_empty() {
                            current = take;
                        } else {
                            lines.push(take);
                        }
                    }
                }
            }
        }
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn compute_table_widths(headers: &TableRow, rows: &[TableRow], width: usize) -> Vec<usize> {
    let n = headers.cells.len();
    if n == 0 {
        return Vec::new();
    }
    let border_chars = n + 1;
    let inner_padding = n * 2;
    let usable = width.saturating_sub(border_chars + inner_padding);
    if usable < n {
        return vec![1; n];
    }

    let mut max_widths = vec![0usize; n];
    let mut min_widths = vec![1usize; n];

    let scan = |row: &TableRow, max_w: &mut [usize], min_w: &mut [usize]| {
        for (i, cell) in row.cells.iter().enumerate() {
            if i >= max_w.len() {
                break;
            }
            let text = cell_plain_text(cell);
            let longest_line = text
                .split('\n')
                .map(|l| l.chars().count())
                .max()
                .unwrap_or(0);
            if longest_line > max_w[i] {
                max_w[i] = longest_line;
            }
            let lw = longest_word(&text).max(1);
            if lw > min_w[i] {
                min_w[i] = lw;
            }
        }
    };
    scan(headers, &mut max_widths, &mut min_widths);
    for row in rows {
        scan(row, &mut max_widths, &mut min_widths);
    }

    let total_max: usize = max_widths.iter().sum();
    if total_max <= usable {
        return max_widths;
    }

    let total_min: usize = min_widths.iter().sum();
    if total_min >= usable {
        return min_widths;
    }

    let extra = usable - total_min;
    let slack: usize = max_widths
        .iter()
        .zip(min_widths.iter())
        .map(|(mx, mn)| mx.saturating_sub(*mn))
        .sum();
    if slack == 0 {
        return min_widths;
    }
    let mut widths = min_widths.clone();
    let mut distributed = 0usize;
    for i in 0..n {
        let s = max_widths[i].saturating_sub(min_widths[i]);
        if i + 1 == n {
            widths[i] += extra - distributed;
        } else {
            let share = extra * s / slack;
            widths[i] += share;
            distributed += share;
        }
    }
    widths
}

fn render_table(
    ctx: &mut LayoutCtx<'_>,
    headers: &TableRow,
    alignments: &[Alignment],
    rows: &[TableRow],
) {
    if headers.cells.is_empty() {
        return;
    }
    let border_fg = ctx.theme.colors.table_border.to_ratatui();
    let header_fg = ctx.theme.colors.table_header_fg.to_ratatui();
    let widths = compute_table_widths(headers, rows, ctx.width);
    let aligns: Vec<Alignment> = headers
        .cells
        .iter()
        .enumerate()
        .map(|(i, _)| *alignments.get(i).unwrap_or(&Alignment::Left))
        .collect();

    let border = |left: char, sep: char, right: char, ctx: &mut LayoutCtx<'_>| {
        let mut s = String::new();
        s.push(left);
        for (i, w) in widths.iter().enumerate() {
            for _ in 0..(*w + 2) {
                s.push('─');
            }
            s.push(if i + 1 == widths.len() { right } else { sep });
        }
        ctx.push_line(vec![plain(&s, border_fg)]);
    };

    let render_row = |ctx: &mut LayoutCtx<'_>, row: &TableRow, is_header: bool| {
        let cell_lines: Vec<Vec<String>> = row
            .cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let text = cell_plain_text(cell);
                wrap_plain_text(&text, widths[i])
            })
            .collect();
        let height = cell_lines.iter().map(|l| l.len()).max().unwrap_or(1);
        for line_idx in 0..height {
            let mut spans: Vec<StyledSpan> = Vec::new();
            spans.push(plain("│", border_fg));
            for (i, w) in widths.iter().enumerate() {
                let raw = cell_lines
                    .get(i)
                    .and_then(|l| l.get(line_idx))
                    .cloned()
                    .unwrap_or_default();
                let aligned = align_cell(&raw, *w, aligns[i]);
                let cell_span = if is_header {
                    StyledSpan {
                        text: format!(" {} ", aligned),
                        fg: header_fg,
                        bg: None,
                        modifier: Modifier::BOLD,
                        link: None,
                    }
                } else {
                    plain(&format!(" {} ", aligned), ctx.fg())
                };
                spans.push(cell_span);
                spans.push(plain("│", border_fg));
                let _ = i;
            }
            ctx.push_line(spans);
        }
    };

    border('┌', '┬', '┐', ctx);
    render_row(ctx, headers, true);
    border('├', '┼', '┤', ctx);
    for (i, row) in rows.iter().enumerate() {
        render_row(ctx, row, false);
        if i + 1 < rows.len() {
            border('├', '┼', '┤', ctx);
        }
    }
    border('└', '┴', '┘', ctx);
}

fn render_rule(ctx: &mut LayoutCtx<'_>) {
    let fg = ctx.theme.colors.hr.to_ratatui();
    let line: String = "─".repeat(ctx.width);
    ctx.push_line(vec![plain(&line, fg)]);
}

fn render_footnote(ctx: &mut LayoutCtx<'_>, label: &str, blocks: &[Block]) {
    let link_fg = ctx.theme.colors.link.to_ratatui();
    let prefix = format!("[^{}]: ", label);
    let prefix_width = prefix.chars().count();
    let body_indent = " ".repeat(prefix_width);

    let mut first_block = true;
    for block in blocks {
        match block {
            Block::Paragraph { spans } => {
                let tokens = spans_to_tokens(spans, ctx.theme);
                let inner_width = ctx.width.saturating_sub(prefix_width).max(1);
                let lines = wrap_tokens_into_lines(&tokens, inner_width, "", ctx.fg());
                for (i, line) in lines.into_iter().enumerate() {
                    let mut row = Vec::new();
                    if first_block && i == 0 {
                        row.push(StyledSpan {
                            text: prefix.clone(),
                            fg: link_fg,
                            bg: None,
                            modifier: Modifier::empty(),
                            link: None,
                        });
                    } else {
                        row.push(plain(&body_indent, ctx.fg()));
                    }
                    row.extend(line);
                    ctx.push_line(row);
                }
                first_block = false;
            }
            _ => {
                first_block = false;
            }
        }
    }
}

fn render_frontmatter(ctx: &mut LayoutCtx<'_>, _kind: FrontmatterKind, pairs: &[(String, String)]) {
    let border_fg = ctx.theme.colors.popup_border.to_ratatui();
    let key_fg = ctx.theme.colors.keyword.to_ratatui();
    let val_fg = ctx.theme.colors.fg.to_ratatui();

    if pairs.is_empty() {
        return;
    }

    let total_inner_width = ctx.width.saturating_sub(2).max(4);
    let max_key_len = pairs
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(1);
    let key_col = max_key_len.min(total_inner_width / 3).max(1);
    let gap = 3usize;
    let val_col = total_inner_width.saturating_sub(2 + key_col + gap).max(1);
    let inner_width = 2 + key_col + gap + val_col;

    let top = format!("┌{}┐", "─".repeat(inner_width));
    let bot = format!("└{}┘", "─".repeat(inner_width));
    ctx.push_line(vec![plain(&top, border_fg)]);

    for (k, v) in pairs {
        let key_text = pad_to_width(k, key_col);
        let val_lines = wrap_plain_text(v, val_col);
        for (i, vl) in val_lines.iter().enumerate() {
            let mut spans = Vec::new();
            spans.push(plain("│", border_fg));
            spans.push(plain("  ", val_fg));
            if i == 0 {
                spans.push(StyledSpan {
                    text: key_text.clone(),
                    fg: key_fg,
                    bg: None,
                    modifier: Modifier::empty(),
                    link: None,
                });
            } else {
                spans.push(plain(&" ".repeat(key_col), val_fg));
            }
            spans.push(plain(&" ".repeat(gap), val_fg));
            let val_padded = pad_to_width(vl, val_col);
            spans.push(plain(&val_padded, val_fg));
            spans.push(plain("│", border_fg));
            ctx.push_line(spans);
        }
    }
    ctx.push_line(vec![plain(&bot, border_fg)]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;

    fn render_text(lines: &[LayoutLine]) -> String {
        let mut out = String::new();
        for (i, line) in lines.iter().enumerate() {
            for span in &line.spans {
                out.push_str(&span.text);
            }
            if i + 1 < lines.len() {
                out.push('\n');
            }
        }
        out
    }

    fn theme() -> Theme {
        Theme::default()
    }

    #[test]
    fn layout_empty_returns_empty() {
        let lines = layout(&[], 80, &theme());
        assert!(lines.is_empty());
    }

    #[test]
    fn layout_block_indices_match_source() {
        let blocks = parse("# A\n\npara\n\n# B\n");
        let lines = layout(&blocks, 80, &theme());
        let last = lines.last().expect("non-empty");
        assert_eq!(last.block_index, blocks.len() - 1);
        let first_non_blank = lines
            .iter()
            .find(|l| !l.spans.iter().all(|s| s.text.trim().is_empty()))
            .unwrap();
        assert_eq!(first_non_blank.block_index, 0);
    }

    #[test]
    fn layout_heading_anchor_on_first_line_only() {
        let blocks = parse("# Some Long Heading That Will Wrap Over The Width\n");
        let lines = layout(&blocks, 20, &theme());
        let with_anchor: Vec<_> = lines
            .iter()
            .filter(|l| l.heading_anchor.is_some())
            .collect();
        assert_eq!(with_anchor.len(), 1, "exactly one line should carry anchor");
        assert_eq!(
            with_anchor[0].heading_anchor.as_deref(),
            Some("some-long-heading-that-will-wrap-over-the-width")
        );
    }

    #[test]
    fn layout_mermaid_index_populated() {
        let blocks = parse("```mermaid\ngraph TD; A-->B;\n```\n");
        let lines = layout(&blocks, 80, &theme());
        let mer = lines
            .iter()
            .find(|l| l.mermaid_block_index.is_some())
            .expect("mermaid line");
        assert_eq!(mer.mermaid_block_index, Some(0));
    }

    #[test]
    fn layout_width_change_relayouts() {
        let blocks = parse("The quick brown fox jumps over the lazy dog repeatedly today.\n");
        let narrow = layout(&blocks, 20, &theme()).len();
        let wide = layout(&blocks, 200, &theme()).len();
        assert!(narrow > wide, "narrow={narrow}, wide={wide}");
    }

    #[test]
    fn layout_heading_simple() {
        let blocks = parse("# Hello World\n");
        let rendered = render_text(&layout(&blocks, 40, &theme()));
        insta::assert_snapshot!("layout_heading_simple", rendered);
    }

    #[test]
    fn layout_paragraph_wraps_at_word_boundary() {
        let blocks = parse("the quick brown fox jumps over the lazy dog\n");
        let rendered = render_text(&layout(&blocks, 20, &theme()));
        insta::assert_snapshot!("layout_paragraph_wraps_at_word_boundary", rendered);
    }

    #[test]
    fn layout_paragraph_with_bold_italic_strike_link() {
        let blocks = parse("**bold** *italic* ~~strike~~ and a [link](https://x.test).\n");
        let lines = layout(&blocks, 60, &theme());
        let rendered = render_text(&lines);
        insta::assert_snapshot!("layout_paragraph_with_bold_italic_strike_link", rendered);
        let bold = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.text == "bold")
            .expect("bold span");
        assert!(bold.modifier.contains(Modifier::BOLD));
        let link = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.text == "link")
            .expect("link span");
        assert!(link.modifier.contains(Modifier::UNDERLINED));
        assert_eq!(link.link.as_deref(), Some("https://x.test"));
        let strike = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .find(|s| s.text == "strike")
            .expect("strike span");
        assert!(strike.modifier.contains(Modifier::CROSSED_OUT));
    }

    #[test]
    fn layout_code_block_with_lang_label() {
        let blocks = parse("```rust\nfn main() {}\n```\n");
        let rendered = render_text(&layout(&blocks, 30, &theme()));
        insta::assert_snapshot!("layout_code_block_with_lang_label", rendered);
    }

    #[test]
    fn layout_code_block_long_line_wraps() {
        let blocks = parse("```\nverylongidentifierwithnoboundariesatall_continues\n```\n");
        let rendered = render_text(&layout(&blocks, 20, &theme()));
        insta::assert_snapshot!("layout_code_block_long_line_wraps", rendered);
    }

    #[test]
    fn layout_mermaid_renders_inline_ascii() {
        let blocks = parse("```mermaid\ngraph TD; A-->B;\n```\n");
        let lines = layout(&blocks, 60, &theme());
        let mermaid_lines: Vec<&LayoutLine> = lines
            .iter()
            .filter(|l| l.mermaid_block_index.is_some())
            .collect();
        assert!(
            mermaid_lines.len() >= 3,
            "expected >=3 mermaid-tagged lines, got {}",
            mermaid_lines.len()
        );
        for l in &mermaid_lines {
            assert_eq!(l.mermaid_block_index, Some(0));
        }
    }

    #[test]
    fn layout_blockquote_nested() {
        let blocks = parse("> outer\n>\n> > inner quote line\n");
        let rendered = render_text(&layout(&blocks, 40, &theme()));
        insta::assert_snapshot!("layout_blockquote_nested", rendered);
    }

    #[test]
    fn layout_unordered_list_two_items() {
        let blocks = parse("- first\n- second\n");
        let rendered = render_text(&layout(&blocks, 30, &theme()));
        insta::assert_snapshot!("layout_unordered_list_two_items", rendered);
    }

    #[test]
    fn layout_ordered_list_starting_at_three() {
        let blocks = parse("3. one\n4. two\n");
        let rendered = render_text(&layout(&blocks, 30, &theme()));
        insta::assert_snapshot!("layout_ordered_list_starting_at_three", rendered);
    }

    #[test]
    fn layout_task_list_done_and_pending() {
        let blocks = parse("- [x] done thing\n- [ ] pending thing\n");
        let rendered = render_text(&layout(&blocks, 40, &theme()));
        insta::assert_snapshot!("layout_task_list_done_and_pending", rendered);
    }

    #[test]
    fn layout_table_with_alignments() {
        let blocks = parse("| L | C | R |\n|:--|:-:|--:|\n| a | bb | ccc |\n| dd | e | f |\n");
        let rendered = render_text(&layout(&blocks, 40, &theme()));
        insta::assert_snapshot!("layout_table_with_alignments", rendered);
    }

    #[test]
    fn layout_table_wide_shrinks_via_wrap() {
        let blocks = parse(
            "| col1 | col2 |\n|---|---|\n| a long sentence that must wrap | another long-ish piece |\n",
        );
        let rendered = render_text(&layout(&blocks, 30, &theme()));
        insta::assert_snapshot!("layout_table_wide_shrinks_via_wrap", rendered);
    }

    #[test]
    fn layout_horizontal_rule() {
        let blocks = parse("foo\n\n---\n\nbar\n");
        let rendered = render_text(&layout(&blocks, 20, &theme()));
        insta::assert_snapshot!("layout_horizontal_rule", rendered);
    }

    #[test]
    fn layout_footnote_definition() {
        let blocks = parse("see[^1].\n\n[^1]: the note body here\n");
        let rendered = render_text(&layout(&blocks, 40, &theme()));
        insta::assert_snapshot!("layout_footnote_definition", rendered);
    }

    #[test]
    fn layout_frontmatter_card_yaml() {
        let blocks =
            parse("---\ntitle: Veol — Spec\nauthor: guiwohl\ndate: 2026-05-26\n---\n\n# Body\n");
        let rendered = render_text(&layout(&blocks, 50, &theme()));
        insta::assert_snapshot!("layout_frontmatter_card_yaml", rendered);
    }

    #[test]
    fn layout_frontmatter_card_toml() {
        let blocks = parse("+++\ntitle = \"Veol\"\nauthor = \"guiwohl\"\n+++\n\n# Body\n");
        let rendered = render_text(&layout(&blocks, 50, &theme()));
        insta::assert_snapshot!("layout_frontmatter_card_toml", rendered);
    }

    fn line_text(line: &LayoutLine) -> String {
        line.spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn render_unordered_list_two_items() {
        let blocks = parse("- one\n- two\n");
        let lines = layout(&blocks, 30, &theme());
        let texts: Vec<String> = lines.iter().map(line_text).collect();
        assert!(
            texts.iter().any(|l| l == "• one"),
            "expected '• one' line, got {texts:?}"
        );
        assert!(
            texts.iter().any(|l| l == "• two"),
            "expected '• two' line, got {texts:?}"
        );
    }

    #[test]
    fn render_nested_unordered_list() {
        let blocks = parse("- outer\n  - inner\n");
        let lines = layout(&blocks, 40, &theme());
        let texts: Vec<String> = lines.iter().map(line_text).collect();
        assert!(
            texts.iter().any(|l| l == "• outer"),
            "expected '• outer', got {texts:?}"
        );
        assert!(
            texts.iter().any(|l| l == "  • inner"),
            "expected '  • inner' (2-space indent), got {texts:?}"
        );
    }

    #[test]
    fn render_ordered_list() {
        let blocks = parse("1. one\n2. two\n");
        let lines = layout(&blocks, 30, &theme());
        let texts: Vec<String> = lines.iter().map(line_text).collect();
        assert!(
            texts.iter().any(|l| l == "1. one"),
            "expected '1. one', got {texts:?}"
        );
        assert!(
            texts.iter().any(|l| l == "2. two"),
            "expected '2. two', got {texts:?}"
        );
    }

    #[test]
    fn render_list_item_wraps_with_continuation_indent() {
        let blocks = parse("- the quick brown fox jumps over the lazy dog repeatedly\n");
        let lines = layout(&blocks, 20, &theme());
        let texts: Vec<String> = lines.iter().map(line_text).collect();
        let first = texts.iter().find(|l| l.starts_with("• ")).expect("marker");
        assert!(
            first.starts_with("• "),
            "first line should start with marker"
        );
        let cont_idx = texts.iter().position(|l| l.starts_with("  ")).unwrap_or(0);
        assert!(
            cont_idx > 0,
            "expected wrapped continuation line, got {texts:?}"
        );
        let cont = &texts[cont_idx];
        assert!(
            cont.starts_with("  ") && !cont.starts_with("• "),
            "continuation should be 2 spaces (no marker), got {cont:?}"
        );
    }

    #[test]
    fn render_table_has_divider_between_data_rows() {
        let blocks = parse("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n| 5 | 6 |\n");
        let lines = layout(&blocks, 40, &theme());
        let texts: Vec<String> = lines.iter().map(line_text).collect();
        assert_eq!(
            texts.len(),
            9,
            "expected 9 lines (top, header, hsep, row, div, row, div, row, bot), got {texts:?}"
        );
        assert!(texts[0].starts_with('┌'), "top: {:?}", texts[0]);
        assert!(texts[2].starts_with('├'), "header sep: {:?}", texts[2]);
        assert!(texts[4].starts_with('├'), "div1: {:?}", texts[4]);
        assert!(texts[6].starts_with('├'), "div2: {:?}", texts[6]);
        assert!(texts[8].starts_with('└'), "bot: {:?}", texts[8]);
    }

    #[test]
    fn layout_centers_prose_at_wide_width() {
        let blocks = parse("hello world\n");
        let lines = layout(&blocks, 200, &theme());
        let first = lines.first().expect("at least one line");
        assert!(
            first.indent_cols > 0,
            "expected non-zero indent at width 200, got {}",
            first.indent_cols
        );
        let prose_w = MAX_CONTENT_WIDTH;
        let expected = (200 - prose_w) / 2;
        assert_eq!(first.indent_cols, expected);
    }

    #[test]
    fn layout_no_padding_when_terminal_narrow() {
        let blocks = parse("hello\n");
        let lines = layout(&blocks, 80, &theme());
        let first = lines.first().expect("line");
        assert_eq!(first.indent_cols, 0);
    }

    #[test]
    fn layout_code_block_uses_same_indent_as_prose() {
        let src = "para text\n\n```\nfn x() {}\n```\n";
        let blocks = parse(src);
        let lines = layout(&blocks, 200, &theme());
        let prose_indent = lines
            .iter()
            .find(|l| matches!(blocks.get(l.block_index), Some(Block::Paragraph { .. })))
            .map(|l| l.indent_cols)
            .expect("paragraph line");
        let code_indent = lines
            .iter()
            .find(|l| matches!(blocks.get(l.block_index), Some(Block::CodeBlock { .. })))
            .map(|l| l.indent_cols)
            .expect("code line");
        assert_eq!(
            code_indent, prose_indent,
            "code blocks should share prose indent (revert breakout)"
        );
    }
}
