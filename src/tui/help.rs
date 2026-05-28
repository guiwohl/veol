use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::render::theme::Theme;

#[derive(Debug, Default)]
pub struct HelpState;

pub struct HelpWidget<'a> {
    pub state: &'a HelpState,
    pub theme: &'a Theme,
}

const BORDER_HORIZ: char = '─';
const BORDER_VERT: char = '│';
const BORDER_TL: char = '╭';
const BORDER_TR: char = '╮';
const BORDER_BL: char = '╰';
const BORDER_BR: char = '╯';
const TITLE_ICON: char = '\u{f128}';

pub fn keybinds() -> &'static [(&'static str, &'static str)] {
    &[
        ("q", "quit"),
        ("j / k / arrows", "line scroll"),
        ("Space / PgDn", "page down"),
        ("b / PgUp", "page up"),
        ("d / u", "half-page down/up"),
        ("g / G", "top / bottom"),
        ("] / [", "next / prev heading"),
        ("/ n N", "search / next / prev"),
        ("t", "table of contents"),
        ("Ctrl+E", "file browser"),
        ("Ctrl+T", "theme switcher"),
        ("m", "toggle mermaid render / source"),
        ("f", "toggle frontmatter"),
        ("r", "reload"),
        ("gx", "open link externally"),
        ("?", "toggle this help"),
        ("Esc", "close popup"),
    ]
}

impl<'a> Widget for HelpWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let _ = self.state;
        if area.width < 6 || area.height < 5 {
            return;
        }

        let bg = self.theme.colors.popup_bg.to_ratatui();
        let border = self.theme.colors.popup_border.to_ratatui();
        let accent = self.theme.colors.popup_accent.to_ratatui();
        let dim = self.theme.colors.popup_dim.to_ratatui();
        let fg = self.theme.colors.fg.to_ratatui();

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

        let title = format!(" {TITLE_ICON} Help ");
        let title_style = Style::default()
            .fg(accent)
            .bg(bg)
            .add_modifier(Modifier::BOLD);
        buf.set_string(area.x + 2, area.y, &title, title_style);

        let inner_left = area.x + 1;
        let inner_right = right;
        let inner_width = inner_right.saturating_sub(inner_left) as usize;
        let list_top = area.y + 1;
        let hint_y = bottom.saturating_sub(1);
        let list_bottom = if hint_y > list_top { hint_y } else { list_top };
        let visible_rows = list_bottom.saturating_sub(list_top) as usize;

        let entries = keybinds();
        let max_key_len = entries
            .iter()
            .map(|(k, _)| k.chars().count())
            .max()
            .unwrap_or(0);
        let key_col_width = max_key_len + 2;

        let key_style = Style::default()
            .fg(accent)
            .bg(bg)
            .add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(fg).bg(bg);

        for (row, (key, desc)) in entries.iter().take(visible_rows).enumerate() {
            let y = list_top + row as u16;
            let key_start = inner_left + 2;
            buf.set_string(key_start, y, *key, key_style);

            let desc_start = key_start + key_col_width as u16;
            if (desc_start as usize) < (inner_right as usize) {
                let max_desc = (inner_right as usize).saturating_sub(desc_start as usize);
                let truncated: String = desc.chars().take(max_desc).collect();
                buf.set_string(desc_start, y, &truncated, desc_style);
            }

            let _ = inner_width;
        }

        let hint = " esc / ? close ";
        let hint_style = Style::default().fg(dim).bg(bg);
        let hint_len = hint.chars().count();
        let hint_x = if inner_width > hint_len {
            inner_left + ((inner_width - hint_len) / 2) as u16
        } else {
            inner_left
        };
        buf.set_string(hint_x, hint_y, hint, hint_style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn widget_renders_keybind_list() {
        let state = HelpState;
        let theme = Theme::default();
        let area = Rect::new(0, 0, 50, 22);
        let mut buf = Buffer::empty(area);
        HelpWidget {
            state: &state,
            theme: &theme,
        }
        .render(area, &mut buf);
        let text = render_buffer_text(&buf);
        assert!(text.contains("quit"), "expected 'quit' description: {text}");
        assert!(text.contains("search"), "expected 'search' line: {text}");
        assert!(
            text.contains("table of contents"),
            "expected toc line: {text}"
        );
        assert!(
            text.contains("file browser"),
            "expected browser line: {text}"
        );
    }

    #[test]
    fn widget_renders_with_border() {
        let state = HelpState;
        let theme = Theme::default();
        let area = Rect::new(0, 0, 50, 22);
        let mut buf = Buffer::empty(area);
        HelpWidget {
            state: &state,
            theme: &theme,
        }
        .render(area, &mut buf);
        insta::assert_snapshot!("help_widget_with_border", render_buffer_text(&buf));
    }
}
