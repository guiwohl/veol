pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod input;
pub mod markdown;
pub mod mermaid;
pub mod render;
pub mod tui;
pub mod watcher;

pub fn welcome_lines() -> Vec<String> {
    vec![
        String::from("Veol — Fast Markdown reading for the terminal."),
        String::new(),
        String::from("Open a file:  veol README.md"),
        String::from("Pipe stdin:   cat NOTES.md | veol -"),
        String::from("Help:         veol --help"),
    ]
}

pub fn usage_hint() -> &'static str {
    "veol: no file given. Pass a path or pipe markdown to stdin. See `veol --help`."
}

pub fn file_not_found_message(path: &str) -> String {
    format!("veol: file not found: {path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welcome_lines_not_empty() {
        assert!(!welcome_lines().is_empty());
    }

    #[test]
    fn file_not_found_format_matches_spec() {
        assert_eq!(
            file_not_found_message("README.md"),
            "veol: file not found: README.md"
        );
    }
}
