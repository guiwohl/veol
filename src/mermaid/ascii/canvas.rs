use super::charset::{get_charset, CharsetKind};
use ratatui::style::Color;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, PartialEq)]
pub struct StyledRun {
    pub text: String,
    pub color: Option<Color>,
}

pub type StyledRow = Vec<StyledRun>;

#[derive(Debug, Clone)]
pub struct Canvas {
    rows: Vec<Vec<char>>,
    colors: Vec<Vec<Option<Color>>>,
    charset_kind: CharsetKind,
}

impl Canvas {
    pub fn new(width: usize, height: usize, charset_kind: CharsetKind) -> Self {
        let rows = vec![vec![' '; width]; height];
        let colors = vec![vec![None; width]; height];
        Self {
            rows,
            colors,
            charset_kind,
        }
    }

    pub fn width(&self) -> usize {
        self.rows.first().map(|r| r.len()).unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn charset_kind(&self) -> CharsetKind {
        self.charset_kind
    }

    pub fn ensure_size(&mut self, width: usize, height: usize) {
        let cur_w = self.width();
        if width > cur_w {
            for row in self.rows.iter_mut() {
                row.resize(width, ' ');
            }
            for row in self.colors.iter_mut() {
                row.resize(width, None);
            }
        }
        let target_w = self.width().max(width);
        while self.rows.len() < height {
            self.rows.push(vec![' '; target_w]);
            self.colors.push(vec![None; target_w]);
        }
    }

    pub fn put_char(&mut self, x: usize, y: usize, c: char) {
        if y >= self.rows.len() {
            return;
        }
        if x >= self.rows[y].len() {
            return;
        }
        self.rows[y][x] = c;
    }

    pub fn put_str(&mut self, x: usize, y: usize, s: &str) {
        if y >= self.rows.len() {
            return;
        }
        let mut cx = x;
        for c in s.chars() {
            let w = UnicodeWidthChar::width(c).unwrap_or(1).max(1);
            if cx >= self.rows[y].len() {
                break;
            }
            self.rows[y][cx] = c;
            cx += w;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> char {
        if y >= self.rows.len() {
            return ' ';
        }
        if x >= self.rows[y].len() {
            return ' ';
        }
        self.rows[y][x]
    }

    pub fn set_color(&mut self, x: usize, y: usize, c: Color) {
        if y >= self.colors.len() {
            return;
        }
        if x >= self.colors[y].len() {
            return;
        }
        self.colors[y][x] = Some(c);
    }

    pub fn paint_hline(&mut self, x1: usize, x2: usize, y: usize, c: Color) {
        if y >= self.colors.len() {
            return;
        }
        let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let row_len = self.colors[y].len();
        for x in lo..=hi {
            if x >= row_len {
                break;
            }
            self.colors[y][x] = Some(c);
        }
    }

    pub fn paint_vline(&mut self, x: usize, y1: usize, y2: usize, c: Color) {
        let (lo, hi) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        for y in lo..=hi {
            if y >= self.colors.len() {
                break;
            }
            if x >= self.colors[y].len() {
                continue;
            }
            self.colors[y][x] = Some(c);
        }
    }

    pub fn paint_box(&mut self, x1: usize, y1: usize, x2: usize, y2: usize, c: Color) {
        let (lx, rx) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (ty, by) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        self.paint_hline(lx, rx, ty, c);
        self.paint_hline(lx, rx, by, c);
        self.paint_vline(lx, ty, by, c);
        self.paint_vline(rx, ty, by, c);
    }

    pub fn paint_text(&mut self, x: usize, y: usize, s: &str, c: Color) {
        if y >= self.colors.len() {
            return;
        }
        let mut cx = x;
        let row_len = self.colors[y].len();
        for ch in s.chars() {
            let w = UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
            if cx >= row_len {
                break;
            }
            self.colors[y][cx] = Some(c);
            cx += w;
        }
    }

    pub fn put_str_colored(&mut self, x: usize, y: usize, s: &str, c: Color) {
        self.put_str(x, y, s);
        self.paint_text(x, y, s, c);
    }

    pub fn draw_hline(&mut self, x1: usize, x2: usize, y: usize) {
        let cs = get_charset(self.charset_kind);
        let h = cs.horiz();
        let (lo, hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        for x in lo..=hi {
            self.put_char(x, y, h);
        }
    }

    pub fn draw_vline(&mut self, x: usize, y1: usize, y2: usize) {
        let cs = get_charset(self.charset_kind);
        let v = cs.vert();
        let (lo, hi) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        for y in lo..=hi {
            self.put_char(x, y, v);
        }
    }

    pub fn draw_box(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let cs = get_charset(self.charset_kind);
        let (lx, rx) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (ty, by) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        if rx > lx + 1 {
            for x in (lx + 1)..rx {
                self.put_char(x, ty, cs.horiz());
                self.put_char(x, by, cs.horiz());
            }
        }
        if by > ty + 1 {
            for y in (ty + 1)..by {
                self.put_char(lx, y, cs.vert());
                self.put_char(rx, y, cs.vert());
            }
        }
        self.put_char(lx, ty, cs.top_left());
        self.put_char(rx, ty, cs.top_right());
        self.put_char(lx, by, cs.bot_left());
        self.put_char(rx, by, cs.bot_right());
    }

    pub fn merge(&mut self, other: &Canvas, offset_x: usize, offset_y: usize) {
        let need_w = offset_x + other.width();
        let need_h = offset_y + other.height();
        self.ensure_size(need_w, need_h);
        let is_unicode = matches!(self.charset_kind, CharsetKind::Unicode);
        for (dy, row) in other.rows.iter().enumerate() {
            for (dx, &c) in row.iter().enumerate() {
                if c == ' ' {
                    continue;
                }
                let tx = offset_x + dx;
                let ty = offset_y + dy;
                let cur = self.rows[ty][tx];
                if is_unicode && is_junction_char(c) && is_junction_char(cur) {
                    self.rows[ty][tx] = merge_junctions(cur, c);
                } else {
                    self.rows[ty][tx] = c;
                }
                if let Some(color) = other.colors[dy][dx] {
                    self.colors[ty][tx] = Some(color);
                }
            }
        }
    }

    pub fn as_lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| r.iter().collect::<String>().trim_end().to_string())
            .collect()
    }

    pub fn into_lines(self) -> Vec<String> {
        self.rows
            .into_iter()
            .map(|r| r.into_iter().collect::<String>().trim_end().to_string())
            .collect()
    }

    pub fn as_styled_lines(&self) -> Vec<StyledRow> {
        self.rows
            .iter()
            .zip(self.colors.iter())
            .map(|(chars, colors)| build_styled_row(chars, colors))
            .collect()
    }

    pub fn into_styled_lines(self) -> Vec<StyledRow> {
        self.rows
            .iter()
            .zip(self.colors.iter())
            .map(|(chars, colors)| build_styled_row(chars, colors))
            .collect()
    }
}

fn build_styled_row(chars: &[char], colors: &[Option<Color>]) -> StyledRow {
    let mut runs: Vec<StyledRun> = Vec::new();
    let mut cur_text = String::new();
    let mut cur_color: Option<Color> = None;
    let mut started = false;
    for (i, &c) in chars.iter().enumerate() {
        let color = colors.get(i).copied().flatten();
        if !started {
            cur_color = color;
            cur_text.push(c);
            started = true;
        } else if color == cur_color {
            cur_text.push(c);
        } else {
            runs.push(StyledRun {
                text: std::mem::take(&mut cur_text),
                color: cur_color,
            });
            cur_color = color;
            cur_text.push(c);
        }
    }
    if started {
        runs.push(StyledRun {
            text: cur_text,
            color: cur_color,
        });
    }
    while let Some(last) = runs.last() {
        if last.color.is_none() {
            let trimmed = last.text.trim_end_matches(' ');
            if trimmed.is_empty() {
                runs.pop();
                continue;
            }
            if trimmed.len() != last.text.len() {
                let new_text = trimmed.to_string();
                let last = runs.last_mut().unwrap();
                last.text = new_text;
            }
        }
        break;
    }
    runs
}

pub fn is_junction_char(c: char) -> bool {
    matches!(
        c,
        '─' | '│'
            | '┌'
            | '┐'
            | '└'
            | '┘'
            | '├'
            | '┤'
            | '┬'
            | '┴'
            | '┼'
            | '╴'
            | '╵'
            | '╶'
            | '╷'
    )
}

pub fn merge_junctions(a: char, b: char) -> char {
    match (a, b) {
        ('─', '│') => '┼',
        ('─', '┌') => '┬',
        ('─', '┐') => '┬',
        ('─', '└') => '┴',
        ('─', '┘') => '┴',
        ('─', '├') => '┼',
        ('─', '┤') => '┼',
        ('─', '┬') => '┬',
        ('─', '┴') => '┴',

        ('│', '─') => '┼',
        ('│', '┌') => '├',
        ('│', '┐') => '┤',
        ('│', '└') => '├',
        ('│', '┘') => '┤',
        ('│', '├') => '├',
        ('│', '┤') => '┤',
        ('│', '┬') => '┼',
        ('│', '┴') => '┼',

        ('┌', '─') => '┬',
        ('┌', '│') => '├',
        ('┌', '┐') => '┬',
        ('┌', '└') => '├',
        ('┌', '┘') => '┼',
        ('┌', '├') => '├',
        ('┌', '┤') => '┼',
        ('┌', '┬') => '┬',
        ('┌', '┴') => '┼',

        ('┐', '─') => '┬',
        ('┐', '│') => '┤',
        ('┐', '┌') => '┬',
        ('┐', '└') => '┼',
        ('┐', '┘') => '┤',
        ('┐', '├') => '┼',
        ('┐', '┤') => '┤',
        ('┐', '┬') => '┬',
        ('┐', '┴') => '┼',

        ('└', '─') => '┴',
        ('└', '│') => '├',
        ('└', '┌') => '├',
        ('└', '┐') => '┼',
        ('└', '┘') => '┴',
        ('└', '├') => '├',
        ('└', '┤') => '┼',
        ('└', '┬') => '┼',
        ('└', '┴') => '┴',

        ('┘', '─') => '┴',
        ('┘', '│') => '┤',
        ('┘', '┌') => '┼',
        ('┘', '┐') => '┤',
        ('┘', '└') => '┴',
        ('┘', '├') => '┼',
        ('┘', '┤') => '┤',
        ('┘', '┬') => '┼',
        ('┘', '┴') => '┴',

        ('├', '─') => '┼',
        ('├', '│') => '├',
        ('├', '┌') => '├',
        ('├', '┐') => '┼',
        ('├', '└') => '├',
        ('├', '┘') => '┼',
        ('├', '┤') => '┼',
        ('├', '┬') => '┼',
        ('├', '┴') => '┼',

        ('┤', '─') => '┼',
        ('┤', '│') => '┤',
        ('┤', '┌') => '┼',
        ('┤', '┐') => '┤',
        ('┤', '└') => '┼',
        ('┤', '┘') => '┤',
        ('┤', '├') => '┼',
        ('┤', '┬') => '┼',
        ('┤', '┴') => '┼',

        ('┬', '─') => '┬',
        ('┬', '│') => '┼',
        ('┬', '┌') => '┬',
        ('┬', '┐') => '┬',
        ('┬', '└') => '┼',
        ('┬', '┘') => '┼',
        ('┬', '├') => '┼',
        ('┬', '┤') => '┼',
        ('┬', '┴') => '┼',

        ('┴', '─') => '┴',
        ('┴', '│') => '┼',
        ('┴', '┌') => '┼',
        ('┴', '┐') => '┼',
        ('┴', '└') => '┴',
        ('┴', '┘') => '┴',
        ('┴', '├') => '┼',
        ('┴', '┤') => '┼',
        ('┴', '┬') => '┼',

        _ => a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_new_has_correct_dims() {
        let c = Canvas::new(10, 5, CharsetKind::Unicode);
        assert_eq!(c.width(), 10);
        assert_eq!(c.height(), 5);
    }

    #[test]
    fn canvas_put_char_in_bounds() {
        let mut c = Canvas::new(5, 3, CharsetKind::Unicode);
        c.put_char(2, 1, 'X');
        assert_eq!(c.get(2, 1), 'X');
    }

    #[test]
    fn canvas_put_char_out_of_bounds_is_noop() {
        let mut c = Canvas::new(5, 3, CharsetKind::Unicode);
        c.put_char(10, 1, 'X');
        c.put_char(1, 10, 'Y');
        assert_eq!(c.get(1, 1), ' ');
    }

    #[test]
    fn canvas_put_str_writes_chars() {
        let mut c = Canvas::new(10, 1, CharsetKind::Unicode);
        c.put_str(1, 0, "hi");
        assert_eq!(c.get(1, 0), 'h');
        assert_eq!(c.get(2, 0), 'i');
    }

    #[test]
    fn canvas_put_str_respects_unicode_width() {
        let mut c = Canvas::new(10, 1, CharsetKind::Unicode);
        c.put_str(0, 0, "日a");
        assert_eq!(c.get(0, 0), '日');
        assert_eq!(c.get(2, 0), 'a');
    }

    #[test]
    fn canvas_into_lines_trims_trailing_whitespace() {
        let mut c = Canvas::new(10, 2, CharsetKind::Unicode);
        c.put_char(0, 0, 'A');
        let lines = c.into_lines();
        assert_eq!(lines[0], "A");
        assert_eq!(lines[1], "");
    }

    #[test]
    fn canvas_into_lines_preserves_internal_spaces() {
        let mut c = Canvas::new(10, 1, CharsetKind::Unicode);
        c.put_char(0, 0, 'A');
        c.put_char(3, 0, 'B');
        let lines = c.into_lines();
        assert_eq!(lines[0], "A  B");
    }

    #[test]
    fn draw_hline_unicode() {
        let mut c = Canvas::new(6, 1, CharsetKind::Unicode);
        c.draw_hline(1, 4, 0);
        assert_eq!(c.get(0, 0), ' ');
        assert_eq!(c.get(1, 0), '─');
        assert_eq!(c.get(4, 0), '─');
        assert_eq!(c.get(5, 0), ' ');
    }

    #[test]
    fn draw_hline_ascii() {
        let mut c = Canvas::new(6, 1, CharsetKind::Ascii);
        c.draw_hline(0, 5, 0);
        assert_eq!(c.get(0, 0), '-');
        assert_eq!(c.get(5, 0), '-');
    }

    #[test]
    fn draw_vline_unicode() {
        let mut c = Canvas::new(1, 5, CharsetKind::Unicode);
        c.draw_vline(0, 1, 3);
        assert_eq!(c.get(0, 0), ' ');
        assert_eq!(c.get(0, 1), '│');
        assert_eq!(c.get(0, 3), '│');
        assert_eq!(c.get(0, 4), ' ');
    }

    #[test]
    fn draw_vline_ascii() {
        let mut c = Canvas::new(1, 3, CharsetKind::Ascii);
        c.draw_vline(0, 0, 2);
        assert_eq!(c.get(0, 0), '|');
        assert_eq!(c.get(0, 2), '|');
    }

    #[test]
    fn draw_box_unicode_corners() {
        let mut c = Canvas::new(5, 4, CharsetKind::Unicode);
        c.draw_box(0, 0, 4, 3);
        assert_eq!(c.get(0, 0), '┌');
        assert_eq!(c.get(4, 0), '┐');
        assert_eq!(c.get(0, 3), '└');
        assert_eq!(c.get(4, 3), '┘');
        assert_eq!(c.get(1, 0), '─');
        assert_eq!(c.get(0, 1), '│');
    }

    #[test]
    fn draw_box_ascii_corners() {
        let mut c = Canvas::new(5, 4, CharsetKind::Ascii);
        c.draw_box(0, 0, 4, 3);
        assert_eq!(c.get(0, 0), '+');
        assert_eq!(c.get(4, 0), '+');
        assert_eq!(c.get(0, 3), '+');
        assert_eq!(c.get(4, 3), '+');
        assert_eq!(c.get(2, 0), '-');
        assert_eq!(c.get(0, 2), '|');
    }

    #[test]
    fn draw_box_empty_interior() {
        let mut c = Canvas::new(5, 4, CharsetKind::Unicode);
        c.draw_box(0, 0, 4, 3);
        assert_eq!(c.get(1, 1), ' ');
        assert_eq!(c.get(2, 2), ' ');
        assert_eq!(c.get(3, 1), ' ');
    }

    #[test]
    fn merge_overlay_horiz_and_vert_makes_cross() {
        let mut base = Canvas::new(3, 3, CharsetKind::Unicode);
        base.draw_hline(0, 2, 1);
        let mut overlay = Canvas::new(3, 3, CharsetKind::Unicode);
        overlay.draw_vline(1, 0, 2);
        base.merge(&overlay, 0, 0);
        assert_eq!(base.get(1, 1), '┼');
    }

    #[test]
    fn merge_overlay_corner_and_horiz() {
        let mut base = Canvas::new(3, 3, CharsetKind::Unicode);
        base.put_char(1, 1, '─');
        let mut overlay = Canvas::new(3, 3, CharsetKind::Unicode);
        overlay.put_char(1, 1, '┌');
        base.merge(&overlay, 0, 0);
        assert_eq!(base.get(1, 1), '┬');
    }

    #[test]
    fn merge_overlay_tee_combinations() {
        let mut base = Canvas::new(3, 3, CharsetKind::Unicode);
        base.put_char(1, 1, '├');
        let mut overlay = Canvas::new(3, 3, CharsetKind::Unicode);
        overlay.put_char(1, 1, '┤');
        base.merge(&overlay, 0, 0);
        assert_eq!(base.get(1, 1), '┼');
    }

    #[test]
    fn merge_overlay_space_does_not_overwrite() {
        let mut base = Canvas::new(3, 3, CharsetKind::Unicode);
        base.put_char(1, 1, 'A');
        let overlay = Canvas::new(3, 3, CharsetKind::Unicode);
        base.merge(&overlay, 0, 0);
        assert_eq!(base.get(1, 1), 'A');
    }

    #[test]
    fn merge_overlay_non_junction_replaces() {
        let mut base = Canvas::new(3, 3, CharsetKind::Unicode);
        base.put_char(1, 1, '─');
        let mut overlay = Canvas::new(3, 3, CharsetKind::Unicode);
        overlay.put_char(1, 1, 'A');
        base.merge(&overlay, 0, 0);
        assert_eq!(base.get(1, 1), 'A');
    }

    #[test]
    fn merge_other_canvas_offsets_correctly() {
        let mut base = Canvas::new(5, 5, CharsetKind::Unicode);
        let mut overlay = Canvas::new(2, 2, CharsetKind::Unicode);
        overlay.put_char(0, 0, 'X');
        base.merge(&overlay, 2, 3);
        assert_eq!(base.get(2, 3), 'X');
        assert_eq!(base.get(0, 0), ' ');
    }

    #[test]
    fn merge_grows_canvas_when_overlay_extends_past_edge() {
        let mut base = Canvas::new(2, 2, CharsetKind::Unicode);
        let mut overlay = Canvas::new(2, 2, CharsetKind::Unicode);
        overlay.put_char(1, 1, 'Z');
        base.merge(&overlay, 3, 3);
        assert!(base.width() >= 5);
        assert!(base.height() >= 5);
        assert_eq!(base.get(4, 4), 'Z');
    }

    #[test]
    fn is_junction_char_recognizes_all_box_chars() {
        for c in [
            '─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '╴', '╵', '╶', '╷',
        ] {
            assert!(is_junction_char(c), "expected {c} to be a junction");
        }
    }

    #[test]
    fn is_junction_char_rejects_letters() {
        assert!(!is_junction_char('A'));
        assert!(!is_junction_char(' '));
        assert!(!is_junction_char('+'));
    }

    #[test]
    fn charset_unicode_and_ascii_differ_in_corners() {
        let mut u = Canvas::new(3, 3, CharsetKind::Unicode);
        let mut a = Canvas::new(3, 3, CharsetKind::Ascii);
        u.draw_box(0, 0, 2, 2);
        a.draw_box(0, 0, 2, 2);
        assert_ne!(u.get(0, 0), a.get(0, 0));
        assert_eq!(u.get(0, 0), '┌');
        assert_eq!(a.get(0, 0), '+');
    }

    #[test]
    fn canvas_clone_is_independent() {
        let mut c1 = Canvas::new(3, 3, CharsetKind::Unicode);
        let c2 = c1.clone();
        c1.put_char(0, 0, 'X');
        assert_eq!(c2.get(0, 0), ' ');
        assert_eq!(c1.get(0, 0), 'X');
    }

    // ---- Color grid tests ----

    #[test]
    fn set_color_in_bounds_is_recorded() {
        let mut c = Canvas::new(4, 2, CharsetKind::Unicode);
        c.put_char(1, 0, 'A');
        c.set_color(1, 0, Color::Red);
        let rows = c.into_styled_lines();
        let row = &rows[0];
        let painted = row.iter().find(|r| r.color == Some(Color::Red));
        assert!(painted.is_some(), "expected a red run, got {row:?}");
        assert_eq!(painted.unwrap().text, "A");
    }

    #[test]
    fn set_color_out_of_bounds_is_noop() {
        let mut c = Canvas::new(3, 2, CharsetKind::Unicode);
        c.set_color(10, 0, Color::Red);
        c.set_color(0, 10, Color::Red);
        let rows = c.into_styled_lines();
        for row in &rows {
            for run in row {
                assert_eq!(run.color, None);
            }
        }
    }

    #[test]
    fn paint_hline_colors_only_the_range() {
        let mut c = Canvas::new(6, 1, CharsetKind::Unicode);
        for x in 0..6 {
            c.put_char(x, 0, 'x');
        }
        c.paint_hline(1, 3, 0, Color::Green);
        let rows = c.into_styled_lines();
        let row = &rows[0];
        let total_text: String = row.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(total_text, "xxxxxx");
        let green_text: String = row
            .iter()
            .filter(|r| r.color == Some(Color::Green))
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(green_text, "xxx");
    }

    #[test]
    fn paint_vline_colors_only_the_range() {
        let mut c = Canvas::new(2, 5, CharsetKind::Unicode);
        for y in 0..5 {
            c.put_char(0, y, 'y');
        }
        c.paint_vline(0, 1, 3, Color::Blue);
        let rows = c.into_styled_lines();
        assert_eq!(rows[0][0].color, None);
        assert_eq!(rows[1][0].color, Some(Color::Blue));
        assert_eq!(rows[2][0].color, Some(Color::Blue));
        assert_eq!(rows[3][0].color, Some(Color::Blue));
        assert_eq!(rows[4][0].color, None);
    }

    #[test]
    fn paint_box_paints_only_border_not_interior() {
        let mut c = Canvas::new(5, 5, CharsetKind::Unicode);
        c.draw_box(0, 0, 4, 4);
        c.paint_box(0, 0, 4, 4, Color::Yellow);
        let rows = c.as_styled_lines();
        // Interior cell (2,2) should be uncolored.
        // Walk row 2 and find char at column 2.
        let row2 = &rows[2];
        let mut cursor = 0usize;
        let mut found_color: Option<Color> = None;
        for run in row2 {
            let len = run.text.chars().count();
            if cursor <= 2 && 2 < cursor + len {
                found_color = run.color;
                break;
            }
            cursor += len;
        }
        assert_eq!(found_color, None, "interior cell should not be painted");
    }

    #[test]
    fn paint_text_advances_with_unicode_width() {
        let mut c = Canvas::new(10, 1, CharsetKind::Unicode);
        c.put_str(0, 0, "日a");
        c.paint_text(0, 0, "日a", Color::Magenta);
        // paint_text mirrors put_str's x-advance: it paints cell 0 (日 head)
        // and cell 2 (a). Cell 1 is the glyph's trailing half — kept uncolored.
        let rows = c.into_styled_lines();
        let row = &rows[0];
        assert_eq!(row.len(), 3, "expected 3 runs: {row:?}");
        assert_eq!(row[0].color, Some(Color::Magenta));
        assert_eq!(row[0].text, "日");
        assert_eq!(row[1].color, None);
        assert_eq!(row[1].text, " ");
        assert_eq!(row[2].color, Some(Color::Magenta));
        assert_eq!(row[2].text, "a");
    }

    #[test]
    fn into_styled_lines_groups_consecutive_runs() {
        let mut c = Canvas::new(6, 1, CharsetKind::Unicode);
        c.put_str(0, 0, "abcdef");
        c.set_color(0, 0, Color::Red);
        c.set_color(1, 0, Color::Red);
        c.set_color(4, 0, Color::Blue);
        c.set_color(5, 0, Color::Blue);
        let rows = c.into_styled_lines();
        let row = &rows[0];
        // Expect: [Red"ab"] [None"cd"] [Blue"ef"]
        assert_eq!(row.len(), 3);
        assert_eq!(row[0].color, Some(Color::Red));
        assert_eq!(row[0].text, "ab");
        assert_eq!(row[1].color, None);
        assert_eq!(row[1].text, "cd");
        assert_eq!(row[2].color, Some(Color::Blue));
        assert_eq!(row[2].text, "ef");
    }

    #[test]
    fn into_styled_lines_trims_uncolored_trailing_whitespace() {
        let mut c = Canvas::new(10, 1, CharsetKind::Unicode);
        c.put_char(0, 0, 'A');
        let rows = c.into_styled_lines();
        let row = &rows[0];
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].text, "A");
    }

    #[test]
    fn into_styled_lines_preserves_colored_trailing_whitespace() {
        let mut c = Canvas::new(5, 1, CharsetKind::Unicode);
        c.put_char(0, 0, 'A');
        c.set_color(3, 0, Color::Red);
        c.set_color(4, 0, Color::Red);
        let rows = c.into_styled_lines();
        let row = &rows[0];
        // Expect: [None "A  "] [Red "  "] (colored trailing spaces stay)
        let last = row.last().expect("at least one run");
        assert_eq!(last.color, Some(Color::Red));
        assert_eq!(last.text, "  ");
    }

    #[test]
    fn put_str_colored_combines_chars_and_color() {
        let mut c = Canvas::new(10, 1, CharsetKind::Unicode);
        c.put_str_colored(0, 0, "hi", Color::Cyan);
        assert_eq!(c.get(0, 0), 'h');
        assert_eq!(c.get(1, 0), 'i');
        let rows = c.into_styled_lines();
        let row = &rows[0];
        assert_eq!(row.len(), 1);
        assert_eq!(row[0].text, "hi");
        assert_eq!(row[0].color, Some(Color::Cyan));
    }

    #[test]
    fn ensure_size_extends_color_grid_in_lockstep() {
        let mut c = Canvas::new(2, 2, CharsetKind::Unicode);
        c.ensure_size(5, 4);
        // No panic when setting color at the new extents.
        c.set_color(4, 3, Color::Red);
        let rows = c.into_styled_lines();
        assert_eq!(rows.len(), 4);
        // Last row has only a colored trailing space — preserved.
        let last_row = &rows[3];
        assert!(last_row.iter().any(|r| r.color == Some(Color::Red)));
    }

    #[test]
    fn merge_propagates_colors_from_overlay() {
        let mut base = Canvas::new(4, 1, CharsetKind::Unicode);
        base.put_str(0, 0, "....");
        let mut overlay = Canvas::new(2, 1, CharsetKind::Unicode);
        overlay.put_str_colored(0, 0, "ab", Color::Red);
        base.merge(&overlay, 1, 0);
        let rows = base.into_styled_lines();
        let row = &rows[0];
        let red: String = row
            .iter()
            .filter(|r| r.color == Some(Color::Red))
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(red, "ab");
    }

    #[test]
    fn into_styled_lines_empty_row_yields_empty_runs() {
        let c = Canvas::new(5, 2, CharsetKind::Unicode);
        let rows = c.into_styled_lines();
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert!(row.is_empty(), "blank row should flatten to []: {row:?}");
        }
    }

    #[test]
    fn styled_lines_flatten_to_same_string_as_into_lines() {
        let mut c = Canvas::new(8, 3, CharsetKind::Unicode);
        c.draw_box(0, 0, 4, 2);
        c.put_str(1, 1, "ok");
        let plain = c.clone().into_lines();
        let styled = c.into_styled_lines();
        for (i, row) in styled.iter().enumerate() {
            let flat: String = row.iter().map(|r| r.text.as_str()).collect();
            assert_eq!(flat, plain[i], "row {i} mismatch");
        }
    }
}
