//! Behavioural contract for the CLI's process-level signals.
//!
//! Exit codes and streams are an API: agents and scripts branch on them. Three
//! defects were found by probing the built binary and are locked here:
//!
//! 1. `--version`/`--help` exited 1 and wrote to stderr (main.rs mapped EVERY
//!    clap error to exit 1), so `gtw --version || echo missing` printed
//!    "missing" on a working install.
//! 2. `analyze` exited 0 when nothing could be parsed, so the first step of the
//!    diagnostic chain reported success while producing no tree.
//! 3. A cached numeric path silently edits whatever now sits at that position.
//!    `--expect-name` turns that into a rejection (verified: without it, one
//!    insert later, `fn a() {}` became `fn CLOBBERED() {}` with exit 0).
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_gnawtreewriter");

fn tmp(name: &str, content: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    // Keep the directory alive for the test's lifetime by leaking it: the
    // process is short-lived and the OS reclaims /tmp.
    std::mem::forget(dir);
    path
}

#[test]
fn version_exits_zero_and_writes_stdout() {
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "--version must not look like a failure"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("gnawtreewriter"),
        "version must go to stdout, not stderr"
    );
    assert!(
        out.stderr.is_empty(),
        "nothing should go to stderr: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_usage_error_is_exit_two_not_one() {
    let out = Command::new(BIN)
        .arg("--definitely-not-a-flag")
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "usage errors exit 2 so callers can tell them from runtime failures (1)"
    );
}

#[test]
fn analyze_of_an_unparseable_file_fails_loudly() {
    let path = tmp("broken.rs", "fn main( {\n");
    let out = Command::new(BIN)
        .arg("analyze")
        .arg(&path)
        .output()
        .unwrap();
    assert_ne!(
        out.status.code(),
        Some(0),
        "analyze must not report success when nothing parsed (skeleton/validate already exit 1)"
    );
}

#[test]
fn analyze_of_a_good_file_still_succeeds() {
    let path = tmp("ok.rs", "fn main() {}\n");
    let out = Command::new(BIN)
        .arg("analyze")
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("function_item"));
}

#[test]
fn expect_name_rejects_a_stale_path_instead_of_editing_the_wrong_node() {
    let path = tmp("shift.rs", "fn a() {}\n\nfn b() {}\n");
    let file = path.to_str().unwrap();

    // "1" is `fn b()`; inserting at position 1 shifts it to `fn a()`.
    let ins = Command::new(BIN)
        .args(["insert", file, "0", "1", "fn c() {}"])
        .output()
        .unwrap();
    assert_eq!(
        ins.status.code(),
        Some(0),
        "insert must work (it used to panic)"
    );

    let before = std::fs::read_to_string(&path).unwrap();

    let edit = Command::new(BIN)
        .args(["edit", file, "1", "fn CLOBBERED() {}", "--expect-name", "b"])
        .output()
        .unwrap();
    assert_ne!(edit.status.code(), Some(0), "stale path must be rejected");
    assert!(
        String::from_utf8_lossy(&edit.stderr).contains("stale path"),
        "error must say so: {:?}",
        String::from_utf8_lossy(&edit.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        before,
        "the file must be untouched when the guard rejects"
    );
}
