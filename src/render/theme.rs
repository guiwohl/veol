use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

use crate::config::veol_config_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Rgb(u8, u8, u8),
    Ansi(ratatui::style::Color),
    Reset,
}

impl Color {
    pub fn to_ratatui(self) -> ratatui::style::Color {
        match self {
            Color::Rgb(r, g, b) => ratatui::style::Color::Rgb(r, g, b),
            Color::Ansi(c) => c,
            Color::Reset => ratatui::style::Color::Reset,
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("color value is empty"));
        }
        let lower = trimmed.to_ascii_lowercase();
        match lower.as_str() {
            "default" | "reset" => return Ok(Color::Reset),
            "black" => return Ok(Color::Ansi(ratatui::style::Color::Black)),
            "red" => return Ok(Color::Ansi(ratatui::style::Color::Red)),
            "green" => return Ok(Color::Ansi(ratatui::style::Color::Green)),
            "yellow" => return Ok(Color::Ansi(ratatui::style::Color::Yellow)),
            "blue" => return Ok(Color::Ansi(ratatui::style::Color::Blue)),
            "magenta" => return Ok(Color::Ansi(ratatui::style::Color::Magenta)),
            "cyan" => return Ok(Color::Ansi(ratatui::style::Color::Cyan)),
            "white" | "gray" | "grey" => return Ok(Color::Ansi(ratatui::style::Color::Gray)),
            "bright-black" | "bright_black" => {
                return Ok(Color::Ansi(ratatui::style::Color::DarkGray))
            }
            "bright-red" | "bright_red" => return Ok(Color::Ansi(ratatui::style::Color::LightRed)),
            "bright-green" | "bright_green" => {
                return Ok(Color::Ansi(ratatui::style::Color::LightGreen))
            }
            "bright-yellow" | "bright_yellow" => {
                return Ok(Color::Ansi(ratatui::style::Color::LightYellow))
            }
            "bright-blue" | "bright_blue" => {
                return Ok(Color::Ansi(ratatui::style::Color::LightBlue))
            }
            "bright-magenta" | "bright_magenta" => {
                return Ok(Color::Ansi(ratatui::style::Color::LightMagenta))
            }
            "bright-cyan" | "bright_cyan" => {
                return Ok(Color::Ansi(ratatui::style::Color::LightCyan))
            }
            "bright-white" | "bright_white" => {
                return Ok(Color::Ansi(ratatui::style::Color::White))
            }
            _ => {}
        }

        if let Some(hex) = trimmed.strip_prefix('#') {
            if hex.len() != 6 {
                return Err(anyhow!(
                    "invalid hex color '{trimmed}': expected '#rrggbb' (6 hex digits)"
                ));
            }
            let r = u8::from_str_radix(&hex[0..2], 16)
                .with_context(|| format!("invalid hex color '{trimmed}'"))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .with_context(|| format!("invalid hex color '{trimmed}'"))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .with_context(|| format!("invalid hex color '{trimmed}'"))?;
            return Ok(Color::Rgb(r, g, b));
        }

        Err(anyhow!(
            "unknown color '{trimmed}': use '#rrggbb', 'default', or an ANSI name"
        ))
    }

    pub fn to_token(self) -> String {
        match self {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            Color::Reset => "default".to_string(),
            Color::Ansi(c) => match c {
                ratatui::style::Color::Black => "black".into(),
                ratatui::style::Color::Red => "red".into(),
                ratatui::style::Color::Green => "green".into(),
                ratatui::style::Color::Yellow => "yellow".into(),
                ratatui::style::Color::Blue => "blue".into(),
                ratatui::style::Color::Magenta => "magenta".into(),
                ratatui::style::Color::Cyan => "cyan".into(),
                ratatui::style::Color::Gray => "white".into(),
                ratatui::style::Color::DarkGray => "bright-black".into(),
                ratatui::style::Color::LightRed => "bright-red".into(),
                ratatui::style::Color::LightGreen => "bright-green".into(),
                ratatui::style::Color::LightYellow => "bright-yellow".into(),
                ratatui::style::Color::LightBlue => "bright-blue".into(),
                ratatui::style::Color::LightMagenta => "bright-magenta".into(),
                ratatui::style::Color::LightCyan => "bright-cyan".into(),
                ratatui::style::Color::White => "bright-white".into(),
                _ => "default".into(),
            },
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Color::parse(&s).map_err(de::Error::custom)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_token())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeColors {
    #[serde(default = "defaults::bg")]
    pub bg: Color,
    #[serde(default = "defaults::fg")]
    pub fg: Color,
    #[serde(default = "defaults::gutter")]
    pub gutter: Color,
    #[serde(default = "defaults::cursor_bg")]
    pub cursor_bg: Color,
    #[serde(default = "defaults::cursor_fg")]
    pub cursor_fg: Color,
    #[serde(default = "defaults::selection")]
    pub selection: Color,
    #[serde(default = "defaults::statusbar_bg")]
    pub statusbar_bg: Color,
    #[serde(default = "defaults::statusbar_fg")]
    pub statusbar_fg: Color,
    #[serde(default = "defaults::keyword")]
    pub keyword: Color,
    #[serde(default = "defaults::string")]
    pub string: Color,
    #[serde(default = "defaults::comment")]
    pub comment: Color,
    #[serde(default = "defaults::function")]
    pub function: Color,
    #[serde(default = "defaults::ty", rename = "type")]
    pub r#type: Color,
    #[serde(default = "defaults::number")]
    pub number: Color,
    #[serde(default = "defaults::operator")]
    pub operator: Color,
    #[serde(default = "defaults::property")]
    pub property: Color,
    #[serde(default = "defaults::heading_1")]
    pub heading_1: Color,
    #[serde(default = "defaults::heading_2")]
    pub heading_2: Color,
    #[serde(default = "defaults::heading_3")]
    pub heading_3: Color,
    #[serde(default = "defaults::heading_4")]
    pub heading_4: Color,
    #[serde(default = "defaults::heading_5")]
    pub heading_5: Color,
    #[serde(default = "defaults::heading_6")]
    pub heading_6: Color,
    #[serde(default = "defaults::link")]
    pub link: Color,
    #[serde(default = "defaults::quote_marker")]
    pub quote_marker: Color,
    #[serde(default = "defaults::quote_text")]
    pub quote_text: Color,
    #[serde(default = "defaults::code_bg")]
    pub code_bg: Color,
    #[serde(default = "defaults::code_border")]
    pub code_border: Color,
    #[serde(default = "defaults::table_border")]
    pub table_border: Color,
    #[serde(default = "defaults::table_header_fg")]
    pub table_header_fg: Color,
    #[serde(default = "defaults::hr")]
    pub hr: Color,
    #[serde(default = "defaults::task_done")]
    pub task_done: Color,
    #[serde(default = "defaults::task_pending")]
    pub task_pending: Color,
    #[serde(default = "defaults::mermaid_caption")]
    pub mermaid_caption: Color,
    #[serde(default = "defaults::search_match")]
    pub search_match: Color,
    #[serde(default = "defaults::popup_bg")]
    pub popup_bg: Color,
    #[serde(default = "defaults::popup_border")]
    pub popup_border: Color,
    #[serde(default = "defaults::popup_selected")]
    pub popup_selected: Color,
    #[serde(default = "defaults::popup_dim")]
    pub popup_dim: Color,
    #[serde(default = "defaults::popup_accent")]
    pub popup_accent: Color,
}

mod defaults {
    use super::Color;
    pub fn bg() -> Color {
        Color::Rgb(0x1a, 0x1b, 0x26)
    }
    pub fn fg() -> Color {
        Color::Rgb(0xc0, 0xca, 0xf5)
    }
    pub fn gutter() -> Color {
        Color::Rgb(0x3b, 0x42, 0x61)
    }
    pub fn cursor_bg() -> Color {
        Color::Rgb(0xc0, 0xca, 0xf5)
    }
    pub fn cursor_fg() -> Color {
        Color::Rgb(0x1a, 0x1b, 0x26)
    }
    pub fn selection() -> Color {
        Color::Rgb(0x28, 0x34, 0x57)
    }
    pub fn statusbar_bg() -> Color {
        Color::Rgb(0x1e, 0x1e, 0x2e)
    }
    pub fn statusbar_fg() -> Color {
        Color::Rgb(0xa6, 0xad, 0xc8)
    }
    pub fn keyword() -> Color {
        Color::Rgb(0xbb, 0x9a, 0xf7)
    }
    pub fn string() -> Color {
        Color::Rgb(0x9e, 0xce, 0x6a)
    }
    pub fn comment() -> Color {
        Color::Rgb(0x56, 0x5f, 0x89)
    }
    pub fn function() -> Color {
        Color::Rgb(0x7a, 0xa2, 0xf7)
    }
    pub fn ty() -> Color {
        Color::Rgb(0x2a, 0xc3, 0xde)
    }
    pub fn number() -> Color {
        Color::Rgb(0xff, 0x9e, 0x64)
    }
    pub fn operator() -> Color {
        Color::Rgb(0x89, 0xdd, 0xff)
    }
    pub fn property() -> Color {
        Color::Rgb(0x73, 0xba, 0xc2)
    }
    pub fn heading_1() -> Color {
        Color::Rgb(0x7a, 0xa2, 0xf7)
    }
    pub fn heading_2() -> Color {
        Color::Rgb(0x7d, 0xcf, 0xff)
    }
    pub fn heading_3() -> Color {
        Color::Rgb(0xbb, 0x9a, 0xf7)
    }
    pub fn heading_4() -> Color {
        Color::Rgb(0x9e, 0xce, 0x6a)
    }
    pub fn heading_5() -> Color {
        Color::Rgb(0xe0, 0xaf, 0x68)
    }
    pub fn heading_6() -> Color {
        Color::Rgb(0xf7, 0x76, 0x8e)
    }
    pub fn link() -> Color {
        Color::Rgb(0x7a, 0xa2, 0xf7)
    }
    pub fn quote_marker() -> Color {
        Color::Rgb(0x56, 0x5f, 0x89)
    }
    pub fn quote_text() -> Color {
        Color::Rgb(0xa9, 0xb1, 0xd6)
    }
    pub fn code_bg() -> Color {
        Color::Rgb(0x1f, 0x23, 0x35)
    }
    pub fn code_border() -> Color {
        Color::Rgb(0x3b, 0x42, 0x61)
    }
    pub fn table_border() -> Color {
        Color::Rgb(0x3b, 0x42, 0x61)
    }
    pub fn table_header_fg() -> Color {
        Color::Rgb(0x7d, 0xcf, 0xff)
    }
    pub fn hr() -> Color {
        Color::Rgb(0x3b, 0x42, 0x61)
    }
    pub fn task_done() -> Color {
        Color::Rgb(0x9e, 0xce, 0x6a)
    }
    pub fn task_pending() -> Color {
        Color::Rgb(0xe0, 0xaf, 0x68)
    }
    pub fn mermaid_caption() -> Color {
        Color::Rgb(0xa9, 0xb1, 0xd6)
    }
    pub fn search_match() -> Color {
        Color::Rgb(0xe0, 0xaf, 0x68)
    }
    pub fn popup_bg() -> Color {
        Color::Rgb(0x1f, 0x23, 0x35)
    }
    pub fn popup_border() -> Color {
        Color::Rgb(0x3b, 0x42, 0x61)
    }
    pub fn popup_selected() -> Color {
        Color::Rgb(0x28, 0x34, 0x57)
    }
    pub fn popup_dim() -> Color {
        Color::Rgb(0x56, 0x5f, 0x89)
    }
    pub fn popup_accent() -> Color {
        Color::Rgb(0x7a, 0xa2, 0xf7)
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg: defaults::bg(),
            fg: defaults::fg(),
            gutter: defaults::gutter(),
            cursor_bg: defaults::cursor_bg(),
            cursor_fg: defaults::cursor_fg(),
            selection: defaults::selection(),
            statusbar_bg: defaults::statusbar_bg(),
            statusbar_fg: defaults::statusbar_fg(),
            keyword: defaults::keyword(),
            string: defaults::string(),
            comment: defaults::comment(),
            function: defaults::function(),
            r#type: defaults::ty(),
            number: defaults::number(),
            operator: defaults::operator(),
            property: defaults::property(),
            heading_1: defaults::heading_1(),
            heading_2: defaults::heading_2(),
            heading_3: defaults::heading_3(),
            heading_4: defaults::heading_4(),
            heading_5: defaults::heading_5(),
            heading_6: defaults::heading_6(),
            link: defaults::link(),
            quote_marker: defaults::quote_marker(),
            quote_text: defaults::quote_text(),
            code_bg: defaults::code_bg(),
            code_border: defaults::code_border(),
            table_border: defaults::table_border(),
            table_header_fg: defaults::table_header_fg(),
            hr: defaults::hr(),
            task_done: defaults::task_done(),
            task_pending: defaults::task_pending(),
            mermaid_caption: defaults::mermaid_caption(),
            search_match: defaults::search_match(),
            popup_bg: defaults::popup_bg(),
            popup_border: defaults::popup_border(),
            popup_selected: defaults::popup_selected(),
            popup_dim: defaults::popup_dim(),
            popup_accent: defaults::popup_accent(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Theme {
    #[serde(default = "default_name")]
    pub name: String,
    #[serde(default)]
    pub colors: ThemeColors,
}

fn default_name() -> String {
    "reedo-dark".to_string()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: default_name(),
            colors: ThemeColors::default(),
        }
    }
}

const BUNDLED: &[(&str, &str)] = &[
    ("default", include_str!("themes/default.toml")),
    ("reedo-dark", include_str!("themes/reedo-dark.toml")),
    ("reedo-light", include_str!("themes/reedo-light.toml")),
    ("catppuccin", include_str!("themes/catppuccin.toml")),
    ("dracula", include_str!("themes/dracula.toml")),
    ("gruvbox", include_str!("themes/gruvbox.toml")),
    ("nord", include_str!("themes/nord.toml")),
    ("rose-pine", include_str!("themes/rose-pine.toml")),
    ("solarized-dark", include_str!("themes/solarized-dark.toml")),
];

impl Theme {
    pub fn bundled_names() -> &'static [&'static str] {
        &[
            "default",
            "reedo-dark",
            "reedo-light",
            "catppuccin",
            "dracula",
            "gruvbox",
            "nord",
            "rose-pine",
            "solarized-dark",
        ]
    }

    pub fn parse(toml_str: &str) -> Result<Self> {
        toml::from_str::<Theme>(toml_str).map_err(|e| anyhow!("failed to parse theme: {e}"))
    }

    pub fn load(name: &str) -> Result<Self> {
        let custom = custom_themes_dir().join(format!("{name}.toml"));
        if custom.exists() {
            let content = std::fs::read_to_string(&custom)
                .with_context(|| format!("reading {}", custom.display()))?;
            return Self::parse(&content).with_context(|| format!("parsing {}", custom.display()));
        }
        for (n, body) in BUNDLED {
            if *n == name {
                return Self::parse(body)
                    .with_context(|| format!("parsing bundled theme '{name}'"));
            }
        }
        Err(anyhow!("theme '{name}' not found"))
    }

    pub fn list_custom() -> Vec<String> {
        let dir = custom_themes_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    out.push(stem.to_string());
                }
            }
        }
        out.sort();
        out
    }

    pub fn list_all() -> Vec<String> {
        let mut out: Vec<String> = Self::bundled_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        for name in Self::list_custom() {
            if !out.contains(&name) {
                out.push(name);
            }
        }
        out.sort();
        out.dedup();
        out
    }

    pub fn preview_dots(&self) -> [Color; 6] {
        [
            self.colors.heading_1,
            self.colors.code_border,
            self.colors.link,
            self.colors.quote_marker,
            self.colors.table_border,
            self.colors.popup_accent,
        ]
    }
}

fn custom_themes_dir() -> PathBuf {
    veol_config_dir().join("themes")
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
    fn parses_hex_color() {
        assert_eq!(
            Color::parse("#abcdef").unwrap(),
            Color::Rgb(0xab, 0xcd, 0xef)
        );
    }

    #[test]
    fn rejects_short_hex() {
        assert!(Color::parse("#abc").is_err());
    }

    #[test]
    fn rejects_garbage_hex() {
        assert!(Color::parse("#xyzxyz").is_err());
    }

    #[test]
    fn rejects_unknown_color_name() {
        let err = Color::parse("chartreuse").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("chartreuse"));
    }

    #[test]
    fn parses_default_keyword_as_reset() {
        assert_eq!(Color::parse("default").unwrap(), Color::Reset);
    }

    #[test]
    fn parses_ansi_names() {
        assert!(matches!(Color::parse("blue").unwrap(), Color::Ansi(_)));
        assert!(matches!(
            Color::parse("bright-black").unwrap(),
            Color::Ansi(_)
        ));
    }

    #[test]
    fn every_bundled_theme_parses() {
        for name in Theme::bundled_names() {
            let theme =
                Theme::load(name).unwrap_or_else(|e| panic!("bundled theme '{name}' failed: {e}"));
            assert_eq!(theme.name, *name);
        }
    }

    #[test]
    fn list_all_contains_every_bundled_name() {
        let all = Theme::list_all();
        for n in Theme::bundled_names() {
            assert!(all.contains(&n.to_string()), "missing {n}");
        }
    }

    #[test]
    fn preview_dots_are_not_all_equal() {
        let theme = Theme::default();
        let dots = theme.preview_dots();
        let first = dots[0];
        let any_diff = dots.iter().any(|c| *c != first);
        assert!(
            any_diff,
            "preview dots are all equal — preview is meaningless"
        );
    }

    #[test]
    fn round_trip_serialize_parse() {
        let theme = Theme::load("dracula").unwrap();
        let s = toml::to_string(&theme).unwrap();
        let parsed = Theme::parse(&s).unwrap();
        assert_eq!(theme, parsed);
    }

    #[test]
    fn custom_theme_loaded_from_xdg_config_home() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("veol").join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        let body = r##"name = "mytheme"
[colors]
heading_1 = "#112233"
"##;
        std::fs::write(themes_dir.join("mytheme.toml"), body).unwrap();

        with_config_home(tmp.path(), || {
            let theme = Theme::load("mytheme").expect("load custom");
            assert_eq!(theme.name, "mytheme");
            assert_eq!(theme.colors.heading_1, Color::Rgb(0x11, 0x22, 0x33));
            let listed = Theme::list_custom();
            assert!(listed.contains(&"mytheme".to_string()));
        });
    }

    #[test]
    fn default_theme_uses_reset_for_bg_and_fg() {
        let theme = Theme::load("default").unwrap();
        assert_eq!(theme.colors.bg, Color::Reset);
        assert_eq!(theme.colors.fg, Color::Reset);
    }

    #[test]
    fn unknown_theme_load_errors() {
        let tmp = tempfile::tempdir().unwrap();
        with_config_home(tmp.path(), || {
            assert!(Theme::load("does-not-exist").is_err());
        });
    }
}
