use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    ScrollDown(usize),
    ScrollUp(usize),
    PageDown,
    PageUp,
    HalfPageDown,
    HalfPageUp,
    JumpTop,
    JumpBottom,
    NextHeading,
    PrevHeading,
    NextParagraph,
    PrevParagraph,
    StartSearch,
    NextMatch,
    PrevMatch,
    ToggleToc,
    Reload,
    ToggleMermaid,
    OpenLink,
    ToggleBrowser,
    ToggleThemeSwitcher,
    ToggleFrontmatter,
    ToggleHelp,
    Escape,
    None,
}

pub fn map_reader_key(ev: KeyEvent) -> Action {
    let shift = ev.modifiers.contains(KeyModifiers::SHIFT);
    let ctrl = ev.modifiers.contains(KeyModifiers::CONTROL);

    match ev.code {
        KeyCode::Char('q') if !ctrl => Action::Quit,
        KeyCode::Char('e') if ctrl => Action::ToggleBrowser,
        KeyCode::Char('t') if ctrl => Action::ToggleThemeSwitcher,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown(1),
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp(1),
        KeyCode::Char(' ') => Action::PageDown,
        KeyCode::Char('b') => Action::PageUp,
        KeyCode::PageDown => Action::NextParagraph,
        KeyCode::PageUp => Action::PrevParagraph,
        KeyCode::Char('d') if !ctrl => Action::HalfPageDown,
        KeyCode::Char('u') if !ctrl => Action::HalfPageUp,
        KeyCode::Char('g') if !shift => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        KeyCode::Char('g') if shift => Action::JumpBottom,
        KeyCode::Char(']') => Action::NextHeading,
        KeyCode::Char('[') => Action::PrevHeading,
        KeyCode::Char('/') => Action::StartSearch,
        KeyCode::Char('n') if !shift => Action::NextMatch,
        KeyCode::Char('N') => Action::PrevMatch,
        KeyCode::Char('n') if shift => Action::PrevMatch,
        KeyCode::Char('t') if !ctrl => Action::ToggleToc,
        KeyCode::Char('r') if !ctrl => Action::Reload,
        KeyCode::Char('m') if !ctrl => Action::ToggleMermaid,
        KeyCode::Char('f') if !ctrl => Action::ToggleFrontmatter,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('o') if !ctrl => Action::OpenLink,
        KeyCode::Esc => Action::Escape,
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn k_mod(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn quit_on_q() {
        assert_eq!(map_reader_key(k(KeyCode::Char('q'))), Action::Quit);
    }

    #[test]
    fn down_on_j_and_arrow_down() {
        assert_eq!(map_reader_key(k(KeyCode::Char('j'))), Action::ScrollDown(1));
        assert_eq!(map_reader_key(k(KeyCode::Down)), Action::ScrollDown(1));
    }

    #[test]
    fn up_on_k_and_arrow_up() {
        assert_eq!(map_reader_key(k(KeyCode::Char('k'))), Action::ScrollUp(1));
        assert_eq!(map_reader_key(k(KeyCode::Up)), Action::ScrollUp(1));
    }

    #[test]
    fn space_is_page_down() {
        assert_eq!(map_reader_key(k(KeyCode::Char(' '))), Action::PageDown);
    }

    #[test]
    fn b_is_page_up() {
        assert_eq!(map_reader_key(k(KeyCode::Char('b'))), Action::PageUp);
    }

    #[test]
    fn pgdn_jumps_next_paragraph() {
        assert_eq!(map_reader_key(k(KeyCode::PageDown)), Action::NextParagraph);
    }

    #[test]
    fn pgup_jumps_prev_paragraph() {
        assert_eq!(map_reader_key(k(KeyCode::PageUp)), Action::PrevParagraph);
    }

    #[test]
    fn d_and_u_half_page() {
        assert_eq!(map_reader_key(k(KeyCode::Char('d'))), Action::HalfPageDown);
        assert_eq!(map_reader_key(k(KeyCode::Char('u'))), Action::HalfPageUp);
    }

    #[test]
    fn g_top_g_bottom() {
        assert_eq!(map_reader_key(k(KeyCode::Char('g'))), Action::JumpTop);
        assert_eq!(map_reader_key(k(KeyCode::Char('G'))), Action::JumpBottom);
    }

    #[test]
    fn bracket_next_prev_heading() {
        assert_eq!(map_reader_key(k(KeyCode::Char(']'))), Action::NextHeading);
        assert_eq!(map_reader_key(k(KeyCode::Char('['))), Action::PrevHeading);
    }

    #[test]
    fn slash_starts_search() {
        assert_eq!(map_reader_key(k(KeyCode::Char('/'))), Action::StartSearch);
    }

    #[test]
    fn n_and_n_for_match_navigation() {
        assert_eq!(map_reader_key(k(KeyCode::Char('n'))), Action::NextMatch);
        assert_eq!(map_reader_key(k(KeyCode::Char('N'))), Action::PrevMatch);
    }

    #[test]
    fn t_toggles_toc() {
        assert_eq!(map_reader_key(k(KeyCode::Char('t'))), Action::ToggleToc);
    }

    #[test]
    fn r_reloads() {
        assert_eq!(map_reader_key(k(KeyCode::Char('r'))), Action::Reload);
    }

    #[test]
    fn m_toggles_mermaid() {
        assert_eq!(map_reader_key(k(KeyCode::Char('m'))), Action::ToggleMermaid);
    }

    #[test]
    fn f_toggles_frontmatter() {
        assert_eq!(
            map_reader_key(k(KeyCode::Char('f'))),
            Action::ToggleFrontmatter
        );
    }

    #[test]
    fn question_mark_toggles_help() {
        assert_eq!(map_reader_key(k(KeyCode::Char('?'))), Action::ToggleHelp);
    }

    #[test]
    fn ctrl_e_toggles_browser() {
        assert_eq!(
            map_reader_key(k_mod(KeyCode::Char('e'), KeyModifiers::CONTROL)),
            Action::ToggleBrowser
        );
    }

    #[test]
    fn ctrl_t_toggles_theme_switcher() {
        assert_eq!(
            map_reader_key(k_mod(KeyCode::Char('t'), KeyModifiers::CONTROL)),
            Action::ToggleThemeSwitcher
        );
    }

    #[test]
    fn o_opens_link() {
        assert_eq!(map_reader_key(k(KeyCode::Char('o'))), Action::OpenLink);
    }

    #[test]
    fn esc_returns_escape() {
        assert_eq!(map_reader_key(k(KeyCode::Esc)), Action::Escape);
    }
}
