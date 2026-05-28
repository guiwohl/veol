use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color as RatColor, Modifier, Style};
use ratatui::widgets::Widget;

use crate::config::Config;
use crate::render::theme::Theme;

#[derive(Debug)]
pub struct ThemeSwitcherState {
    pub themes: Vec<String>,
    pub selected: usize,
    pub preview_cache: HashMap<String, [RatColor; 6]>,
    pub current_theme_name: String,
}

pub enum SwitcherOutcome {
    Persisted(Theme),
    Cancelled(Theme),
    StillOpen,
}

impl ThemeSwitcherState {
    pub fn new(current: &str) -> Self {
        let themes = Theme::list_all();
        let selected = themes.iter().position(|name| name == current).unwrap_or(0);
        Self {
            themes,
            selected,
            preview_cache: HashMap::new(),
            current_theme_name: current.to_string(),
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.themes.len() {
            self.selected += 1;
        }
    }

    pub fn selected_name(&self) -> &str {
        self.themes
            .get(self.selected)
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn preview_dots(&mut self, name: &str) -> [RatColor; 6] {
        if let Some(cached) = self.preview_cache.get(name) {
            return *cached;
        }
        let dots = match Theme::load(name) {
            Ok(theme) => {
                let raw = theme.preview_dots();
                [
                    raw[0].to_ratatui(),
                    raw[1].to_ratatui(),
                    raw[2].to_ratatui(),
                    raw[3].to_ratatui(),
                    raw[4].to_ratatui(),
                    raw[5].to_ratatui(),
                ]
            }
            Err(_) => [RatColor::Reset; 6],
        };
        self.preview_cache.insert(name.to_string(), dots);
        dots
    }

    pub fn populate_previews_for_visible(&mut self, visible: std::ops::Range<usize>) {
        for i in visible {
            if let Some(name) = self.themes.get(i).cloned() {
                let _ = self.preview_dots(&name);
            }
        }
    }

    pub fn confirm(&mut self, config: &mut Config) -> SwitcherOutcome {
        let name = self.selected_name().to_string();
        match Theme::load(&name) {
            Ok(theme) => match config.update_theme(&name) {
                Ok(()) => SwitcherOutcome::Persisted(theme),
                Err(_) => fallback_to_current(&self.current_theme_name),
            },
            Err(_) => fallback_to_current(&self.current_theme_name),
        }
    }

    pub fn cancel(&self) -> SwitcherOutcome {
        fallback_to_current(&self.current_theme_name)
    }
}

fn fallback_to_current(name: &str) -> SwitcherOutcome {
    match Theme::load(name) {
        Ok(theme) => SwitcherOutcome::Cancelled(theme),
        Err(_) => SwitcherOutcome::Cancelled(Theme::default()),
    }
}

pub struct ThemeSwitcherWidget<'a> {
    pub state: &'a ThemeSwitcherState,
    pub theme: &'a Theme,
}

impl<'a> ThemeSwitcherWidget<'a> {
    fn cached_dots(&self, name: &str) -> [RatColor; 6] {
        self.state
            .preview_cache
            .get(name)
            .copied()
            .unwrap_or([RatColor::Reset; 6])
    }
}

const BORDER_HORIZ: char = '─';
const BORDER_VERT: char = '│';
const BORDER_TL: char = '╭';
const BORDER_TR: char = '╮';
const BORDER_BL: char = '╰';
const BORDER_BR: char = '╯';
const DOT: char = '\u{25CF}';
const TITLE_ICON: char = '\u{e22b}';

impl<'a> Widget for ThemeSwitcherWidget<'a> {
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

        let title = format!(" {TITLE_ICON} Theme ");
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

        let total = self.state.themes.len();
        let scroll_offset = if visible_rows == 0 || total <= visible_rows {
            0
        } else if self.state.selected >= visible_rows {
            self.state.selected + 1 - visible_rows
        } else {
            0
        };

        for row in 0..visible_rows {
            let idx = scroll_offset + row;
            if idx >= total {
                break;
            }
            let y = list_top + row as u16;
            let name = &self.state.themes[idx];
            let is_selected = idx == self.state.selected;
            let row_bg = if is_selected { selected_bg } else { bg };

            for x in inner_left..inner_right {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(row_bg));
                }
            }

            let name_style = if is_selected {
                Style::default()
                    .fg(accent)
                    .bg(row_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(dim).bg(row_bg)
            };

            let name_x = inner_left + 1;
            let max_name_chars = inner_width.saturating_sub(10) as usize;
            let displayed: String = name.chars().take(max_name_chars).collect();
            buf.set_string(name_x, y, &displayed, name_style);

            let dots = self.cached_dots(name);
            let dots_total: u16 = 6 + 5;
            if inner_width >= dots_total + 2 {
                let dots_start = inner_right.saturating_sub(dots_total + 1);
                for (i, color) in dots.iter().enumerate() {
                    let dx = dots_start + (i as u16) * 2;
                    if dx >= inner_right {
                        break;
                    }
                    if let Some(cell) = buf.cell_mut((dx, y)) {
                        cell.set_char(DOT);
                        cell.set_style(Style::default().fg(*color).bg(row_bg));
                    }
                }
            }
        }

        let hint = " \u{2191}\u{2193} navigate  \u{23ce} apply  esc cancel ";
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_env_lock;

    fn with_config_home<R>(dir: &std::path::Path, f: impl FnOnce() -> R) -> R {
        let _guard = test_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", dir);
        let out = f();
        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        out
    }

    #[test]
    fn new_selects_current_theme_index() {
        let state = ThemeSwitcherState::new("dracula");
        assert!(state.themes.contains(&"dracula".to_string()));
        assert_eq!(state.themes[state.selected], "dracula");
        assert_eq!(state.current_theme_name, "dracula");
    }

    #[test]
    fn new_unknown_current_falls_back_to_zero() {
        let state = ThemeSwitcherState::new("not-a-real-theme-xyz");
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_up_at_zero_stays_at_zero() {
        let mut state = ThemeSwitcherState::new("not-a-real-theme-xyz");
        assert_eq!(state.selected, 0);
        state.move_up();
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_down_at_last_stays_at_last() {
        let mut state = ThemeSwitcherState::new("not-a-real-theme-xyz");
        let last = state.themes.len() - 1;
        state.selected = last;
        state.move_down();
        assert_eq!(state.selected, last);
    }

    #[test]
    fn selected_name_returns_correct_string() {
        let mut state = ThemeSwitcherState::new("dracula");
        state.selected = 0;
        let first = state.themes[0].clone();
        assert_eq!(state.selected_name(), first.as_str());
    }

    #[test]
    fn confirm_persists_via_config() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            let mut cfg = Config::default();
            let mut state = ThemeSwitcherState::new("reedo-dark");
            let target = "catppuccin";
            state.selected = state.themes.iter().position(|n| n == target).unwrap();
            let outcome = state.confirm(&mut cfg);
            assert!(matches!(outcome, SwitcherOutcome::Persisted(_)));
            assert_eq!(cfg.theme, target);

            let on_disk = Config::load();
            assert_eq!(on_disk.theme, target);
        });
    }

    #[test]
    fn cancel_returns_cancelled_with_prior() {
        let state = ThemeSwitcherState::new("dracula");
        let outcome = state.cancel();
        match outcome {
            SwitcherOutcome::Cancelled(theme) => assert_eq!(theme.name, "dracula"),
            _ => panic!("expected Cancelled outcome"),
        }
    }

    #[test]
    fn preview_dots_cached_after_first_call() {
        let mut state = ThemeSwitcherState::new("dracula");
        assert!(state.preview_cache.is_empty());
        let _ = state.preview_dots("dracula");
        assert!(state.preview_cache.contains_key("dracula"));
        let first = state.preview_cache.get("dracula").copied().unwrap();
        let _ = state.preview_dots("dracula");
        let second = state.preview_cache.get("dracula").copied().unwrap();
        assert_eq!(first, second);
        assert_eq!(state.preview_cache.len(), 1);
    }

    #[test]
    fn widget_renders_six_color_dots() {
        let mut state = ThemeSwitcherState::new("dracula");
        let theme = Theme::load("dracula").unwrap();
        state.populate_previews_for_visible(0..state.themes.len());

        let area = Rect::new(0, 0, 60, 14);
        let mut buf = Buffer::empty(area);
        let widget = ThemeSwitcherWidget {
            state: &state,
            theme: &theme,
        };
        widget.render(area, &mut buf);

        let selected_row = state.selected;
        let mut found_with_six = false;
        for y in area.y..area.y + area.height {
            let mut dots_in_row = 0;
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell((x, y)) {
                    if cell.symbol() == "\u{25CF}" {
                        dots_in_row += 1;
                    }
                }
            }
            if dots_in_row == 6 {
                found_with_six = true;
            }
        }
        assert!(
            found_with_six,
            "no row contained exactly six dots; selected={selected_row}"
        );
    }

    #[test]
    fn widget_highlights_selected_row() {
        let mut state = ThemeSwitcherState::new("dracula");
        let theme = Theme::load("dracula").unwrap();
        state.populate_previews_for_visible(0..state.themes.len());
        let selected_bg = theme.colors.popup_selected.to_ratatui();

        let area = Rect::new(0, 0, 60, 14);
        let mut buf = Buffer::empty(area);
        let widget = ThemeSwitcherWidget {
            state: &state,
            theme: &theme,
        };
        widget.render(area, &mut buf);

        let mut selected_row_count = 0;
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
                selected_row_count += 1;
            }
        }
        assert_eq!(
            selected_row_count, 1,
            "expected exactly one row tinted with popup_selected bg"
        );
    }
}
