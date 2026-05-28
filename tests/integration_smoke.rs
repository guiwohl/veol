use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn veol_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_BIN_EXE_veol"));
    p.set_file_name("veol");
    p
}

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(veol_bin())
        .args(args)
        .env_remove("RUST_LOG")
        .output()
        .expect("spawn veol");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn run_plain_stdin(markdown: &str) -> (i32, String) {
    let mut child = Command::new(veol_bin())
        .args(["--plain", "--no-pager", "-"])
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(markdown.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("wait");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn version_prints_clean() {
    let (code, stdout, stderr) = run(&["--version"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("veol") || stdout.contains("0.1"));
}

#[test]
fn help_prints_clean() {
    let (code, stdout, stderr) = run(&["--help"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("veol"));
    assert!(stdout.contains("--theme-list"));
    assert!(stdout.contains("--no-watch"));
}

#[test]
fn theme_list_includes_all_bundled() {
    let (code, stdout, _) = run(&["--theme-list"]);
    assert_eq!(code, 0);
    let needed = [
        "reedo-dark",
        "reedo-light",
        "catppuccin",
        "dracula",
        "gruvbox",
        "nord",
        "rose-pine",
        "solarized-dark",
    ];
    for name in needed {
        assert!(
            stdout.lines().any(|l| l.trim() == name),
            "missing theme '{name}' in: {stdout}"
        );
    }
}

#[test]
fn nonexistent_file_returns_error() {
    let (code, _, stderr) = run(&["/no/such/path/here-xyz.md"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("file not found"), "stderr: {stderr}");
}

#[test]
fn plain_mode_renders_to_stdout() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("doc.md");
    std::fs::write(&path, "# Hi\n\nHello world.\n").unwrap();
    let (code, stdout, stderr) = run(&["--plain", path.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("Hi"), "stdout missing 'Hi': {stdout}");
    assert!(stdout.contains("Hello"), "stdout missing 'Hello': {stdout}");
}

#[test]
fn plain_mode_reads_stdin_dash() {
    let mut child = Command::new(veol_bin())
        .args(["--plain", "-"])
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"# Test\n\nHello stdin.\n").unwrap();
    }
    let out = child.wait_with_output().expect("wait");
    assert_eq!(out.status.code().unwrap_or(-1), 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Test"), "got: {stdout}");
    assert!(stdout.contains("Hello"), "got: {stdout}");
}

#[test]
fn plain_mode_with_mermaid_renders_inline_ascii() {
    let (code, stdout) = run_plain_stdin("```mermaid\ngraph LR\nA-->B\n```\n");
    assert_eq!(code, 0);
    // Either an ASCII box rendered or, worst case, the source code-block fallback.
    // Either way, the OLD "Mermaid diagram — rendering..." placeholder must NOT appear.
    assert!(
        !stdout.contains("Mermaid diagram"),
        "old placeholder leaked: {stdout}"
    );
    assert!(
        stdout.contains('A') && stdout.contains('B'),
        "got: {stdout}"
    );
}

#[test]
fn plain_mode_with_code_fence_renders_content() {
    let (code, stdout) = run_plain_stdin("```rust\nfn foo() {}\n```\n");
    assert_eq!(code, 0);
    assert!(
        stdout.contains("fn foo()"),
        "expected fenced rust code body in: {stdout}"
    );
}

#[test]
fn plain_mode_with_footnote_renders_reference_and_body() {
    let (code, stdout) = run_plain_stdin("a footnote ref[^1] here.\n\n[^1]: body text alpha.\n");
    assert_eq!(code, 0);
    assert!(
        stdout.contains("[^1]"),
        "expected '[^1]' reference in: {stdout}"
    );
    assert!(
        stdout.contains("body text alpha"),
        "expected footnote body in: {stdout}"
    );
}

#[test]
fn plain_mode_with_table_renders_borders() {
    let (code, stdout) = run_plain_stdin("| A | B |\n|---|---|\n| 1 | 2 |\n");
    assert_eq!(code, 0);
    assert!(stdout.contains('│'), "expected table cell sep in: {stdout}");
    assert!(stdout.contains('─'), "expected border in: {stdout}");
}

#[test]
fn plain_mode_with_frontmatter_renders_card() {
    let (code, stdout) = run_plain_stdin("---\ntitle: Hello\nauthor: Me\n---\n\nBody paragraph.\n");
    assert_eq!(code, 0);
    assert!(stdout.contains('┌'), "expected card border in: {stdout}");
    assert!(stdout.contains("title"), "expected key in: {stdout}");
    assert!(stdout.contains("Hello"), "expected value in: {stdout}");
}

#[test]
fn plain_mode_with_blockquote_renders_marker() {
    let (code, stdout) = run_plain_stdin("> a quoted line\n");
    assert_eq!(code, 0);
    assert!(stdout.contains('│'), "expected quote marker in: {stdout}");
}

#[test]
fn plain_mode_with_task_list_renders_checkboxes() {
    let (code, stdout) = run_plain_stdin("- [x] done item\n- [ ] pending item\n");
    assert_eq!(code, 0);
    assert!(stdout.contains("[x]"), "expected checked box in: {stdout}");
    assert!(stdout.contains("[ ]"), "expected empty box in: {stdout}");
}
