use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ratatui::layout::Rect;

use crate::cli::Cli;
use crate::config::Config;
use crate::markdown::layout::{layout, LayoutLine};
use crate::markdown::model::Block;
use crate::markdown::parse;
use crate::mermaid::DisplayRegistry;
use crate::render::theme::Theme;
use crate::tui::browser::{BrowserState, HintResult, TreeAction};
use crate::tui::keymap::Action;
use crate::tui::search::{SearchMode, SearchState};
use crate::tui::theme_switcher::{SwitcherOutcome, ThemeSwitcherState};
use crate::tui::toc::TocState;
use crate::tui::viewport::ViewportState;
use crate::watcher::Watcher;

const FLASH_DURATION: Duration = Duration::from_millis(2500);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    #[default]
    None,
    Browser,
    Toc,
    ThemeSwitcher,
    Help,
}

pub struct App {
    pub source_path: Option<PathBuf>,
    pub source_text: String,
    pub blocks: Vec<Block>,
    pub lines: Vec<LayoutLine>,
    pub viewport: ViewportState,
    pub last_layout_width: u16,
    pub config: Config,
    pub theme: Theme,
    pub popup: Popup,
    pub browser: BrowserState,
    pub toc: TocState,
    pub search: SearchState,
    pub theme_switcher: Option<ThemeSwitcherState>,
    pub display: DisplayRegistry,
    pub watcher: Option<Watcher>,
    pub show_frontmatter: bool,
    pub flash: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        let mut config = Config::load();

        if let Some(name) = cli.theme.as_deref() {
            config.theme = name.to_string();
        }
        if cli.no_watch {
            config.watch = false;
        }
        if cli.no_mermaid {
            config.mermaid = false;
        }

        let (source_path, source_text) = read_source(cli)?;

        let theme = Theme::load(&config.theme).unwrap_or_else(|e| {
            tracing::warn!(
                "failed to load theme '{}': {e}; using default",
                config.theme
            );
            Theme::default()
        });

        let blocks = parse(&source_text);
        let initial_width: u16 = cli.width.unwrap_or(80);
        let lines = layout(&blocks, initial_width, &theme);

        let mut viewport = ViewportState::new(lines.len(), 0);
        viewport.set_total(lines.len());

        let browser = BrowserState::default();
        let toc = TocState::default();
        let search = SearchState::default();

        let mut display = DisplayRegistry::new();
        if config.mermaid {
            populate_registry(&mut display, &blocks, initial_width);
        }

        let mut app = Self {
            source_path,
            source_text,
            blocks,
            lines,
            viewport,
            last_layout_width: initial_width,
            config,
            theme,
            popup: Popup::None,
            browser,
            toc,
            search,
            theme_switcher: None,
            display,
            watcher: None,
            show_frontmatter: true,
            flash: None,
            should_quit: false,
        };

        app.show_frontmatter = app.config.frontmatter;
        app.setup_watcher();

        Ok(app)
    }

    pub fn relayout(&mut self, width: u16) {
        if width == self.last_layout_width {
            return;
        }
        let top_line = self.viewport.top_line;
        self.lines = layout(&self.blocks, width, &self.theme);
        self.last_layout_width = width;
        self.viewport.set_total(self.lines.len());
        self.viewport.top_line = top_line.min(self.lines.len().saturating_sub(1));
        if self.search.mode == SearchMode::Active && !self.search.query.is_empty() {
            self.search.confirm(&self.lines);
        }
    }

    pub fn reload(&mut self) -> Result<()> {
        let Some(path) = self.source_path.clone() else {
            return Ok(());
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let prev_anchor = self
            .lines
            .get(self.viewport.top_line)
            .and_then(|l| l.heading_anchor.clone());
        let prev_top = self.viewport.top_line;

        self.source_text = text;
        self.blocks = parse(&self.source_text);
        self.lines = layout(&self.blocks, self.last_layout_width, &self.theme);
        self.viewport.set_total(self.lines.len());

        let restored = prev_anchor
            .as_ref()
            .and_then(|a| {
                self.lines
                    .iter()
                    .position(|l| l.heading_anchor.as_deref() == Some(a.as_str()))
            })
            .unwrap_or(prev_top.min(self.lines.len().saturating_sub(1)));
        self.viewport.top_line = restored;
        self.viewport.set_total(self.lines.len());

        self.display.reset();
        if self.config.mermaid {
            populate_registry(&mut self.display, &self.blocks, self.last_layout_width);
        }

        if let Some(w) = self.watcher.as_mut() {
            w.rebaseline();
        }
        if self.search.mode == SearchMode::Active && !self.search.query.is_empty() {
            self.search.confirm(&self.lines);
        }
        self.flash_message("reloaded");
        Ok(())
    }

    pub fn handle_action(&mut self, action: Action, area: Rect) {
        let body_height = area.height.saturating_sub(1) as usize;
        self.viewport.set_height(body_height.max(1));
        match action {
            Action::Quit => self.should_quit = true,
            Action::ScrollDown(n) => self.viewport.scroll_down(n),
            Action::ScrollUp(n) => self.viewport.scroll_up(n),
            Action::PageDown => self.viewport.page_down(),
            Action::PageUp => self.viewport.page_up(),
            Action::HalfPageDown => self.viewport.half_page_down(),
            Action::HalfPageUp => self.viewport.half_page_up(),
            Action::JumpTop => self.viewport.jump_top(),
            Action::JumpBottom => self.viewport.jump_bottom(),
            Action::NextHeading => self.viewport.next_heading(&self.lines),
            Action::PrevHeading => self.viewport.prev_heading(&self.lines),
            Action::NextParagraph => self.viewport.next_paragraph(&self.lines),
            Action::PrevParagraph => self.viewport.prev_paragraph(&self.lines),
            Action::StartSearch => self.search.start(),
            Action::NextMatch => {
                if !self.search.matches.is_empty() {
                    self.search.next();
                    if let Some(m) = self.search.current_match() {
                        self.viewport.jump_to_line(m.line_index);
                    }
                }
            }
            Action::PrevMatch => {
                if !self.search.matches.is_empty() {
                    self.search.prev();
                    if let Some(m) = self.search.current_match() {
                        self.viewport.jump_to_line(m.line_index);
                    }
                }
            }
            Action::ToggleToc => {
                if self.popup == Popup::Toc {
                    self.popup = Popup::None;
                } else {
                    self.toc = TocState::build(&self.lines, &self.blocks);
                    self.popup = Popup::Toc;
                }
            }
            Action::Reload => {
                if let Err(e) = self.reload() {
                    self.flash_message(format!("reload failed: {e}"));
                }
            }
            Action::ToggleMermaid => {
                self.flash_message("mermaid rendering is always inline now");
            }
            Action::OpenLink => self.open_link_under_cursor(),
            Action::ToggleBrowser => {
                if self.popup == Popup::Browser {
                    self.popup = Popup::None;
                } else {
                    if self.browser.root.is_none() {
                        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        self.browser.build(&cwd);
                        self.browser.init_hint_scope();
                    }
                    self.popup = Popup::Browser;
                }
            }
            Action::ToggleThemeSwitcher => {
                if self.popup == Popup::ThemeSwitcher {
                    self.popup = Popup::None;
                } else {
                    let state = self
                        .theme_switcher
                        .take()
                        .unwrap_or_else(|| ThemeSwitcherState::new(&self.config.theme));
                    self.theme_switcher = Some(state);
                    self.popup = Popup::ThemeSwitcher;
                }
            }
            Action::ToggleFrontmatter => {
                self.show_frontmatter = !self.show_frontmatter;
                self.flash_message(if self.show_frontmatter {
                    "frontmatter: shown"
                } else {
                    "frontmatter: hidden"
                });
            }
            Action::ToggleHelp => {
                self.popup = if self.popup == Popup::Help {
                    Popup::None
                } else {
                    Popup::Help
                };
            }
            Action::Escape => self.handle_escape(),
            Action::None => {}
        }
    }

    pub fn handle_escape(&mut self) {
        if self.popup != Popup::None {
            self.popup = Popup::None;
            return;
        }
        if self.search.mode != SearchMode::Off {
            self.search.cancel();
        }
    }

    pub fn poll_watcher(&mut self) -> bool {
        let should_reload = match self.watcher.as_mut() {
            Some(w) => w.poll(),
            None => false,
        };
        if should_reload {
            if let Err(e) = self.reload() {
                self.flash_message(format!("reload failed: {e}"));
            }
            return true;
        }
        false
    }

    pub fn flash_message(&mut self, msg: impl Into<String>) {
        self.flash = Some((msg.into(), Instant::now()));
    }

    pub fn flash_active(&self) -> Option<&str> {
        let (msg, t) = self.flash.as_ref()?;
        if t.elapsed() < FLASH_DURATION {
            Some(msg.as_str())
        } else {
            None
        }
    }

    pub fn apply_switcher_outcome(&mut self, outcome: SwitcherOutcome) {
        match outcome {
            SwitcherOutcome::Persisted(theme) => {
                self.theme = theme;
                self.config.theme = self.theme.name.clone();
                self.lines = layout(&self.blocks, self.last_layout_width, &self.theme);
                self.viewport.set_total(self.lines.len());
                self.flash_message(format!("theme: {}", self.theme.name));
            }
            SwitcherOutcome::Cancelled(theme) => {
                self.theme = theme;
            }
            SwitcherOutcome::StillOpen => {}
        }
    }

    pub fn filename_for_status(&self) -> String {
        match &self.source_path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => "[stdin]".to_string(),
        }
    }

    fn open_link_under_cursor(&mut self) {
        let line = match self.lines.get(self.viewport.top_line) {
            Some(l) => l,
            None => {
                self.flash_message("no link here");
                return;
            }
        };
        let link = line.spans.iter().find_map(|s| s.link.clone());
        match link {
            Some(url) => {
                if let Err(e) = spawn_browser(&url) {
                    self.flash_message(format!("open link failed: {e}"));
                } else {
                    self.flash_message(format!("opened: {url}"));
                }
            }
            None => self.flash_message("no link here"),
        }
    }

    fn setup_watcher(&mut self) {
        if let Some(path) = self.source_path.clone() {
            if self.config.watch {
                self.watcher = Some(Watcher::new(path));
            }
        }
    }
}

fn populate_registry(display: &mut DisplayRegistry, blocks: &[Block], width: u16) {
    for (idx, block) in blocks.iter().enumerate() {
        if let Block::MermaidBlock { source } = block {
            display.ensure_rendered(idx, source, width);
        }
    }
}

pub fn handle_browser_open(app: &mut App, path: PathBuf) -> Result<()> {
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    app.source_text = text;
    app.source_path = Some(path.clone());
    app.blocks = parse(&app.source_text);
    app.lines = layout(&app.blocks, app.last_layout_width, &app.theme);
    app.viewport = ViewportState::new(app.lines.len(), app.viewport.height);
    app.display.reset();
    if app.config.mermaid {
        populate_registry(&mut app.display, &app.blocks, app.last_layout_width);
    }
    if app.config.watch {
        app.watcher = Some(Watcher::new(path));
    }
    app.popup = Popup::None;
    Ok(())
}

pub fn confirm_browser_action(app: &mut App, action: TreeAction) -> Option<PathBuf> {
    let result = match action {
        TreeAction::NewFile => app.browser.confirm_new_file(),
        TreeAction::NewFolder => app.browser.confirm_new_folder(),
        TreeAction::Rename => app.browser.confirm_rename(),
        TreeAction::Delete => {
            let _ = app.browser.confirm_delete();
            return None;
        }
        TreeAction::None => return None,
    };
    match result {
        Ok(p) => p,
        Err(e) => {
            app.flash_message(e);
            None
        }
    }
}

pub fn open_selected_browser_entry(app: &mut App) -> Result<bool> {
    let Some(entry_path) = app.browser.selected_path().cloned() else {
        return Ok(false);
    };
    let is_dir = app
        .browser
        .entries
        .get(app.browser.selected)
        .map(|e| e.is_dir)
        .unwrap_or(false);
    if is_dir {
        app.browser.toggle_dir();
        return Ok(false);
    }
    handle_browser_open(app, entry_path)?;
    Ok(true)
}

pub fn handle_hint_result(app: &mut App, result: HintResult) -> Result<bool> {
    match result {
        HintResult::EnteredFolder => Ok(false),
        HintResult::OpenFile(path) => {
            handle_browser_open(app, path)?;
            Ok(true)
        }
    }
}

fn read_source(cli: &Cli) -> Result<(Option<PathBuf>, String)> {
    use std::io::IsTerminal;

    match cli.file.as_deref() {
        Some("-") => Ok((None, read_stdin()?)),
        Some(path_str) => {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                anyhow::bail!(crate::file_not_found_message(path_str));
            }
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            Ok((Some(path), text))
        }
        None => {
            if std::io::stdin().is_terminal() {
                Ok((None, String::new()))
            } else {
                Ok((None, read_stdin()?))
            }
        }
    }
}

fn read_stdin() -> Result<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("reading stdin")?;
    Ok(buf)
}

fn spawn_browser(url: &str) -> std::io::Result<()> {
    if let Ok(browser) = std::env::var("BROWSER") {
        return std::process::Command::new(browser)
            .arg(url)
            .spawn()
            .map(|_| ());
    }
    if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()?;
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()?;
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;

    fn parse_cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("valid cli")
    }

    #[test]
    fn app_from_cli_loads_file_and_parses() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hello\n\nWorld\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let app = App::from_cli(&cli).expect("from_cli ok");
        assert_eq!(app.source_path.as_deref(), Some(path.as_path()));
        assert!(!app.blocks.is_empty());
        assert!(!app.lines.is_empty());
        assert!(app.watcher.is_none());
    }

    #[test]
    fn app_from_cli_missing_file_errors() {
        let cli = parse_cli(&["veol", "/no/such/path/here.md"]);
        match App::from_cli(&cli) {
            Ok(_) => panic!("expected error for missing file"),
            Err(e) => assert!(e.to_string().contains("file not found"), "got: {e}"),
        }
    }

    #[test]
    fn relayout_only_runs_when_width_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hello\n\nWorld wide rich text here.\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        let before = app.lines.clone();
        app.relayout(80);
        assert_eq!(app.lines, before, "same width must be a no-op");
        app.relayout(40);
        assert_ne!(app.last_layout_width, 80);
        assert_eq!(app.last_layout_width, 40);
    }

    #[test]
    fn reload_preserves_viewport_position_for_unchanged_doc() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        let body = "# Alpha\n\n## Beta\n\nbody beta\n\n## Gamma\n\nbody gamma\n";
        std::fs::write(&path, body).unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        let gamma_idx = app
            .lines
            .iter()
            .position(|l| l.heading_anchor.as_deref() == Some("gamma"))
            .expect("gamma anchor");
        app.viewport.top_line = gamma_idx;
        app.reload().expect("reload");
        let new_gamma_idx = app
            .lines
            .iter()
            .position(|l| l.heading_anchor.as_deref() == Some("gamma"))
            .expect("gamma anchor after reload");
        assert_eq!(app.viewport.top_line, new_gamma_idx);
    }

    #[test]
    fn handle_action_quit_sets_should_quit() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hi\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        let area = Rect::new(0, 0, 80, 24);
        app.handle_action(Action::Quit, area);
        assert!(app.should_quit);
    }

    #[test]
    fn handle_action_escape_closes_popup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hi\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        app.popup = Popup::Help;
        app.handle_action(Action::Escape, Rect::new(0, 0, 80, 24));
        assert_eq!(app.popup, Popup::None);
    }

    #[test]
    fn handle_action_toggle_browser_lazily_builds_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hi\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        assert!(app.browser.entries.is_empty(), "browser not yet built");
        app.handle_action(Action::ToggleBrowser, Rect::new(0, 0, 80, 24));
        assert_eq!(app.popup, Popup::Browser);
        assert!(
            !app.browser.entries.is_empty(),
            "browser built lazily on open"
        );
    }

    #[test]
    fn handle_action_toggle_theme_switcher_inits_state_first_call() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hi\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        assert!(app.theme_switcher.is_none());
        app.handle_action(Action::ToggleThemeSwitcher, Rect::new(0, 0, 80, 24));
        assert_eq!(app.popup, Popup::ThemeSwitcher);
        assert!(app.theme_switcher.is_some(), "lazy-inited on first toggle");
    }

    #[test]
    fn handle_action_escape_cancels_search_when_no_popup() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        std::fs::write(&path, "# Hi\n").unwrap();
        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let mut app = App::from_cli(&cli).expect("from_cli ok");
        app.search.start();
        assert_eq!(app.search.mode, SearchMode::Editing);
        app.handle_action(Action::Escape, Rect::new(0, 0, 80, 24));
        assert_eq!(app.search.mode, SearchMode::Off);
    }

    #[test]
    fn from_cli_populates_display_registry_for_mermaid_blocks() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("doc.md");
        let body = "# Title\n\n```mermaid\ngraph TD; A-->B;\n```\n\nprose\n\n```mermaid\nsequenceDiagram\nA->>B: hi\n```\n";
        std::fs::write(&path, body).unwrap();

        let cli = parse_cli(&["veol", "--no-watch", path.to_str().unwrap()]);
        let app = App::from_cli(&cli).expect("from_cli ok");

        let mermaid_indices: Vec<usize> = app
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(i, b)| matches!(b, Block::MermaidBlock { .. }).then_some(i))
            .collect();
        assert_eq!(mermaid_indices.len(), 2);
        for idx in mermaid_indices {
            let rows = app.display.get(idx).expect("rendered rows");
            assert!(!rows.is_empty());
        }
    }
}
