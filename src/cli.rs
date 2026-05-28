use std::path::PathBuf;

use clap::Parser;

pub const WIDTH_MIN_COLS: u16 = 20;

#[derive(Debug, Parser)]
#[command(
    name = "veol",
    version,
    about = "Fast Markdown reading for the terminal.",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[arg(value_name = "FILE")]
    pub file: Option<String>,

    #[arg(long, conflicts_with_all = ["no_pager", "plain"])]
    pub pager: bool,

    #[arg(long, conflicts_with = "pager")]
    pub no_pager: bool,

    #[arg(long)]
    pub plain: bool,

    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,

    #[arg(long)]
    pub theme_list: bool,

    #[arg(long, value_name = "COLS", value_parser = parse_width)]
    pub width: Option<u16>,

    #[arg(long)]
    pub no_mermaid: bool,

    #[arg(long)]
    pub no_watch: bool,

    #[arg(long)]
    pub no_mouse: bool,

    #[arg(long)]
    pub toc: bool,

    #[arg(long)]
    pub line_numbers: bool,

    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub debug_render: bool,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

fn parse_width(s: &str) -> Result<u16, String> {
    let n: u16 = s.parse().map_err(|_| format!("`{s}` is not a u16"))?;
    if n < WIDTH_MIN_COLS {
        return Err(format!("must be >= {WIDTH_MIN_COLS} columns (got {n})"));
    }
    Ok(n)
}

pub const BUNDLED_THEMES: &[&str] = &[
    "Default",
    "reedo-dark",
    "reedo-light",
    "catppuccin",
    "dracula",
    "gruvbox",
    "nord",
    "rose-pine",
    "solarized-dark",
];

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_no_watch_flag() {
        let cli = Cli::try_parse_from(["veol", "--no-watch", "foo.md"]).unwrap();
        assert!(cli.no_watch);
        assert_eq!(cli.file.as_deref(), Some("foo.md"));
    }

    #[test]
    fn parses_no_mermaid_flag() {
        let cli = Cli::try_parse_from(["veol", "--no-mermaid", "foo.md"]).unwrap();
        assert!(cli.no_mermaid);
    }

    #[test]
    fn parses_no_mouse_flag() {
        let cli = Cli::try_parse_from(["veol", "--no-mouse", "foo.md"]).unwrap();
        assert!(cli.no_mouse);
    }

    #[test]
    fn no_mouse_defaults_to_false() {
        let cli = Cli::try_parse_from(["veol", "foo.md"]).unwrap();
        assert!(!cli.no_mouse);
    }

    #[test]
    fn rejects_unknown_flag() {
        let err = Cli::try_parse_from(["veol", "--nope"]).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn rejects_width_below_min() {
        let err = Cli::try_parse_from(["veol", "--width", "10"]).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn rejects_pager_and_plain() {
        let err = Cli::try_parse_from(["veol", "--pager", "--plain"]).unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
