use std::collections::HashMap;

use crate::mermaid::ascii;
use crate::mermaid::ascii::StyledRow;

#[derive(Debug, Default, Clone)]
pub struct DisplayRegistry {
    rendered: HashMap<usize, Vec<StyledRow>>,
}

impl DisplayRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, index: usize) -> Option<&[StyledRow]> {
        self.rendered.get(&index).map(|v| v.as_slice())
    }

    pub fn insert(&mut self, index: usize, rows: Vec<StyledRow>) {
        self.rendered.insert(index, rows);
    }

    pub fn ensure_rendered(&mut self, index: usize, source: &str, max_width: u16) -> &[StyledRow] {
        self.rendered
            .entry(index)
            .or_insert_with(|| ascii::render_mermaid_styled(source, max_width))
            .as_slice()
    }

    pub fn reset(&mut self) {
        self.rendered.clear();
    }

    pub fn len(&self) -> usize {
        self.rendered.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rendered.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::ascii::{plain_row, StyledRun};

    fn flatten(rows: &[StyledRow]) -> Vec<String> {
        rows.iter()
            .map(|r| r.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect()
    }

    #[test]
    fn get_returns_none_for_missing_index() {
        let r = DisplayRegistry::new();
        assert!(r.get(0).is_none());
    }

    #[test]
    fn insert_then_get_returns_rows() {
        let mut r = DisplayRegistry::new();
        r.insert(2, vec![plain_row("hello"), plain_row("world")]);
        let got = r.get(2).expect("rows");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0][0].text, "hello");
        assert_eq!(got[1][0].text, "world");
    }

    #[test]
    fn ensure_rendered_caches_and_returns_rows() {
        let mut r = DisplayRegistry::new();
        let src = "graph LR\nA-->B\n";
        let first = r.ensure_rendered(0, src, 80).to_vec();
        assert!(!first.is_empty());
        let again = r.ensure_rendered(0, src, 80).to_vec();
        assert_eq!(flatten(&again), flatten(&first));
    }

    #[test]
    fn ensure_rendered_unknown_diagram_falls_back_to_source() {
        let mut r = DisplayRegistry::new();
        let src = "totallyUnknownDiagram\n  thing\n";
        let rows = r.ensure_rendered(0, src, 80).to_vec();
        let flat = flatten(&rows);
        assert!(flat
            .first()
            .map(|s| s.starts_with("// mermaid:"))
            .unwrap_or(false));
        assert!(flat.iter().any(|l| l == "```mermaid"));
        assert!(flat.iter().any(|l| l == "```"));
    }

    #[test]
    fn reset_clears_all_rendered() {
        let mut r = DisplayRegistry::new();
        r.insert(
            0,
            vec![vec![StyledRun {
                text: "x".into(),
                color: None,
            }]],
        );
        r.insert(
            1,
            vec![vec![StyledRun {
                text: "y".into(),
                color: None,
            }]],
        );
        assert_eq!(r.len(), 2);
        r.reset();
        assert!(r.is_empty());
    }
}
