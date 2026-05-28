use std::sync::LazyLock;

use ratatui::style::{Color as RColor, Modifier};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SynStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::markdown::layout::StyledSpan;
use crate::render::theme::Theme;

const SYNTECT_THEME: &str = "base16-ocean.dark";

pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for CodeHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    fn resolve_syntax(&self, lang: Option<&str>) -> &syntect::parsing::SyntaxReference {
        if let Some(l) = lang {
            let l = l.trim();
            if !l.is_empty() {
                if let Some(s) = self.syntax_set.find_syntax_by_token(l) {
                    return s;
                }
                if let Some(s) = self.syntax_set.find_syntax_by_extension(l) {
                    return s;
                }
                if let Some(s) = self.syntax_set.find_syntax_by_name(l) {
                    return s;
                }
            }
        }
        self.syntax_set.find_syntax_plain_text()
    }

    fn syn_theme(&self) -> &syntect::highlighting::Theme {
        &self.theme_set.themes[SYNTECT_THEME]
    }

    pub fn highlight_line(
        &self,
        line: &str,
        lang: Option<&str>,
        veol_theme: &Theme,
    ) -> Vec<StyledSpan> {
        if line.is_empty() {
            return Vec::new();
        }
        let syntax = self.resolve_syntax(lang);
        let mut h = HighlightLines::new(syntax, self.syn_theme());
        let with_nl: String = if line.ends_with('\n') {
            line.to_string()
        } else {
            format!("{line}\n")
        };
        let ranges = h
            .highlight_line(&with_nl, &self.syntax_set)
            .unwrap_or_default();
        let mut spans = convert_ranges(&ranges, veol_theme);
        if let Some(last) = spans.last_mut() {
            if last.text.ends_with('\n') {
                last.text.pop();
                if last.text.is_empty() {
                    spans.pop();
                }
            }
        }
        spans
    }

    pub fn highlight_lines(
        &self,
        code: &str,
        lang: Option<&str>,
        veol_theme: &Theme,
    ) -> Vec<Vec<StyledSpan>> {
        let syntax = self.resolve_syntax(lang);
        let mut h = HighlightLines::new(syntax, self.syn_theme());
        let mut out: Vec<Vec<StyledSpan>> = Vec::new();
        for raw in code.split('\n') {
            if raw.is_empty() {
                out.push(Vec::new());
                continue;
            }
            let with_nl = format!("{raw}\n");
            let ranges = h
                .highlight_line(&with_nl, &self.syntax_set)
                .unwrap_or_default();
            let mut spans = convert_ranges(&ranges, veol_theme);
            if let Some(last) = spans.last_mut() {
                if last.text.ends_with('\n') {
                    last.text.pop();
                    if last.text.is_empty() {
                        spans.pop();
                    }
                }
            }
            out.push(spans);
        }
        out
    }
}

fn convert_ranges(ranges: &[(SynStyle, &str)], veol_theme: &Theme) -> Vec<StyledSpan> {
    let bg = veol_theme.colors.code_bg.to_ratatui();
    ranges
        .iter()
        .filter(|(_, t)| !t.is_empty())
        .map(|(style, text)| StyledSpan {
            text: (*text).to_string(),
            fg: RColor::Rgb(style.foreground.r, style.foreground.g, style.foreground.b),
            bg: Some(bg),
            modifier: font_style_to_modifier(style.font_style),
            link: None,
        })
        .collect()
}

fn font_style_to_modifier(fs: FontStyle) -> Modifier {
    let mut m = Modifier::empty();
    if fs.contains(FontStyle::BOLD) {
        m |= Modifier::BOLD;
    }
    if fs.contains(FontStyle::ITALIC) {
        m |= Modifier::ITALIC;
    }
    if fs.contains(FontStyle::UNDERLINE) {
        m |= Modifier::UNDERLINED;
    }
    m
}

pub static HIGHLIGHTER: LazyLock<CodeHighlighter> = LazyLock::new(CodeHighlighter::new);

#[cfg(test)]
mod tests {
    use super::*;

    fn th() -> Theme {
        Theme::default()
    }

    #[test]
    fn new_loads_default_themes_and_syntaxes() {
        let h = CodeHighlighter::new();
        assert!(!h.syntax_set.syntaxes().is_empty());
        assert!(h.theme_set.themes.contains_key("base16-ocean.dark"));
    }

    #[test]
    fn highlight_line_with_unknown_lang_returns_plain() {
        let h = CodeHighlighter::new();
        let spans = h.highlight_line("hello world", Some("unknown_xyz"), &th());
        assert!(!spans.is_empty());
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "hello world");
    }

    #[test]
    fn highlight_line_rust_keyword_gets_styled() {
        let h = CodeHighlighter::new();
        let theme = th();
        let spans = h.highlight_line("fn foo() {}", Some("rust"), &theme);
        assert!(!spans.is_empty());
        let fg = theme.colors.fg.to_ratatui();
        let any_non_fg = spans.iter().any(|s| s.fg != fg);
        assert!(any_non_fg, "expected at least one styled span");
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "fn foo() {}");
    }

    #[test]
    fn highlight_line_python_string_gets_styled() {
        let h = CodeHighlighter::new();
        let theme = th();
        let spans = h.highlight_line("x = 'hello'", Some("python"), &theme);
        let fg = theme.colors.fg.to_ratatui();
        let any_non_fg = spans.iter().any(|s| s.fg != fg);
        assert!(any_non_fg, "expected python tokens to be styled");
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "x = 'hello'");
    }

    #[test]
    fn highlight_lines_carries_state() {
        let h = CodeHighlighter::new();
        let theme = th();
        let lines = h.highlight_lines("/* multi\nline */", Some("c"), &theme);
        assert!(lines.len() >= 2);
        let fg = theme.colors.fg.to_ratatui();
        let second = &lines[1];
        assert!(!second.is_empty());
        let any_non_fg = second.iter().any(|s| s.fg != fg);
        assert!(
            any_non_fg,
            "second line should still be styled as comment continuation"
        );
    }

    #[test]
    fn highlight_line_empty_returns_empty_or_single_blank() {
        let h = CodeHighlighter::new();
        let spans = h.highlight_line("", Some("rust"), &th());
        let total_text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert!(total_text.is_empty());
    }

    #[test]
    fn highlight_line_uses_code_bg_from_theme() {
        let h = CodeHighlighter::new();
        let theme = th();
        let spans = h.highlight_line("fn foo() {}", Some("rust"), &theme);
        let bg = theme.colors.code_bg.to_ratatui();
        assert!(!spans.is_empty());
        for s in &spans {
            assert_eq!(s.bg, Some(bg), "every span must wear the theme code_bg");
        }
    }

    #[test]
    fn lookup_by_extension_falls_back_then_plain() {
        let h = CodeHighlighter::new();
        let plain = h.syntax_set.find_syntax_plain_text();
        let resolved = h.resolve_syntax(Some("unknown_xyz"));
        assert_eq!(resolved.name, plain.name);
    }
}
