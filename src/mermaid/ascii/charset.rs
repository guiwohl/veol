#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharsetKind {
    Unicode,
    Ascii,
}

pub trait Charset {
    fn horiz(&self) -> char;
    fn vert(&self) -> char;
    fn top_left(&self) -> char;
    fn top_right(&self) -> char;
    fn bot_left(&self) -> char;
    fn bot_right(&self) -> char;
    fn tee_right(&self) -> char;
    fn tee_left(&self) -> char;
    fn tee_down(&self) -> char;
    fn tee_up(&self) -> char;
    fn cross(&self) -> char;
    fn arrow_up(&self) -> char;
    fn arrow_down(&self) -> char;
    fn arrow_left(&self) -> char;
    fn arrow_right(&self) -> char;
    fn diag_down(&self) -> char;
    fn diag_up(&self) -> char;
    fn round_top_left(&self) -> char;
    fn round_top_right(&self) -> char;
    fn round_bot_left(&self) -> char;
    fn round_bot_right(&self) -> char;
}

pub struct UnicodeSet;
pub struct AsciiSet;

impl Charset for UnicodeSet {
    fn horiz(&self) -> char {
        '─'
    }
    fn vert(&self) -> char {
        '│'
    }
    fn top_left(&self) -> char {
        '┌'
    }
    fn top_right(&self) -> char {
        '┐'
    }
    fn bot_left(&self) -> char {
        '└'
    }
    fn bot_right(&self) -> char {
        '┘'
    }
    fn tee_right(&self) -> char {
        '├'
    }
    fn tee_left(&self) -> char {
        '┤'
    }
    fn tee_down(&self) -> char {
        '┬'
    }
    fn tee_up(&self) -> char {
        '┴'
    }
    fn cross(&self) -> char {
        '┼'
    }
    fn arrow_up(&self) -> char {
        '▲'
    }
    fn arrow_down(&self) -> char {
        '▼'
    }
    fn arrow_left(&self) -> char {
        '◄'
    }
    fn arrow_right(&self) -> char {
        '►'
    }
    fn diag_down(&self) -> char {
        '╲'
    }
    fn diag_up(&self) -> char {
        '╱'
    }
    fn round_top_left(&self) -> char {
        '╭'
    }
    fn round_top_right(&self) -> char {
        '╮'
    }
    fn round_bot_left(&self) -> char {
        '╰'
    }
    fn round_bot_right(&self) -> char {
        '╯'
    }
}

impl Charset for AsciiSet {
    fn horiz(&self) -> char {
        '-'
    }
    fn vert(&self) -> char {
        '|'
    }
    fn top_left(&self) -> char {
        '+'
    }
    fn top_right(&self) -> char {
        '+'
    }
    fn bot_left(&self) -> char {
        '+'
    }
    fn bot_right(&self) -> char {
        '+'
    }
    fn tee_right(&self) -> char {
        '+'
    }
    fn tee_left(&self) -> char {
        '+'
    }
    fn tee_down(&self) -> char {
        '+'
    }
    fn tee_up(&self) -> char {
        '+'
    }
    fn cross(&self) -> char {
        '+'
    }
    fn arrow_up(&self) -> char {
        '^'
    }
    fn arrow_down(&self) -> char {
        'v'
    }
    fn arrow_left(&self) -> char {
        '<'
    }
    fn arrow_right(&self) -> char {
        '>'
    }
    fn diag_down(&self) -> char {
        '\\'
    }
    fn diag_up(&self) -> char {
        '/'
    }
    fn round_top_left(&self) -> char {
        '+'
    }
    fn round_top_right(&self) -> char {
        '+'
    }
    fn round_bot_left(&self) -> char {
        '+'
    }
    fn round_bot_right(&self) -> char {
        '+'
    }
}

static UNICODE: UnicodeSet = UnicodeSet;
static ASCII: AsciiSet = AsciiSet;

pub fn get_charset(kind: CharsetKind) -> &'static dyn Charset {
    match kind {
        CharsetKind::Unicode => &UNICODE,
        CharsetKind::Ascii => &ASCII,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_unicode_horiz_is_box_char() {
        assert_eq!(get_charset(CharsetKind::Unicode).horiz(), '─');
    }

    #[test]
    fn charset_ascii_horiz_is_dash() {
        assert_eq!(get_charset(CharsetKind::Ascii).horiz(), '-');
    }

    #[test]
    fn charset_unicode_corners_differ() {
        let cs = get_charset(CharsetKind::Unicode);
        assert_eq!(cs.top_left(), '┌');
        assert_eq!(cs.top_right(), '┐');
        assert_eq!(cs.bot_left(), '└');
        assert_eq!(cs.bot_right(), '┘');
        assert_ne!(cs.top_left(), cs.top_right());
        assert_ne!(cs.top_left(), cs.bot_left());
    }

    #[test]
    fn charset_ascii_corners_all_plus() {
        let cs = get_charset(CharsetKind::Ascii);
        assert_eq!(cs.top_left(), '+');
        assert_eq!(cs.top_right(), '+');
        assert_eq!(cs.bot_left(), '+');
        assert_eq!(cs.bot_right(), '+');
    }
}
