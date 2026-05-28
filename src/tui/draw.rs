use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::markdown::layout::LayoutLine;
use crate::render::theme::Theme;
use crate::tui::viewport::ViewportState;

pub struct DocumentWidget<'a> {
    pub lines: &'a [LayoutLine],
    pub viewport: &'a ViewportState,
    pub theme: &'a Theme,
}

impl<'a> Widget for DocumentWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bg = self.theme.colors.bg.to_ratatui();
        let fg = self.theme.colors.fg.to_ratatui();
        if !matches!(bg, Color::Reset) {
            buf.set_style(area, Style::default().bg(bg));
        }

        if self.lines.is_empty() {
            WelcomeWidget { theme: self.theme }.render(area, buf);
            return;
        }

        let top = self.viewport.top_line;
        let end = top
            .saturating_add(area.height as usize)
            .min(self.lines.len());
        for (row_idx, line_idx) in (top..end).enumerate() {
            let y = area.y + row_idx as u16;
            let line = &self.lines[line_idx];
            let right = area.x.saturating_add(area.width);
            let start_x = area.x.saturating_add(line.indent_cols).min(right);
            let mut x = start_x;
            for span in &line.spans {
                if x >= right {
                    break;
                }
                let remaining = (right - x) as usize;
                let mut style =
                    Style::default().fg(if span.fg == Color::Reset { fg } else { span.fg });
                if let Some(b) = span.bg {
                    style = style.bg(b);
                }
                let mut modifier = span.modifier;
                if span.link.is_some() {
                    modifier |= Modifier::UNDERLINED;
                }
                style = style.add_modifier(modifier);
                let (nx, _) = buf.set_stringn(x, y, &span.text, remaining, style);
                x = nx;
            }
        }
    }
}

pub struct WelcomeWidget<'a> {
    pub theme: &'a Theme,
}

impl<'a> Widget for WelcomeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let fg = self.theme.colors.fg.to_ratatui();
        let style = Style::default().fg(fg);
        let lines: [&str; 6] = [
            "Veol",
            "────",
            "Fast Markdown reading for the terminal.",
            "",
            "Pass a .md file, or pipe markdown from stdin.",
            "Ctrl+E browse · Ctrl+T theme · ? help · q quit",
        ];
        if area.height == 0 || area.width == 0 {
            return;
        }
        let total = lines.len() as u16;
        let start_y = area.y + area.height.saturating_sub(total) / 2;
        for (i, text) in lines.iter().enumerate() {
            let len = text.chars().count() as u16;
            let x = area.x + area.width.saturating_sub(len) / 2;
            let y = start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            buf.set_stringn(x, y, *text, area.width as usize, style);
        }
    }
}

pub struct StatusBarWidget<'a> {
    pub filename: &'a str,
    pub viewport: &'a ViewportState,
    pub flash: Option<&'a str>,
    pub theme: &'a Theme,
}

impl<'a> Widget for StatusBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let bg = self.theme.colors.statusbar_bg.to_ratatui();
        let fg = self.theme.colors.statusbar_fg.to_ratatui();
        let base = Style::default().fg(fg).bg(bg);
        buf.set_style(area, base);

        let total = self.viewport.total_lines;
        let percent = self.viewport.percent();
        let cur = if total == 0 {
            0
        } else {
            self.viewport.top_line + 1
        };
        let left = format!(
            "{name}  {pct}%  line {cur}/{total}",
            name = self.filename,
            pct = percent,
            cur = cur,
            total = total,
        );

        let y = area.y;
        buf.set_stringn(area.x, y, &left, area.width as usize, base);

        if let Some(flash) = self.flash {
            let flash_len = flash.chars().count() as u16;
            if flash_len <= area.width {
                let x = area.x + area.width - flash_len;
                buf.set_stringn(x, y, flash, flash_len as usize, base);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;

    use crate::markdown::layout::{LayoutLine, StyledSpan};
    use crate::markdown::parse;
    use crate::render::theme::{Color as ThemeColor, Theme};

    fn theme() -> Theme {
        Theme::default()
    }

    fn theme_reset_bg() -> Theme {
        let mut t = Theme::default();
        t.colors.bg = ThemeColor::Reset;
        t
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

    fn plain_line(text: &str) -> LayoutLine {
        LayoutLine {
            spans: vec![StyledSpan {
                text: text.to_string(),
                fg: ratatui::style::Color::Reset,
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
    fn document_renders_visible_window_only() {
        let lines: Vec<LayoutLine> = (0..100).map(|i| plain_line(&format!("line {i}"))).collect();
        let mut viewport = ViewportState::new(lines.len(), 5);
        viewport.top_line = 10;
        let theme = theme_reset_bg();
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        DocumentWidget {
            lines: &lines,
            viewport: &viewport,
            theme: &theme,
        }
        .render(area, &mut buf);
        insta::assert_snapshot!(
            "document_renders_visible_window_only",
            render_buffer_text(&buf)
        );
    }

    #[test]
    fn document_paints_bg_when_theme_bg_set() {
        let lines = vec![plain_line("hi")];
        let viewport = ViewportState::new(lines.len(), 1);
        let mut t = Theme::default();
        t.colors.bg = ThemeColor::Rgb(0x10, 0x20, 0x30);
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        DocumentWidget {
            lines: &lines,
            viewport: &viewport,
            theme: &t,
        }
        .render(area, &mut buf);
        let cell = buf.cell((4, 0)).unwrap();
        assert_eq!(cell.bg, ratatui::style::Color::Rgb(0x10, 0x20, 0x30));
    }

    #[test]
    fn document_skips_bg_when_theme_bg_reset() {
        let lines = vec![plain_line("hi")];
        let viewport = ViewportState::new(lines.len(), 1);
        let t = theme_reset_bg();
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        DocumentWidget {
            lines: &lines,
            viewport: &viewport,
            theme: &t,
        }
        .render(area, &mut buf);
        let cell = buf.cell((4, 0)).unwrap();
        assert_eq!(cell.bg, ratatui::style::Color::Reset);
    }

    #[test]
    fn document_truncates_at_right_edge() {
        let lines = vec![plain_line("0123456789ABCDEFGHIJ")];
        let viewport = ViewportState::new(lines.len(), 1);
        let t = theme_reset_bg();
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);
        DocumentWidget {
            lines: &lines,
            viewport: &viewport,
            theme: &t,
        }
        .render(area, &mut buf);
        let snap = render_buffer_text(&buf);
        assert_eq!(snap, "01234");
    }

    #[test]
    fn document_renders_styled_spans() {
        let blocks = parse("**bold** *italic* ~~strike~~ and a [link](https://x.test).\n");
        let lines = crate::markdown::layout::layout(&blocks, 60, &theme());
        let viewport = ViewportState::new(lines.len(), lines.len().max(1));
        let t = theme_reset_bg();
        let area = Rect::new(0, 0, 60, lines.len().max(1) as u16);
        let mut buf = Buffer::empty(area);
        DocumentWidget {
            lines: &lines,
            viewport: &viewport,
            theme: &t,
        }
        .render(area, &mut buf);

        let mut bold_found = false;
        let mut underline_found = false;
        let mut strike_found = false;
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    let m = cell.modifier;
                    if m.contains(Modifier::BOLD) {
                        bold_found = true;
                    }
                    if m.contains(Modifier::UNDERLINED) {
                        underline_found = true;
                    }
                    if m.contains(Modifier::CROSSED_OUT) {
                        strike_found = true;
                    }
                }
            }
        }
        assert!(bold_found, "bold modifier missing");
        assert!(underline_found, "underline (link) modifier missing");
        assert!(strike_found, "strikethrough modifier missing");
        insta::assert_snapshot!("document_renders_styled_spans", render_buffer_text(&buf));
    }

    #[test]
    fn statusbar_renders_filename_and_position() {
        let viewport = ViewportState::new(10, 10);
        let t = theme();
        let area = Rect::new(0, 0, 50, 1);
        let mut buf = Buffer::empty(area);
        StatusBarWidget {
            filename: "[stdin]",
            viewport: &viewport,
            flash: None,
            theme: &t,
        }
        .render(area, &mut buf);
        let rendered = render_buffer_text(&buf);
        assert!(rendered.contains("[stdin]"), "got: {rendered:?}");
    }

    #[test]
    fn statusbar_with_flash_right_aligned() {
        let viewport = ViewportState::new(10, 10);
        let t = theme();
        let area = Rect::new(0, 0, 50, 1);
        let mut buf = Buffer::empty(area);
        StatusBarWidget {
            filename: "doc.md",
            viewport: &viewport,
            flash: Some("reloaded"),
            theme: &t,
        }
        .render(area, &mut buf);
        let rendered = render_buffer_text(&buf);
        assert!(rendered.ends_with("reloaded"), "got: {rendered:?}");
        insta::assert_snapshot!("statusbar_with_flash_right_aligned", rendered);
    }

    #[test]
    fn welcome_centered_in_area() {
        let t = theme_reset_bg();
        let area = Rect::new(0, 0, 60, 12);
        let mut buf = Buffer::empty(area);
        WelcomeWidget { theme: &t }.render(area, &mut buf);
        insta::assert_snapshot!("welcome_centered_in_area", render_buffer_text(&buf));
    }

    #[test]
    fn document_renders_mermaid_block_inline_as_ascii() {
        let blocks = parse("```mermaid\ngraph LR; A-->B;\n```\n");
        let lines = crate::markdown::layout::layout(&blocks, 60, &theme());
        let viewport = ViewportState::new(lines.len(), lines.len().max(1));
        let t = theme_reset_bg();
        let area = Rect::new(0, 0, 60, lines.len().max(1) as u16);
        let mut buf = Buffer::empty(area);
        DocumentWidget {
            lines: &lines,
            viewport: &viewport,
            theme: &t,
        }
        .render(area, &mut buf);
        let rendered = render_buffer_text(&buf);
        assert!(
            !rendered.contains("Mermaid diagram"),
            "old placeholder leaked: {rendered:?}"
        );
    }
}
