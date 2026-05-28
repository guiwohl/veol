use std::path::PathBuf;
use std::process::Command;

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
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn help_flag_prints_usage() {
    let (code, stdout, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("veol"));
    assert!(stdout.contains("--no-watch"));
    assert!(stdout.contains("--no-mermaid"));
}

#[test]
fn version_flag_prints_version() {
    let (code, stdout, _) = run(&["--version"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn unknown_flag_errors() {
    let (code, _, stderr) = run(&["--definitely-not-a-flag"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty());
}

#[test]
fn no_watch_flag_parses() {
    let (code, _, stderr) = run(&["--no-watch", "/this/path/does/not/exist.md"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("file not found"));
}

#[test]
fn missing_file_prints_error_and_exits_one() {
    let (code, _, stderr) = run(&["README.does.not.exist.md"]);
    assert_eq!(code, 1);
    assert_eq!(
        stderr.trim(),
        "veol: file not found: README.does.not.exist.md"
    );
}

#[test]
fn pager_and_no_pager_conflict() {
    let (code, _, stderr) = run(&["--pager", "--no-pager"]);
    assert_ne!(code, 0);
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("cannot be used with") || lower.contains("conflict"),
        "stderr was: {stderr}"
    );
}

#[test]
fn pager_and_plain_conflict() {
    let (code, _, stderr) = run(&["--pager", "--plain"]);
    assert_ne!(code, 0);
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("cannot be used with") || lower.contains("conflict"),
        "stderr was: {stderr}"
    );
}

#[test]
fn width_below_min_rejected() {
    let (code, _, stderr) = run(&["--width", "10"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty());
}

#[test]
fn width_at_or_above_min_accepted() {
    let (code, _, _) = run(&["--width", "80", "--theme-list"]);
    assert_eq!(code, 0);
}

#[test]
fn no_mermaid_flag_accepted() {
    let (code, _, _) = run(&["--no-mermaid", "--theme-list"]);
    assert_eq!(code, 0);
}

#[test]
fn theme_list_prints_bundled_themes() {
    let (code, stdout, _) = run(&["--theme-list"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() >= 9,
        "expected >=9 themes, got {}: {stdout}",
        lines.len()
    );
}
