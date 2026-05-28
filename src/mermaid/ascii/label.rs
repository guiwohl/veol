use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Default)]
pub struct Label {
    pub lines: Vec<String>,
}

impl Label {
    pub fn new<S: Into<String>>(text: S) -> Self {
        let text = text.into();
        if text.is_empty() {
            return Self { lines: vec![] };
        }
        let lines = text.split("<br>").map(|s| s.trim().to_string()).collect();
        Self { lines }
    }

    pub fn width(&self) -> usize {
        self.lines
            .iter()
            .map(|l| UnicodeWidthStr::width(l.as_str()))
            .max()
            .unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.lines.len().max(1)
    }
}
