use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::markdown::layout::{LayoutLine, StyledSpan};
use crate::render::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Off,
    Editing,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub line_index: usize,
    pub start_col: usize,
    pub length: usize,
}

#[derive(Debug, Default)]
pub struct SearchState {
    pub mode: SearchMode,
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub current: usize,
}

impl SearchState {
    pub fn start(&mut self) {
        self.mode = SearchMode::Editing;
        self.query.clear();
        self.matches.clear();
        self.current = 0;
    }

    pub fn input_char(&mut self, c: char) {
        self.query.push(c);
    }

    pub fn backspace(&mut self) {
        self.query.pop();
    }

    pub fn confirm(&mut self, lines: &[LayoutLine]) {
        self.matches = find_matches(lines, &self.query);
        self.mode = SearchMode::Active;
        self.current = 0;
    }

    pub fn cancel(&mut self) {
        self.mode = SearchMode::Off;
        self.query.clear();
        self.matches.clear();
        self.current = 0;
    }

    pub fn next(&mut self) {
        if self.matches.is_empty() {
            self.current = 0;
            return;
        }
        self.current = (self.current + 1) % self.matches.len();
    }

    pub fn prev(&mut self) {
        if self.matches.is_empty() {
            self.current = 0;
            return;
        }
        if self.current == 0 {
            self.current = self.matches.len() - 1;
        } else {
            self.current -= 1;
        }
    }

    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current)
    }

    pub fn is_smart_case(&self) -> bool {
        self.query.chars().any(|c| c.is_uppercase())
    }
}

pub fn find_matches(lines: &[LayoutLine], query: &str) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }
    let case_sensitive = query.chars().any(|c| c.is_uppercase());
    let needle: String = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };
    let needle_chars: Vec<char> = needle.chars().collect();
    let needle_len = needle_chars.len();
    if needle_len == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let mut text = String::new();
        for span in &line.spans {
            text.push_str(&span.text);
        }
        let hay_chars: Vec<char> = if case_sensitive {
            text.chars().collect()
        } else {
            text.chars().flat_map(|c| c.to_lowercase()).collect()
        };
        if hay_chars.len() < needle_len {
            continue;
        }
        let mut i = 0;
        while i + needle_len <= hay_chars.len() {
            if hay_chars[i..i + needle_len] == needle_chars[..] {
                out.push(SearchMatch {
                    line_index,
                    start_col: i,
                    length: needle_len,
                });
                i += needle_len;
            } else {
                i += 1;
            }
        }
    }
    out
}

fn span_char_count(span: &StyledSpan) -> usize {
    span.text.chars().count()
}

fn split_span_at_char_offset(span: &StyledSpan, offset: usize) -> (StyledSpan, StyledSpan) {
    // char offset, not byte offset — slicing by byte indices on UTF-8 text would panic.
    let total = span_char_count(span);
    let off = offset.min(total);
    let left_text: String = span.text.chars().take(off).collect();
    let right_text: String = span.text.chars().skip(off).collect();
    let left = StyledSpan {
        text: left_text,
        fg: span.fg,
        bg: span.bg,
        modifier: span.modifier,
        link: span.link.clone(),
    };
    let right = StyledSpan {
        text: right_text,
        fg: span.fg,
        bg: span.bg,
        modifier: span.modifier,
        link: span.link.clone(),
    };
    (left, right)
}

pub fn highlight_line_with_matches(
    line: &LayoutLine,
    matches_on_this_line: &[&SearchMatch],
    current_match: Option<&SearchMatch>,
    theme: &Theme,
) -> LayoutLine {
    if matches_on_this_line.is_empty() {
        return line.clone();
    }
    let hl_bg = theme.colors.search_match.to_ratatui();

    let mut ranges: Vec<(usize, usize, bool)> = matches_on_this_line
        .iter()
        .map(|m| {
            let is_current = current_match
                .map(|c| c.line_index == m.line_index && c.start_col == m.start_col)
                .unwrap_or(false);
            (m.start_col, m.start_col + m.length, is_current)
        })
        .collect();
    ranges.sort_by_key(|r| r.0);

    let mut new_spans: Vec<StyledSpan> = Vec::new();
    let mut col: usize = 0;
    for span in &line.spans {
        let span_len = span_char_count(span);
        if span_len == 0 {
            new_spans.push(span.clone());
            continue;
        }
        let span_start = col;
        let span_end = col + span_len;

        let mut cuts: Vec<usize> = vec![0, span_len];
        for (s, e, _) in &ranges {
            if *e <= span_start || *s >= span_end {
                continue;
            }
            let local_s = s.saturating_sub(span_start);
            let local_e = (*e - span_start).min(span_len);
            cuts.push(local_s);
            cuts.push(local_e);
        }
        cuts.sort_unstable();
        cuts.dedup();

        let mut remaining = span.clone();
        let mut consumed: usize = 0;
        for &cut in cuts.iter().skip(1) {
            let take = cut - consumed;
            if take == 0 {
                continue;
            }
            let (mut piece, rest) = split_span_at_char_offset(&remaining, take);
            remaining = rest;
            let abs_start = span_start + consumed;
            let active = ranges
                .iter()
                .find(|(s, e, _)| *s <= abs_start && abs_start < *e);
            if let Some((_, _, is_current)) = active {
                piece.bg = Some(hl_bg);
                if *is_current {
                    piece.modifier |= Modifier::BOLD;
                }
            }
            new_spans.push(piece);
            consumed = cut;
        }

        col = span_end;
    }

    LayoutLine {
        spans: new_spans,
        block_index: line.block_index,
        heading_anchor: line.heading_anchor.clone(),
        mermaid_block_index: line.mermaid_block_index,
        indent_cols: line.indent_cols,
    }
}

pub struct SearchBarWidget<'a> {
    pub state: &'a SearchState,
    pub theme: &'a Theme,
}

impl<'a> Widget for SearchBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.state.mode == SearchMode::Off || area.height == 0 || area.width == 0 {
            return;
        }
        let bg = self.theme.colors.statusbar_bg.to_ratatui();
        let fg = self.theme.colors.fg.to_ratatui();
        let dim = self.theme.colors.popup_dim.to_ratatui();
        let warn = self.theme.colors.task_pending.to_ratatui();
        let base = Style::default().fg(fg).bg(bg);
        buf.set_style(area, base);

        let y = area.y;

        if self.state.mode == SearchMode::Active && self.state.matches.is_empty() {
            let text = format!("  no matches for {}", self.state.query);
            let style = Style::default().fg(warn).bg(bg);
            buf.set_stringn(area.x, y, &text, area.width as usize, style);
            return;
        }

        let mut left = format!("  /{}", self.state.query);
        if self.state.mode == SearchMode::Editing {
            left.push('█');
        }
        let (nx, _) = buf.set_stringn(area.x, y, &left, area.width as usize, base);

        let total = self.state.matches.len();
        if total > 0 {
            let cur_disp = self.state.current + 1;
            let right_text = format!("   {cur_disp}/{total} matches");
            let right_len = right_text.chars().count() as u16;
            let right_style = Style::default().fg(dim).bg(bg);
            if right_len <= area.width {
                let used = nx.saturating_sub(area.x);
                let right_x = area.x + area.width - right_len;
                if right_x >= area.x + used {
                    buf.set_stringn(right_x, y, &right_text, right_len as usize, right_style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::layout::{LayoutLine, StyledSpan};
    use ratatui::style::Color;

    fn span(text: &str) -> StyledSpan {
        StyledSpan {
            text: text.to_string(),
            fg: Color::Reset,
            bg: None,
            modifier: Modifier::empty(),
            link: None,
        }
    }

    fn line_from(text: &str) -> LayoutLine {
        LayoutLine {
            spans: vec![span(text)],
            block_index: 0,
            heading_anchor: None,
            mermaid_block_index: None,
            indent_cols: 0,
        }
    }

    fn theme() -> Theme {
        Theme::default()
    }

    #[test]
    fn start_enters_editing_mode() {
        let mut s = SearchState::default();
        s.query.push_str("stale");
        s.matches.push(SearchMatch {
            line_index: 0,
            start_col: 0,
            length: 1,
        });
        s.start();
        assert_eq!(s.mode, SearchMode::Editing);
        assert!(s.query.is_empty());
        assert!(s.matches.is_empty());
    }

    #[test]
    fn input_char_appends_to_query() {
        let mut s = SearchState::default();
        s.start();
        s.input_char('a');
        s.input_char('b');
        s.input_char('é');
        assert_eq!(s.query, "abé");
    }

    #[test]
    fn backspace_pops_last_char_or_noop_when_empty() {
        let mut s = SearchState::default();
        s.start();
        s.backspace();
        assert_eq!(s.query, "");
        s.input_char('x');
        s.input_char('y');
        s.backspace();
        assert_eq!(s.query, "x");
    }

    #[test]
    fn confirm_computes_matches_and_enters_active() {
        let lines = vec![line_from("hello world"), line_from("world peace")];
        let mut s = SearchState::default();
        s.start();
        s.input_char('w');
        s.input_char('o');
        s.input_char('r');
        s.input_char('l');
        s.input_char('d');
        s.confirm(&lines);
        assert_eq!(s.mode, SearchMode::Active);
        assert_eq!(s.matches.len(), 2);
        assert_eq!(s.current, 0);
    }

    #[test]
    fn cancel_resets_state() {
        let mut s = SearchState::default();
        s.start();
        s.input_char('a');
        s.cancel();
        assert_eq!(s.mode, SearchMode::Off);
        assert!(s.query.is_empty());
        assert!(s.matches.is_empty());
        assert_eq!(s.current, 0);
    }

    #[test]
    fn next_and_prev_wraparound() {
        let lines = vec![line_from("a a a")];
        let mut s = SearchState::default();
        s.start();
        s.input_char('a');
        s.confirm(&lines);
        assert_eq!(s.matches.len(), 3);
        assert_eq!(s.current, 0);
        s.next();
        assert_eq!(s.current, 1);
        s.next();
        assert_eq!(s.current, 2);
        s.next();
        assert_eq!(s.current, 0);
        s.prev();
        assert_eq!(s.current, 2);
        s.prev();
        assert_eq!(s.current, 1);
    }

    #[test]
    fn current_match_none_when_empty() {
        let mut s = SearchState::default();
        assert!(s.current_match().is_none());
        s.start();
        s.input_char('z');
        s.confirm(&[line_from("hello")]);
        assert!(s.current_match().is_none());
    }

    #[test]
    fn query_all_lower_is_case_insensitive() {
        let mut s = SearchState::default();
        s.start();
        s.input_char('a');
        s.input_char('b');
        assert!(!s.is_smart_case());
    }

    #[test]
    fn query_with_uppercase_is_case_sensitive() {
        let mut s = SearchState::default();
        s.start();
        s.input_char('a');
        s.input_char('B');
        assert!(s.is_smart_case());
    }

    #[test]
    fn case_insensitive_finds_mixed_case_text() {
        let lines = vec![line_from("Hello HELLO hello")];
        let matches = find_matches(&lines, "hello");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].start_col, 0);
        assert_eq!(matches[1].start_col, 6);
        assert_eq!(matches[2].start_col, 12);
    }

    #[test]
    fn case_sensitive_skips_lowercase_when_query_uppercase() {
        let lines = vec![line_from("Hello HELLO hello")];
        let matches = find_matches(&lines, "Hello");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_col, 0);
    }

    #[test]
    fn find_matches_returns_empty_for_no_query() {
        let lines = vec![line_from("anything")];
        assert!(find_matches(&lines, "").is_empty());
    }

    #[test]
    fn find_matches_finds_multiple_on_one_line() {
        let lines = vec![line_from("ababab")];
        let matches = find_matches(&lines, "ab");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].start_col, 0);
        assert_eq!(matches[1].start_col, 2);
        assert_eq!(matches[2].start_col, 4);
    }

    #[test]
    fn find_matches_finds_across_lines() {
        let lines = vec![line_from("foo"), line_from("foofoo")];
        let matches = find_matches(&lines, "foo");
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_index, 0);
        assert_eq!(matches[1].line_index, 1);
        assert_eq!(matches[2].line_index, 1);
        assert_eq!(matches[2].start_col, 3);
    }

    #[test]
    fn find_matches_char_offsets_not_byte_offsets() {
        let lines = vec![line_from("café é")];
        let matches = find_matches(&lines, "é");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start_col, 3);
        assert_eq!(matches[1].start_col, 5);
    }

    #[test]
    fn find_matches_overlapping_skipped() {
        let lines = vec![line_from("aaaa")];
        let matches = find_matches(&lines, "aa");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start_col, 0);
        assert_eq!(matches[1].start_col, 2);
    }

    #[test]
    fn highlight_line_with_no_matches_returns_unchanged() {
        let line = line_from("hello world");
        let out = highlight_line_with_matches(&line, &[], None, &theme());
        assert_eq!(out, line);
    }

    #[test]
    fn highlight_line_marks_match_with_search_bg() {
        let line = line_from("hello world");
        let m = SearchMatch {
            line_index: 0,
            start_col: 6,
            length: 5,
        };
        let theme = theme();
        let out = highlight_line_with_matches(&line, &[&m], None, &theme);
        let hl_bg = theme.colors.search_match.to_ratatui();
        let found = out
            .spans
            .iter()
            .find(|s| s.text == "world")
            .expect("highlighted span exists");
        assert_eq!(found.bg, Some(hl_bg));
        assert!(!found.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn highlight_line_current_match_gets_bold_modifier() {
        let line = line_from("hello world");
        let m = SearchMatch {
            line_index: 0,
            start_col: 6,
            length: 5,
        };
        let theme = theme();
        let out = highlight_line_with_matches(&line, &[&m], Some(&m), &theme);
        let found = out
            .spans
            .iter()
            .find(|s| s.text == "world")
            .expect("highlighted span exists");
        assert!(found.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn split_span_at_char_offset_zero_returns_empty_left() {
        let s = span("hello");
        let (l, r) = split_span_at_char_offset(&s, 0);
        assert_eq!(l.text, "");
        assert_eq!(r.text, "hello");
    }

    #[test]
    fn split_span_at_char_offset_at_end_returns_empty_right() {
        let s = span("hello");
        let (l, r) = split_span_at_char_offset(&s, 5);
        assert_eq!(l.text, "hello");
        assert_eq!(r.text, "");
    }

    #[test]
    fn split_span_at_char_offset_middle_splits_correctly() {
        let s = span("café");
        let (l, r) = split_span_at_char_offset(&s, 3);
        assert_eq!(l.text, "caf");
        assert_eq!(r.text, "é");
    }
}
