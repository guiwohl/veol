use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use directories::ProjectDirs;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui::{Frame, Terminal};
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;

use veol::app::{
    confirm_browser_action, handle_browser_open, handle_hint_result, open_selected_browser_entry,
    App, Popup,
};
use veol::cli::{Cli, BUNDLED_THEMES};
use veol::input::{map_mouse, MouseAction, PopupTarget};
use veol::markdown::layout::LayoutLine;
use veol::tui::browser::{BrowserWidget, HintResult, TreeAction};
use veol::tui::draw::{DocumentWidget, StatusBarWidget};
use veol::tui::keymap::map_reader_key;
use veol::tui::search::{highlight_line_with_matches, SearchBarWidget, SearchMode, SearchState};
use veol::tui::theme_switcher::ThemeSwitcherWidget;
use veol::tui::toc::TocWidget;
use veol::{file_not_found_message, usage_hint, welcome_lines};

fn main() -> ExitCode {
    let cli = Cli::parse_args();
    let _log_guard = init_logging();

    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "veol: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    if cli.theme_list {
        return print_theme_list();
    }

    let stdin_is_tty = io::stdin().is_terminal();
    let stdout_is_tty = io::stdout().is_terminal();

    if cli.file.is_none() && stdin_is_tty {
        let mut out = io::stdout().lock();
        writeln!(out, "{}", usage_hint())?;
        return Ok(ExitCode::from(0));
    }
    if let Some(path_str) = cli.file.as_deref() {
        if path_str != "-" && !std::path::Path::new(path_str).exists() {
            let mut stderr = io::stderr().lock();
            writeln!(stderr, "{}", file_not_found_message(path_str))?;
            return Ok(ExitCode::from(1));
        }
    }

    let mut app = App::from_cli(&cli)?;

    let use_tui = should_use_tui(&cli, stdin_is_tty, stdout_is_tty);
    if !use_tui {
        return print_plain_to_stdout(&mut app);
    }

    run_tui(&mut app, !cli.no_mouse)?;
    Ok(ExitCode::from(0))
}

fn should_use_tui(cli: &Cli, stdin_is_tty: bool, stdout_is_tty: bool) -> bool {
    if cli.plain || cli.no_pager {
        return false;
    }
    if cli.pager {
        return true;
    }
    stdout_is_tty && stdin_is_tty
}

fn print_plain_to_stdout(app: &mut App) -> Result<ExitCode> {
    let mut out = io::stdout().lock();
    if app.lines.is_empty() {
        for line in welcome_lines() {
            writeln!(out, "{line}")?;
        }
        return Ok(ExitCode::from(0));
    }
    for line in &app.lines {
        write_styled_line(&mut out, line)?;
        writeln!(out)?;
    }
    Ok(ExitCode::from(0))
}

fn write_styled_line(out: &mut impl Write, line: &LayoutLine) -> io::Result<()> {
    if line.indent_cols > 0 {
        for _ in 0..line.indent_cols {
            out.write_all(b" ")?;
        }
    }
    for span in &line.spans {
        out.write_all(span.text.as_bytes())?;
    }
    Ok(())
}

fn run_tui(app: &mut App, mouse: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if mouse {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        if mouse {
            let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        } else {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
        }
        original(info);
    }));

    let result = event_loop(&mut terminal, app);

    disable_raw_mode()?;
    if mouse {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    } else {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    }
    terminal.show_cursor()?;
    result
}

fn event_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        if app.should_quit {
            break;
        }
        app.poll_watcher();

        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        app.relayout(area.width);

        terminal.draw(|frame| draw_frame(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => handle_key(k, app, area),
                Event::Mouse(m) => handle_mouse(m, app, area),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    Ok(())
}

fn doc_and_status_areas(area: Rect, search_active: bool) -> (Rect, Option<Rect>, Rect) {
    let mut remaining = area;
    let status_rect = Rect::new(
        remaining.x,
        remaining.y + remaining.height.saturating_sub(1),
        remaining.width,
        1.min(remaining.height),
    );
    remaining.height = remaining.height.saturating_sub(1);

    let search_rect = if search_active && remaining.height > 0 {
        let r = Rect::new(
            remaining.x,
            remaining.y + remaining.height.saturating_sub(1),
            remaining.width,
            1,
        );
        remaining.height = remaining.height.saturating_sub(1);
        Some(r)
    } else {
        None
    };

    (remaining, search_rect, status_rect)
}

fn draw_frame(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let search_visible = app.search.mode != SearchMode::Off;
    let (doc_area, search_area, status_area) = doc_and_status_areas(area, search_visible);
    app.viewport.set_height(doc_area.height as usize);

    let lines_for_render = render_lines_with_search(&app.lines, &app.search, &app.theme);

    DocumentWidget {
        lines: &lines_for_render,
        viewport: &app.viewport,
        theme: &app.theme,
    }
    .render(doc_area, frame.buffer_mut());

    let filename = app.filename_for_status();
    let flash_owned = app.flash_active().map(|s| s.to_string());
    StatusBarWidget {
        filename: &filename,
        viewport: &app.viewport,
        flash: flash_owned.as_deref(),
        theme: &app.theme,
    }
    .render(status_area, frame.buffer_mut());

    if let Some(sr) = search_area {
        SearchBarWidget {
            state: &app.search,
            theme: &app.theme,
        }
        .render(sr, frame.buffer_mut());
    }

    match app.popup {
        Popup::None => {}
        Popup::Browser => {
            let pop = centered_rect(area, 70, 70);
            BrowserWidget {
                state: &app.browser,
                theme: &app.theme,
            }
            .render(pop, frame.buffer_mut());
        }
        Popup::Toc => {
            let pop = centered_rect(area, 70, 70);
            TocWidget {
                state: &app.toc,
                theme: &app.theme,
            }
            .render(pop, frame.buffer_mut());
        }
        Popup::ThemeSwitcher => {
            if let Some(state) = app.theme_switcher.as_ref() {
                let pop = centered_rect(area, 50, 60);
                ThemeSwitcherWidget {
                    state,
                    theme: &app.theme,
                }
                .render(pop, frame.buffer_mut());
            }
        }
        Popup::Help => {
            let pop = centered_rect(area, 50, 50);
            draw_help_popup(frame, pop, app);
        }
    }
}

fn render_lines_with_search(
    lines: &[LayoutLine],
    search: &SearchState,
    theme: &veol::render::theme::Theme,
) -> Vec<LayoutLine> {
    if search.mode != SearchMode::Active || search.matches.is_empty() {
        return lines.to_vec();
    }
    let current = search.current_match();
    let mut by_line: std::collections::HashMap<usize, Vec<&_>> = std::collections::HashMap::new();
    for m in &search.matches {
        by_line.entry(m.line_index).or_default().push(m);
    }
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| match by_line.get(&i) {
            Some(ms) => highlight_line_with_matches(l, ms, current, theme),
            None => l.clone(),
        })
        .collect()
}

fn draw_help_popup(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    let bg = app.theme.colors.popup_bg.to_ratatui();
    let border = app.theme.colors.popup_border.to_ratatui();
    let fg = app.theme.colors.fg.to_ratatui();
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_char(' ');
                c.set_style(Style::default().bg(bg));
            }
        }
    }
    let lines = [
        "Veol — Help",
        "",
        "j/k          scroll",
        "Space / b    page down/up",
        "g / G        top / bottom",
        "] / [        next / prev heading",
        "/            search   n / N next / prev match",
        "t            toggle TOC",
        "Ctrl+E       toggle file browser",
        "Ctrl+T       toggle theme switcher",
        "r            reload",
        "f            toggle frontmatter",
        "o            open link externally",
        "?            this help        q  quit",
    ];
    let border_style = Style::default().fg(border).bg(bg);
    let text_style = Style::default()
        .fg(fg)
        .bg(bg)
        .add_modifier(Modifier::empty());
    for x in area.x..area.x + area.width {
        if let Some(c) = buf.cell_mut((x, area.y)) {
            c.set_char('─');
            c.set_style(border_style);
        }
        if let Some(c) = buf.cell_mut((x, area.y + area.height - 1)) {
            c.set_char('─');
            c.set_style(border_style);
        }
    }
    for y in area.y..area.y + area.height {
        if let Some(c) = buf.cell_mut((area.x, y)) {
            c.set_char('│');
            c.set_style(border_style);
        }
        if let Some(c) = buf.cell_mut((area.x + area.width - 1, y)) {
            c.set_char('│');
            c.set_style(border_style);
        }
    }
    for (i, t) in lines.iter().enumerate() {
        let y = area.y + 1 + i as u16;
        if y >= area.y + area.height - 1 {
            break;
        }
        buf.set_stringn(area.x + 2, y, t, area.width as usize - 4, text_style);
    }
}

fn centered_rect(area: Rect, pct_w: u16, pct_h: u16) -> Rect {
    let w = (area.width as u32 * pct_w as u32 / 100) as u16;
    let h = (area.height as u32 * pct_h as u32 / 100) as u16;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

fn handle_key(k: KeyEvent, app: &mut App, area: Rect) {
    match app.popup {
        Popup::Browser => handle_browser_key(k, app),
        Popup::Toc => handle_toc_key(k, app),
        Popup::ThemeSwitcher => handle_theme_switcher_key(k, app),
        Popup::Help => {
            app.popup = Popup::None;
        }
        Popup::None => {
            if app.search.mode == SearchMode::Editing {
                handle_search_editing_key(k, app);
            } else {
                let action = map_reader_key(k);
                app.handle_action(action, area);
            }
        }
    }
}

fn handle_search_editing_key(k: KeyEvent, app: &mut App) {
    match k.code {
        KeyCode::Esc => app.search.cancel(),
        KeyCode::Enter => app.search.confirm(&app.lines),
        KeyCode::Backspace => app.search.backspace(),
        KeyCode::Char(c) => {
            if c == 'u' && k.modifiers.contains(KeyModifiers::CONTROL) {
                app.search.query.clear();
            } else {
                app.search.input_char(c);
            }
        }
        _ => {}
    }
}

fn handle_browser_key(k: KeyEvent, app: &mut App) {
    if app.browser.action != TreeAction::None {
        handle_browser_action_input(k, app);
        return;
    }
    match k.code {
        KeyCode::Esc => app.popup = Popup::None,
        KeyCode::Char('j') | KeyCode::Down => app.browser.move_down(8),
        KeyCode::Char('k') | KeyCode::Up => app.browser.move_up(),
        KeyCode::Enter => {
            if let Err(e) = open_selected_browser_entry(app) {
                app.flash_message(format!("open failed: {e}"));
            }
        }
        KeyCode::Char('n') => app.browser.start_action(TreeAction::NewFile),
        KeyCode::Char('f') => app.browser.start_action(TreeAction::NewFolder),
        KeyCode::Char('r') => app.browser.start_action(TreeAction::Rename),
        KeyCode::Char('d') => app.browser.start_action(TreeAction::Delete),
        KeyCode::Char('m') => app.browser.mark_for_move(),
        KeyCode::Char('z') if k.modifiers.contains(KeyModifiers::CONTROL) => {
            app.browser.undo_last_fs_op();
        }
        KeyCode::Char('y') if k.modifiers.contains(KeyModifiers::CONTROL) => {
            app.browser.redo_last_fs_op();
        }
        KeyCode::Backspace => app.browser.hint_back(),
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as u8 - b'1') as usize;
            if let Some(r) = app.browser.hint_enter(idx) {
                match r {
                    HintResult::EnteredFolder => {}
                    HintResult::OpenFile(p) => {
                        if let Err(e) = handle_browser_open(app, p) {
                            app.flash_message(format!("open failed: {e}"));
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_browser_action_input(k: KeyEvent, app: &mut App) {
    let action = app.browser.action.clone();
    match (action, k.code) {
        (_, KeyCode::Esc) => app.browser.cancel_action(),
        (TreeAction::Delete, KeyCode::Char('y')) => {
            let _ = app.browser.confirm_delete();
        }
        (TreeAction::Delete, KeyCode::Char('n')) => app.browser.cancel_action(),
        (act, KeyCode::Enter) => {
            if let Some(path) = confirm_browser_action(app, act) {
                let _ = handle_hint_result(app, HintResult::OpenFile(path));
            }
        }
        (_, KeyCode::Backspace) => {
            app.browser.input_buf.pop();
        }
        (_, KeyCode::Char(c)) => app.browser.input_buf.push(c),
        _ => {}
    }
}

fn handle_toc_key(k: KeyEvent, app: &mut App) {
    match k.code {
        KeyCode::Esc => app.popup = Popup::None,
        KeyCode::Char('j') | KeyCode::Down => app.toc.move_down(8),
        KeyCode::Char('k') | KeyCode::Up => app.toc.move_up(),
        KeyCode::Enter => {
            if let Some(line) = app.toc.selected_line() {
                app.viewport.jump_to_line(line);
            }
            app.popup = Popup::None;
        }
        _ => {}
    }
}

fn handle_theme_switcher_key(k: KeyEvent, app: &mut App) {
    let Some(state) = app.theme_switcher.as_mut() else {
        app.popup = Popup::None;
        return;
    };
    match k.code {
        KeyCode::Esc => {
            let outcome = state.cancel();
            app.apply_switcher_outcome(outcome);
            app.popup = Popup::None;
        }
        KeyCode::Char('j') | KeyCode::Down => state.move_down(),
        KeyCode::Char('k') | KeyCode::Up => state.move_up(),
        KeyCode::Enter => {
            let outcome = state.confirm(&mut app.config);
            app.apply_switcher_outcome(outcome);
            app.popup = Popup::None;
            app.theme_switcher = None;
        }
        _ => {}
    }
}

#[allow(dead_code)]
// Kept for a future --mouse flag; mouse capture is currently disabled to allow text selection.
fn handle_mouse(m: MouseEvent, app: &mut App, area: Rect) {
    let popup_target = match app.popup {
        Popup::Browser => PopupTarget::Browser,
        Popup::Toc => PopupTarget::Toc,
        Popup::ThemeSwitcher => PopupTarget::ThemeSwitcher,
        Popup::Help => PopupTarget::Help,
        Popup::None => PopupTarget::None,
    };
    match map_mouse(m, popup_target) {
        MouseAction::ScrollUp(n) => app.viewport.scroll_up(n),
        MouseAction::ScrollDown(n) => app.viewport.scroll_down(n),
        _ => {
            let _ = area;
        }
    }
}

// ---------- legacy helpers (theme list, stdin, logging) ----------

fn print_theme_list() -> Result<ExitCode> {
    let mut out = io::stdout().lock();
    for name in BUNDLED_THEMES {
        writeln!(out, "{name}")?;
    }
    for name in user_theme_names()? {
        writeln!(out, "{name}")?;
    }
    Ok(ExitCode::from(0))
}

fn user_theme_names() -> Result<Vec<String>> {
    let Some(dir) = user_themes_dir() else {
        return Ok(Vec::new());
    };
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

fn user_themes_dir() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", "veol")?;
    Some(dirs.config_dir().join("themes"))
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let dirs = ProjectDirs::from("", "", "veol")?;
    let cache_dir = dirs.cache_dir();
    std::fs::create_dir_all(cache_dir).ok()?;
    let file_appender = rolling::daily(cache_dir, "veol.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("veol=info"));
    let _ = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_env_filter(filter)
        .try_init();
    Some(guard)
}
