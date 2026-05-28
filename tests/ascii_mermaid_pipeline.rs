use veol::mermaid::ascii;

#[test]
fn renders_flowchart_inline() {
    let src = "graph LR\n    A[Start] --> B{Decision}\n    B -->|yes| C[End]\n    B -->|no| A\n";
    let lines = ascii::render_mermaid(src, 80);
    assert!(
        lines.len() >= 3,
        "flowchart should produce >=3 rows, got {}: {lines:?}",
        lines.len()
    );
    assert!(
        !lines
            .first()
            .map(|s| s.starts_with("// mermaid:"))
            .unwrap_or(false),
        "flowchart should not fall back to source: {lines:?}"
    );
}

#[test]
fn renders_sequence_inline() {
    let src = "sequenceDiagram\n    Alice->>Bob: Hi\n    Bob-->>Alice: Hey\n";
    let lines = ascii::render_mermaid(src, 80);
    assert!(
        lines.len() >= 3,
        "sequence should produce >=3 rows, got {}: {lines:?}",
        lines.len()
    );
    assert!(
        !lines
            .first()
            .map(|s| s.starts_with("// mermaid:"))
            .unwrap_or(false),
        "sequence should not fall back to source: {lines:?}"
    );
}

#[test]
fn unknown_kind_falls_back_to_source_block() {
    let src = "totallyUnknownDiagram\n    A: 1\n";
    let lines = ascii::render_mermaid(src, 80);
    assert!(!lines.is_empty(), "must produce something");
    assert!(
        lines
            .first()
            .map(|s| s.starts_with("// mermaid:"))
            .unwrap_or(false),
        "fallback first line should be a note: {lines:?}"
    );
    assert!(lines.iter().any(|l| l == "```mermaid"));
    assert_eq!(lines.last().map(String::as_str), Some("```"));
}

#[test]
fn class_diagram_renders_or_falls_back_without_panicking() {
    let src = "classDiagram\n    Animal <|-- Dog\n";
    let lines = ascii::render_mermaid(src, 80);
    assert!(!lines.is_empty(), "must produce at least one line");
}

#[test]
fn pie_chart_renders_or_falls_back_without_panicking() {
    let src = "pie title Pets\n    \"Dogs\" : 50\n    \"Cats\" : 30\n";
    let lines = ascii::render_mermaid(src, 80);
    assert!(!lines.is_empty(), "must produce at least one line");
}
