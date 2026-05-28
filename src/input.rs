use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupTarget {
    Browser,
    Toc,
    ThemeSwitcher,
    Help,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MouseAction {
    None,
    ScrollUp(usize),
    ScrollDown(usize),
    ClickAt { x: u16, y: u16, target: PopupTarget },
    LeftPress { x: u16, y: u16 },
    RightPress { x: u16, y: u16 },
}

pub fn map_mouse(event: MouseEvent, foreground_popup: PopupTarget) -> MouseAction {
    match event.kind {
        MouseEventKind::ScrollUp => MouseAction::ScrollUp(3),
        MouseEventKind::ScrollDown => MouseAction::ScrollDown(3),
        MouseEventKind::Down(MouseButton::Left) => {
            if foreground_popup != PopupTarget::None {
                MouseAction::ClickAt {
                    x: event.column,
                    y: event.row,
                    target: foreground_popup,
                }
            } else {
                MouseAction::LeftPress {
                    x: event.column,
                    y: event.row,
                }
            }
        }
        MouseEventKind::Down(MouseButton::Right) => MouseAction::RightPress {
            x: event.column,
            y: event.row,
        },
        _ => MouseAction::None,
    }
}

pub fn click_to_row_index(
    inner: ratatui::layout::Rect,
    title_rows: u16,
    hint_rows: u16,
    scroll_offset: usize,
    x: u16,
    y: u16,
) -> Option<usize> {
    if y < inner.y + title_rows {
        return None;
    }
    if y >= inner.y + inner.height - hint_rows {
        return None;
    }
    if x < inner.x || x >= inner.x + inner.width {
        return None;
    }
    Some(scroll_offset + (y - inner.y - title_rows) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    fn me(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn scroll_up_returns_scroll_up_three() {
        let ev = me(MouseEventKind::ScrollUp, 0, 0);
        assert_eq!(map_mouse(ev, PopupTarget::None), MouseAction::ScrollUp(3));
    }

    #[test]
    fn scroll_down_returns_scroll_down_three() {
        let ev = me(MouseEventKind::ScrollDown, 0, 0);
        assert_eq!(map_mouse(ev, PopupTarget::None), MouseAction::ScrollDown(3));
    }

    #[test]
    fn left_press_with_no_popup_returns_left_press() {
        let ev = me(MouseEventKind::Down(MouseButton::Left), 7, 11);
        assert_eq!(
            map_mouse(ev, PopupTarget::None),
            MouseAction::LeftPress { x: 7, y: 11 }
        );
    }

    #[test]
    fn left_press_with_browser_popup_returns_click_at_browser() {
        let ev = me(MouseEventKind::Down(MouseButton::Left), 4, 9);
        assert_eq!(
            map_mouse(ev, PopupTarget::Browser),
            MouseAction::ClickAt {
                x: 4,
                y: 9,
                target: PopupTarget::Browser,
            }
        );
    }

    #[test]
    fn left_press_with_toc_popup_returns_click_at_toc() {
        let ev = me(MouseEventKind::Down(MouseButton::Left), 2, 3);
        assert_eq!(
            map_mouse(ev, PopupTarget::Toc),
            MouseAction::ClickAt {
                x: 2,
                y: 3,
                target: PopupTarget::Toc,
            }
        );
    }

    #[test]
    fn right_press_returns_right_press() {
        let ev = me(MouseEventKind::Down(MouseButton::Right), 5, 6);
        assert_eq!(
            map_mouse(ev, PopupTarget::None),
            MouseAction::RightPress { x: 5, y: 6 }
        );
    }

    #[test]
    fn mouse_up_returns_none() {
        let ev = me(MouseEventKind::Up(MouseButton::Left), 5, 6);
        assert_eq!(map_mouse(ev, PopupTarget::None), MouseAction::None);
    }

    #[test]
    fn drag_returns_none() {
        let ev = me(MouseEventKind::Drag(MouseButton::Left), 5, 6);
        assert_eq!(map_mouse(ev, PopupTarget::Browser), MouseAction::None);
    }

    #[test]
    fn moved_returns_none() {
        let ev = me(MouseEventKind::Moved, 5, 6);
        assert_eq!(map_mouse(ev, PopupTarget::None), MouseAction::None);
    }

    fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    #[test]
    fn click_above_title_returns_none() {
        let r = rect(10, 5, 40, 20);
        assert_eq!(click_to_row_index(r, 1, 1, 0, 15, 5), None);
    }

    #[test]
    fn click_on_first_content_row_returns_scroll_offset() {
        let r = rect(10, 5, 40, 20);
        assert_eq!(click_to_row_index(r, 1, 1, 0, 15, 6), Some(0));
    }

    #[test]
    fn click_on_third_row_returns_scroll_offset_plus_two() {
        let r = rect(10, 5, 40, 20);
        assert_eq!(click_to_row_index(r, 1, 1, 0, 15, 8), Some(2));
    }

    #[test]
    fn click_in_hint_bar_returns_none() {
        let r = rect(10, 5, 40, 20);
        assert_eq!(click_to_row_index(r, 1, 1, 0, 15, 24), None);
    }

    #[test]
    fn click_outside_horizontal_returns_none() {
        let r = rect(10, 5, 40, 20);
        assert_eq!(click_to_row_index(r, 1, 1, 0, 9, 10), None);
        assert_eq!(click_to_row_index(r, 1, 1, 0, 50, 10), None);
    }

    #[test]
    fn click_with_nonzero_scroll_offset_offsets_correctly() {
        let r = rect(10, 5, 40, 20);
        assert_eq!(click_to_row_index(r, 1, 1, 17, 15, 8), Some(19));
    }
}
