use std::fs;
use veol::mermaid::ascii;

fn read_kitchen() -> String {
    fs::read_to_string("examples/kitchen-sink.md").expect("kitchen-sink.md")
}

fn extract_mermaid_blocks(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut iter = src.lines();
    while let Some(line) = iter.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```mermaid") || trimmed.starts_with("```mmd") {
            let mut body = String::new();
            for inner in iter.by_ref() {
                if inner.trim_start().starts_with("```") {
                    break;
                }
                body.push_str(inner);
                body.push('\n');
            }
            out.push(body);
        }
    }
    out
}

#[test]
fn kitchen_sink_has_every_diagram_type() {
    let blocks = extract_mermaid_blocks(&read_kitchen());
    assert!(
        blocks.len() >= 19,
        "expected >= 19 mermaid blocks for every supported diagram type, got {}",
        blocks.len()
    );
}

#[test]
fn every_mermaid_block_renders_without_panic() {
    for block in extract_mermaid_blocks(&read_kitchen()) {
        let _ = ascii::render_mermaid(&block, 80);
    }
}

#[test]
fn every_mermaid_block_emits_at_least_one_line() {
    for block in extract_mermaid_blocks(&read_kitchen()) {
        let lines = ascii::render_mermaid(&block, 80);
        assert!(!lines.is_empty(), "empty rendering for block: {block}");
    }
}

#[test]
fn no_block_returns_empty_strings_only() {
    for block in extract_mermaid_blocks(&read_kitchen()) {
        let lines = ascii::render_mermaid(&block, 80);
        assert!(
            lines.iter().any(|l| !l.trim().is_empty()),
            "block produced only whitespace lines: {block}"
        );
    }
}

#[test]
fn flowchart_block_renders_with_box_chars() {
    let src = "flowchart TD\n    A[Start] --> B{Branch}\n    B --> C[Left]\n    B --> D[Right]\n";
    let lines = ascii::render_mermaid(src, 80);
    assert!(
        lines.iter().any(|l| l.contains('┌') || l.contains('+')),
        "flowchart should produce a box-drawing line, got: {lines:?}"
    );
}
