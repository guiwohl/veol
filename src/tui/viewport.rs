use crate::markdown::layout::LayoutLine;

#[derive(Debug, Clone, Default)]
pub struct ViewportState {
    pub top_line: usize,
    pub height: usize,
    pub total_lines: usize,
    pub cursor_line: Option<usize>,
}

impl ViewportState {
    pub fn new(total_lines: usize, height: usize) -> Self {
        let mut v = Self {
            top_line: 0,
            height,
            total_lines,
            cursor_line: None,
        };
        v.clamp_top();
        v
    }

    pub fn set_total(&mut self, total: usize) {
        self.total_lines = total;
        self.clamp_top();
    }

    pub fn set_height(&mut self, h: usize) {
        self.height = h;
        self.clamp_top();
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.top_line = self.top_line.saturating_add(n);
        self.clamp_top();
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.top_line = self.top_line.saturating_sub(n);
    }

    pub fn page_down(&mut self) {
        self.scroll_down(self.height.saturating_sub(2));
    }

    pub fn page_up(&mut self) {
        self.scroll_up(self.height.saturating_sub(2));
    }

    pub fn half_page_down(&mut self) {
        self.scroll_down(self.height / 2);
    }

    pub fn half_page_up(&mut self) {
        self.scroll_up(self.height / 2);
    }

    pub fn jump_top(&mut self) {
        self.top_line = 0;
    }

    pub fn jump_bottom(&mut self) {
        self.top_line = self.max_top();
    }

    pub fn jump_to_line(&mut self, line: usize) {
        self.top_line = line;
        self.clamp_top();
    }

    pub fn next_heading(&mut self, lines: &[LayoutLine]) {
        let start = self.top_line.saturating_add(1);
        if let Some(idx) = lines
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, l)| l.heading_anchor.is_some())
            .map(|(i, _)| i)
        {
            self.top_line = idx;
            self.clamp_top();
        }
    }

    pub fn prev_heading(&mut self, lines: &[LayoutLine]) {
        if self.top_line == 0 {
            return;
        }
        if let Some(idx) = lines
            .iter()
            .enumerate()
            .take(self.top_line)
            .rev()
            .find(|(_, l)| l.heading_anchor.is_some())
            .map(|(i, _)| i)
        {
            self.top_line = idx;
        }
    }

    pub fn next_paragraph(&mut self, lines: &[LayoutLine]) {
        let max = lines.len().saturating_sub(1);
        let mut i = self.top_line;
        while i < max && !is_blank(&lines[i]) {
            i += 1;
        }
        while i < max && is_blank(&lines[i]) {
            i += 1;
        }
        self.top_line = i;
        self.clamp_top();
    }

    pub fn prev_paragraph(&mut self, lines: &[LayoutLine]) {
        if self.top_line == 0 || lines.is_empty() {
            return;
        }
        let mut i = self.top_line.saturating_sub(1);
        while i > 0 && is_blank(&lines[i]) {
            i -= 1;
        }
        while i > 0 && !is_blank(&lines[i - 1]) {
            i -= 1;
        }
        self.top_line = i;
    }

    pub fn visible_range(&self) -> std::ops::Range<usize> {
        let end = self
            .top_line
            .saturating_add(self.height)
            .min(self.total_lines);
        self.top_line..end
    }

    pub fn percent(&self) -> u8 {
        if self.total_lines == 0 {
            return 100;
        }
        if self.total_lines <= self.height {
            return 100;
        }
        let max = self.total_lines - self.height;
        let denom = max.max(1);
        ((self.top_line * 100 / denom).min(100)) as u8
    }

    fn max_top(&self) -> usize {
        self.total_lines.saturating_sub(self.height)
    }

    fn clamp_top(&mut self) {
        let max = self.max_top();
        if self.top_line > max {
            self.top_line = max;
        }
    }
}

fn is_blank(line: &LayoutLine) -> bool {
    line.spans.iter().all(|s| s.text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};

    use crate::markdown::layout::{LayoutLine, StyledSpan};

    fn blank_line() -> LayoutLine {
        LayoutLine {
            spans: vec![StyledSpan {
                text: String::new(),
                fg: Color::Reset,
                bg: None,
                modifier: Modifier::empty(),
                link: None,
            }],
            block_index: 0,
            heading_anchor: None,
            mermaid_block_index: None,
            indent_cols: 0,
        }
    }

    fn heading_line(anchor: &str) -> LayoutLine {
        let mut l = blank_line();
        l.heading_anchor = Some(anchor.to_string());
        l
    }

    fn text_line(text: &str) -> LayoutLine {
        LayoutLine {
            spans: vec![StyledSpan {
                text: text.to_string(),
                fg: Color::Reset,
                bg: None,
                modifier: Modifier::empty(),
                link: None,
            }],
            block_index: 0,
            heading_anchor: None,
            mermaid_block_index: None,
            indent_cols: 0,
        }
    }

    #[test]
    fn scroll_down_clamps_at_end() {
        let mut v = ViewportState::new(100, 10);
        v.scroll_down(500);
        assert_eq!(v.top_line, 90);
    }

    #[test]
    fn scroll_up_saturates_at_zero() {
        let mut v = ViewportState::new(100, 10);
        v.top_line = 5;
        v.scroll_up(50);
        assert_eq!(v.top_line, 0);
    }

    #[test]
    fn page_down_overlaps_by_two() {
        let mut v = ViewportState::new(100, 10);
        v.page_down();
        assert_eq!(v.top_line, 8);
    }

    #[test]
    fn page_up_overlaps_by_two() {
        let mut v = ViewportState::new(100, 10);
        v.top_line = 50;
        v.page_up();
        assert_eq!(v.top_line, 42);
    }

    #[test]
    fn half_page_uses_height_div_two() {
        let mut v = ViewportState::new(100, 10);
        v.half_page_down();
        assert_eq!(v.top_line, 5);
        v.half_page_up();
        assert_eq!(v.top_line, 0);
    }

    #[test]
    fn jump_top_and_bottom() {
        let mut v = ViewportState::new(100, 10);
        v.jump_bottom();
        assert_eq!(v.top_line, 90);
        v.jump_top();
        assert_eq!(v.top_line, 0);
    }

    #[test]
    fn next_heading_skips_top_line_if_already_heading() {
        let lines = vec![
            heading_line("a"),
            blank_line(),
            heading_line("b"),
            blank_line(),
            heading_line("c"),
        ];
        let mut v = ViewportState::new(lines.len(), 1);
        v.top_line = 0;
        v.next_heading(&lines);
        assert_eq!(v.top_line, 2);
        v.next_heading(&lines);
        assert_eq!(v.top_line, 4);
        v.next_heading(&lines);
        assert_eq!(v.top_line, 4);
    }

    #[test]
    fn prev_heading_finds_earliest_above() {
        let lines = vec![
            heading_line("a"),
            blank_line(),
            heading_line("b"),
            blank_line(),
            heading_line("c"),
        ];
        let mut v = ViewportState::new(lines.len(), 1);
        v.top_line = 4;
        v.prev_heading(&lines);
        assert_eq!(v.top_line, 2);
        v.prev_heading(&lines);
        assert_eq!(v.top_line, 0);
        v.prev_heading(&lines);
        assert_eq!(v.top_line, 0);
    }

    #[test]
    fn percent_zero_when_total_lte_height() {
        let v = ViewportState::new(5, 10);
        assert_eq!(v.percent(), 100);
        let v = ViewportState::new(0, 10);
        assert_eq!(v.percent(), 100);
    }

    #[test]
    fn percent_hundred_at_bottom() {
        let mut v = ViewportState::new(100, 10);
        v.jump_bottom();
        assert_eq!(v.percent(), 100);
    }

    #[test]
    fn next_paragraph_skips_current_block_then_blanks() {
        let lines = vec![
            text_line("para one line 1"),
            text_line("para one line 2"),
            blank_line(),
            blank_line(),
            text_line("para two line 1"),
            text_line("para two line 2"),
            blank_line(),
            text_line("para three"),
        ];
        let mut v = ViewportState::new(lines.len(), 1);
        v.top_line = 0;
        v.next_paragraph(&lines);
        assert_eq!(v.top_line, 4);
        v.next_paragraph(&lines);
        assert_eq!(v.top_line, 7);
        v.next_paragraph(&lines);
        assert_eq!(v.top_line, 7);
    }

    #[test]
    fn next_paragraph_from_blank_advances_to_next_block() {
        let lines = vec![
            text_line("first"),
            blank_line(),
            text_line("second"),
            blank_line(),
            text_line("third"),
        ];
        let mut v = ViewportState::new(lines.len(), 1);
        v.top_line = 1;
        v.next_paragraph(&lines);
        assert_eq!(v.top_line, 2);
    }

    #[test]
    fn prev_paragraph_lands_on_first_line_of_previous_block() {
        let lines = vec![
            text_line("para one line 1"),
            text_line("para one line 2"),
            blank_line(),
            text_line("para two line 1"),
            text_line("para two line 2"),
            blank_line(),
            text_line("para three"),
        ];
        let mut v = ViewportState::new(lines.len(), 1);
        v.top_line = 6;
        v.prev_paragraph(&lines);
        assert_eq!(v.top_line, 3);
        v.prev_paragraph(&lines);
        assert_eq!(v.top_line, 0);
        v.prev_paragraph(&lines);
        assert_eq!(v.top_line, 0);
    }

    #[test]
    fn prev_paragraph_from_within_block_lands_on_block_start() {
        let lines = vec![
            text_line("p1 a"),
            text_line("p1 b"),
            blank_line(),
            text_line("p2 a"),
            text_line("p2 b"),
            text_line("p2 c"),
        ];
        let mut v = ViewportState::new(lines.len(), 1);
        v.top_line = 5;
        v.prev_paragraph(&lines);
        assert_eq!(v.top_line, 3);
    }

    #[test]
    fn paragraph_navigation_no_op_on_empty_doc() {
        let lines: Vec<LayoutLine> = vec![];
        let mut v = ViewportState::new(0, 10);
        v.next_paragraph(&lines);
        assert_eq!(v.top_line, 0);
        v.prev_paragraph(&lines);
        assert_eq!(v.top_line, 0);
    }

    #[test]
    fn set_total_clamps_top_line() {
        let mut v = ViewportState::new(1000, 10);
        v.top_line = 900;
        v.set_total(50);
        assert!(
            v.top_line <= 40,
            "top_line should clamp, got {}",
            v.top_line
        );
        assert_eq!(v.top_line, 40);
    }
}
