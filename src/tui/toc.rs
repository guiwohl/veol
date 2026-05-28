use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::markdown::layout::LayoutLine;
use crate::markdown::model::{Block, Span};
use crate::render::theme::{Color as ThemeColor, Theme};

#[derive(Debug, Clone)]
pub struct TocEntry {
    pub level: u8,
    pub text: String,
    pub anchor: String,
    pub line_index: usize,
}

#[derive(Debug, Default)]
pub struct TocState {
    pub entries: Vec<TocEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
}

impl TocState {
    pub fn build(lines: &[LayoutLine], blocks: &[Block]) -> Self {
        let line_positions: Vec<(String, usize)> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| l.heading_anchor.as_ref().map(|a| (a.clone(), i)))
            .collect();

        let mut consumed = vec![false; line_positions.len()];
        let mut entries = Vec::new();
        for block in flatten_blocks(blocks) {
            if let Block::Heading {
                level,
                spans,
                anchor,
                ..
            } = block
            {
                if let Some(idx) = line_positions
                    .iter()
                    .enumerate()
                    .position(|(i, (a, _))| !consumed[i] && a == anchor)
                {
                    consumed[idx] = true;
                    entries.push(TocEntry {
                        level: *level,
                        text: spans_text(spans),
                        anchor: anchor.clone(),
                        line_index: line_positions[idx].1,
                    });
                }
            }
        }

        Self {
            entries,
            selected: 0,
            scroll_offset: 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self, visible_height: usize) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            if visible_height > 0 && self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected + 1 - visible_height;
            }
        }
    }

    pub fn selected_line(&self) -> Option<usize> {
        self.entries.get(self.selected).map(|e| e.line_index)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn spans_text(spans: &[Span]) -> String {
    let mut out = String::new();
    for s in spans {
        out.push_str(&s.text);
    }
    out
}

fn flatten_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut out = Vec::new();
    for b in blocks {
        flatten_into(b, &mut out);
    }
    out
}

fn flatten_into<'a>(block: &'a Block, out: &mut Vec<&'a Block>) {
    out.push(block);
    match block {
        Block::Quote { blocks } => {
            for b in blocks {
                flatten_into(b, out);
            }
        }
        Block::List { items, .. } => {
            for item in items {
                for b in &item.children {
                    flatten_into(b, out);
                }
            }
        }
        Block::Footnote { blocks, .. } => {
            for b in blocks {
                flatten_into(b, out);
            }
        }
        _ => {}
    }
}

pub struct TocWidget<'a> {
    pub state: &'a TocState,
    pub theme: &'a Theme,
}

const BORDER_HORIZ: char = '─';
const BORDER_VERT: char = '│';
const BORDER_TL: char = '╭';
const BORDER_TR: char = '╮';
const BORDER_BL: char = '╰';
const BORDER_BR: char = '╯';
const TITLE_ICON: char = '\u{f03a}';

fn heading_fg(theme: &Theme, level: u8) -> ThemeColor {
    match level {
        1 => theme.colors.heading_1,
        2 => theme.colors.heading_2,
        3 => theme.colors.heading_3,
        4 => theme.colors.heading_4,
        5 => theme.colors.heading_5,
        _ => theme.colors.heading_6,
    }
}

fn heading_prefix(level: u8) -> String {
    let n = level.clamp(1, 6) as usize;
    "#".repeat(n)
}

impl<'a> Widget for TocWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 6 || area.height < 5 {
            return;
        }

        let bg = self.theme.colors.popup_bg.to_ratatui();
        let border = self.theme.colors.popup_border.to_ratatui();
        let accent = self.theme.colors.popup_accent.to_ratatui();
        let dim = self.theme.colors.popup_dim.to_ratatui();
        let selected_bg = self.theme.colors.popup_selected.to_ratatui();

        let bg_style = Style::default().bg(bg);
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(bg_style);
                }
            }
        }

        let right = area.x + area.width - 1;
        let bottom = area.y + area.height - 1;
        let border_style = Style::default().fg(border).bg(bg);
        for x in area.x + 1..right {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(BORDER_HORIZ);
                cell.set_style(border_style);
            }
            if let Some(cell) = buf.cell_mut((x, bottom)) {
                cell.set_char(BORDER_HORIZ);
                cell.set_style(border_style);
            }
        }
        for y in area.y + 1..bottom {
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_char(BORDER_VERT);
                cell.set_style(border_style);
            }
            if let Some(cell) = buf.cell_mut((right, y)) {
                cell.set_char(BORDER_VERT);
                cell.set_style(border_style);
            }
        }
        for (px, py, ch) in [
            (area.x, area.y, BORDER_TL),
            (right, area.y, BORDER_TR),
            (area.x, bottom, BORDER_BL),
            (right, bottom, BORDER_BR),
        ] {
            if let Some(cell) = buf.cell_mut((px, py)) {
                cell.set_char(ch);
                cell.set_style(border_style);
            }
        }

        let title = format!(" {TITLE_ICON} Table of Contents ");
        let title_style = Style::default()
            .fg(accent)
            .bg(bg)
            .add_modifier(Modifier::BOLD);
        let title_start = area.x + 2;
        buf.set_string(title_start, area.y, &title, title_style);

        let inner_left = area.x + 1;
        let inner_right = right;
        let inner_width = inner_right.saturating_sub(inner_left);
        let list_top = area.y + 1;
        let hint_y = bottom.saturating_sub(1);
        let list_bottom = if hint_y > list_top { hint_y } else { list_top };
        let visible_rows = list_bottom.saturating_sub(list_top) as usize;

        if self.state.entries.is_empty() {
            let empty = "(no headings)";
            let empty_style = Style::default().fg(dim).bg(bg);
            let empty_len = empty.chars().count() as u16;
            let cy = list_top + (visible_rows as u16) / 2;
            let cx = if inner_width > empty_len {
                inner_left + (inner_width - empty_len) / 2
            } else {
                inner_left
            };
            buf.set_string(cx, cy, empty, empty_style);
        } else {
            let scroll_offset = compute_scroll_offset(
                self.state.scroll_offset,
                self.state.selected,
                self.state.entries.len(),
                visible_rows,
            );

            for row in 0..visible_rows {
                let idx = scroll_offset + row;
                if idx >= self.state.entries.len() {
                    break;
                }
                let y = list_top + row as u16;
                let entry = &self.state.entries[idx];
                let is_selected = idx == self.state.selected;
                let row_bg = if is_selected { selected_bg } else { bg };

                for x in inner_left..inner_right {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char(' ');
                        cell.set_style(Style::default().bg(row_bg));
                    }
                }

                let indent = "  ".repeat(entry.level.saturating_sub(1) as usize);
                let prefix = heading_prefix(entry.level);
                let prefix_style = Style::default().fg(dim).bg(row_bg);
                let text_fg = heading_fg(self.theme, entry.level).to_ratatui();
                let mut text_style = Style::default().fg(text_fg).bg(row_bg);
                if is_selected {
                    text_style = text_style.add_modifier(Modifier::BOLD);
                }

                let mut cx = inner_left + 1;
                let max_x = inner_right;

                for ch in indent.chars() {
                    if cx >= max_x {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((cx, y)) {
                        cell.set_char(ch);
                        cell.set_style(Style::default().bg(row_bg));
                    }
                    cx += 1;
                }
                for ch in prefix.chars() {
                    if cx >= max_x {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((cx, y)) {
                        cell.set_char(ch);
                        cell.set_style(prefix_style);
                    }
                    cx += 1;
                }
                if cx < max_x {
                    if let Some(cell) = buf.cell_mut((cx, y)) {
                        cell.set_char(' ');
                        cell.set_style(Style::default().bg(row_bg));
                    }
                    cx += 1;
                }
                for ch in entry.text.chars() {
                    if cx >= max_x {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((cx, y)) {
                        cell.set_char(ch);
                        cell.set_style(text_style);
                    }
                    cx += 1;
                }
            }
        }

        let hint = " \u{2191}\u{2193} navigate  \u{23ce} jump  esc close ";
        let hint_style = Style::default().fg(dim).bg(bg);
        let hint_chars: Vec<char> = hint.chars().collect();
        let hint_len = hint_chars.len() as u16;
        if inner_width > hint_len {
            let hint_x = inner_left + (inner_width - hint_len) / 2;
            buf.set_string(hint_x, hint_y, hint, hint_style);
        } else {
            buf.set_string(inner_left, hint_y, hint, hint_style);
        }
    }
}

fn compute_scroll_offset(current: usize, selected: usize, total: usize, visible: usize) -> usize {
    if visible == 0 || total <= visible {
        return 0;
    }
    let mut offset = current;
    if selected < offset {
        offset = selected;
    }
    if selected >= offset + visible {
        offset = selected + 1 - visible;
    }
    offset.min(total.saturating_sub(visible))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::layout::layout;
    use crate::markdown::parse;

    fn theme() -> Theme {
        Theme::default()
    }

    fn build(src: &str, width: u16) -> (Vec<Block>, Vec<LayoutLine>) {
        let blocks = parse(src);
        let lines = layout(&blocks, width, &theme());
        (blocks, lines)
    }

    #[test]
    fn build_empty_returns_empty() {
        let (blocks, lines) = build("just a paragraph here\n", 60);
        let state = TocState::build(&lines, &blocks);
        assert!(state.entries.is_empty());
        assert!(state.is_empty());
    }

    #[test]
    fn build_returns_one_entry_per_heading() {
        let (blocks, lines) = build("# A\n\npara\n\n## B\n\n### C\n", 60);
        let state = TocState::build(&lines, &blocks);
        assert_eq!(state.entries.len(), 3);
    }

    #[test]
    fn build_records_correct_levels_and_anchors() {
        let src = "# One\n\n## Two\n\n### Three\n\n## Two B\n";
        let (blocks, lines) = build(src, 60);
        let state = TocState::build(&lines, &blocks);
        let levels: Vec<u8> = state.entries.iter().map(|e| e.level).collect();
        let texts: Vec<&str> = state.entries.iter().map(|e| e.text.as_str()).collect();
        let anchors: Vec<&str> = state.entries.iter().map(|e| e.anchor.as_str()).collect();
        assert_eq!(levels, vec![1, 2, 3, 2]);
        assert_eq!(texts, vec!["One", "Two", "Three", "Two B"]);
        assert_eq!(anchors, vec!["one", "two", "three", "two-b"]);
    }

    #[test]
    fn build_records_line_index_matching_layout_heading_position() {
        let src = "# A\n\npara line\n\n## B\n";
        let (blocks, lines) = build(src, 60);
        let state = TocState::build(&lines, &blocks);

        let heading_positions: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.heading_anchor.is_some())
            .map(|(i, _)| i)
            .collect();

        let entry_lines: Vec<usize> = state.entries.iter().map(|e| e.line_index).collect();
        assert_eq!(entry_lines, heading_positions);
    }

    #[test]
    fn move_up_at_zero_stays_at_zero() {
        let (blocks, lines) = build("# A\n\n# B\n", 60);
        let mut state = TocState::build(&lines, &blocks);
        assert_eq!(state.selected, 0);
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_down_at_last_stays_at_last() {
        let (blocks, lines) = build("# A\n\n# B\n", 60);
        let mut state = TocState::build(&lines, &blocks);
        state.selected = state.entries.len() - 1;
        state.move_down(5);
        assert_eq!(state.selected, state.entries.len() - 1);
    }

    #[test]
    fn selected_line_returns_correct_index() {
        let (blocks, lines) = build("# A\n\n## B\n", 60);
        let mut state = TocState::build(&lines, &blocks);
        state.selected = 1;
        let expected = state.entries[1].line_index;
        assert_eq!(state.selected_line(), Some(expected));
    }

    #[test]
    fn selected_line_returns_none_when_empty() {
        let (blocks, lines) = build("plain paragraph\n", 60);
        let state = TocState::build(&lines, &blocks);
        assert_eq!(state.selected_line(), None);
    }

    fn render_buffer_text(buf: &Buffer) -> String {
        let area = buf.area();
        let mut out = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                let pos = (area.x + x, area.y + y);
                if let Some(cell) = buf.cell(pos) {
                    out.push_str(cell.symbol());
                }
            }
            out.push('\n');
        }
        out.trim_end_matches('\n').to_string()
    }

    #[test]
    fn widget_renders_multilevel_toc() {
        let src = "# Alpha\n\n## Beta\n\n### Gamma\n\n## Delta\n";
        let (blocks, lines) = build(src, 60);
        let state = TocState::build(&lines, &blocks);
        let t = theme();
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        TocWidget {
            state: &state,
            theme: &t,
        }
        .render(area, &mut buf);
        insta::assert_snapshot!("widget_renders_multilevel_toc", render_buffer_text(&buf));
    }

    #[test]
    fn widget_renders_empty_state() {
        let (blocks, lines) = build("just text, no headings\n", 60);
        let state = TocState::build(&lines, &blocks);
        let t = theme();
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        TocWidget {
            state: &state,
            theme: &t,
        }
        .render(area, &mut buf);
        insta::assert_snapshot!("widget_renders_empty_state", render_buffer_text(&buf));
    }

    #[test]
    fn widget_highlights_selected_row() {
        let src = "# A\n\n## B\n\n## C\n";
        let (blocks, lines) = build(src, 60);
        let mut state = TocState::build(&lines, &blocks);
        state.selected = 1;
        let t = theme();
        let selected_bg = t.colors.popup_selected.to_ratatui();
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        TocWidget {
            state: &state,
            theme: &t,
        }
        .render(area, &mut buf);

        let mut selected_rows = 0;
        for y in area.y + 1..area.y + area.height - 2 {
            let mut row_matches = 0;
            for x in area.x + 1..area.x + area.width - 1 {
                if let Some(cell) = buf.cell((x, y)) {
                    if cell.bg == selected_bg {
                        row_matches += 1;
                    }
                }
            }
            if row_matches > 5 {
                selected_rows += 1;
            }
        }
        assert_eq!(
            selected_rows, 1,
            "expected exactly one row tinted with popup_selected bg"
        );
    }
}
