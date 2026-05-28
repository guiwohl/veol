use veol::welcome_lines;

#[test]
fn welcome_viewport_snapshot() {
    let lines = welcome_lines();
    let rendered = lines.join("\n");
    insta::assert_snapshot!("welcome_viewport", rendered);
}
