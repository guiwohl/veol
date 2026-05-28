use sha2::{Digest, Sha256};

use crate::markdown::model::Block;

pub const VEOL_RENDER_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Png,
    Svg,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Svg => "svg",
        }
    }

    fn tag(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Svg => "svg",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MermaidJob {
    pub block_index: usize,
    pub source: String,
    pub cache_key: String,
}

fn normalize_source(raw: &str) -> String {
    let mut lines: Vec<&str> = raw
        .split('\n')
        .map(|l| l.trim_end_matches(|c: char| c.is_whitespace()))
        .collect();
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines.join("\n")
}

fn compute_cache_key(
    source: &str,
    theme: &str,
    format: OutputFormat,
    renderer_version: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(b"\x00");
    hasher.update(theme.as_bytes());
    hasher.update(b"\x00");
    hasher.update(format.tag().as_bytes());
    hasher.update(b"\x00");
    hasher.update(b"mmdc");
    hasher.update(b"\x00");
    hasher.update(renderer_version.as_bytes());
    hasher.update(b"\x00");
    hasher.update(VEOL_RENDER_VERSION.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

pub fn extract_jobs(
    blocks: &[Block],
    theme: &str,
    output_format: OutputFormat,
    renderer_version: &str,
) -> Vec<MermaidJob> {
    let mut jobs = Vec::new();
    for (idx, block) in blocks.iter().enumerate() {
        if let Block::MermaidBlock { source, .. } = block {
            let normalized = normalize_source(source);
            let cache_key = compute_cache_key(&normalized, theme, output_format, renderer_version);
            jobs.push(MermaidJob {
                block_index: idx,
                source: normalized,
                cache_key,
            });
        }
    }
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::model::{Span, SpanStyle};

    fn mermaid(src: &str) -> Block {
        Block::MermaidBlock {
            source: src.to_string(),
        }
    }

    fn paragraph(text: &str) -> Block {
        Block::Paragraph {
            spans: vec![Span {
                text: text.to_string(),
                style: SpanStyle::default(),
            }],
        }
    }

    #[test]
    fn extract_returns_empty_for_no_mermaid() {
        let blocks = vec![paragraph("hello"), Block::Rule];
        let jobs = extract_jobs(&blocks, "default", OutputFormat::Png, "10.0.0");
        assert!(jobs.is_empty());
    }

    #[test]
    fn extract_emits_one_job_per_mermaid_block() {
        let blocks = vec![
            paragraph("intro"),
            mermaid("graph TD\nA --> B"),
            paragraph("between"),
            mermaid("sequenceDiagram\nA->>B: hi"),
        ];
        let jobs = extract_jobs(&blocks, "default", OutputFormat::Png, "10.0.0");
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].block_index, 1);
        assert_eq!(jobs[1].block_index, 3);
    }

    #[test]
    fn extract_normalizes_trailing_whitespace() {
        let blocks = vec![mermaid("graph TD   \nA --> B\t\n\n\n")];
        let jobs = extract_jobs(&blocks, "default", OutputFormat::Png, "10.0.0");
        assert_eq!(jobs[0].source, "graph TD\nA --> B");
    }

    #[test]
    fn extract_cache_key_changes_with_theme() {
        let blocks = vec![mermaid("graph TD\nA --> B")];
        let a = extract_jobs(&blocks, "dark", OutputFormat::Png, "10.0.0");
        let b = extract_jobs(&blocks, "light", OutputFormat::Png, "10.0.0");
        assert_ne!(a[0].cache_key, b[0].cache_key);
    }

    #[test]
    fn extract_cache_key_changes_with_format() {
        let blocks = vec![mermaid("graph TD\nA --> B")];
        let a = extract_jobs(&blocks, "dark", OutputFormat::Png, "10.0.0");
        let b = extract_jobs(&blocks, "dark", OutputFormat::Svg, "10.0.0");
        assert_ne!(a[0].cache_key, b[0].cache_key);
    }

    #[test]
    fn extract_cache_key_changes_with_renderer_version() {
        let blocks = vec![mermaid("graph TD\nA --> B")];
        let a = extract_jobs(&blocks, "dark", OutputFormat::Png, "10.0.0");
        let b = extract_jobs(&blocks, "dark", OutputFormat::Png, "10.1.0");
        assert_ne!(a[0].cache_key, b[0].cache_key);
    }

    #[test]
    fn extract_cache_key_stable_for_identical_input() {
        let blocks = vec![mermaid("graph TD\nA --> B")];
        let a = extract_jobs(&blocks, "dark", OutputFormat::Png, "10.0.0");
        let b = extract_jobs(&blocks, "dark", OutputFormat::Png, "10.0.0");
        assert_eq!(a[0].cache_key, b[0].cache_key);
        assert_eq!(a[0].cache_key.len(), 64);
    }

    #[test]
    fn extract_block_index_matches_position_in_doc() {
        let blocks = vec![
            paragraph("0"),
            paragraph("1"),
            mermaid("graph TD\nA --> B"),
            paragraph("3"),
            mermaid("graph TD\nC --> D"),
        ];
        let jobs = extract_jobs(&blocks, "default", OutputFormat::Png, "10.0.0");
        assert_eq!(jobs[0].block_index, 2);
        assert_eq!(jobs[1].block_index, 4);
    }
}
