//! End-to-end tests for the `rune` CLI binary.
//!
//! Each test shells out to the compiled binary (via `CARGO_BIN_EXE_rune`,
//! which Cargo sets for integration tests) and asserts on its stdout/stderr
//! and exit status. This pins the *user-facing contract* of each command —
//! the unit tests in the library crates cover the internals.

use std::process::{Command, Output};

/// Invoke the `rune` binary with `args` and capture its output.
fn rune(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rune"))
        .args(args)
        .output()
        .expect("failed to spawn rune binary")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ── eval ─────────────────────────────────────────────────────────────────────

#[test]
fn eval_arithmetic_prints_atom() {
    let out = rune(&["eval", "6 * 7"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out).trim(), "42");
}

#[test]
fn eval_prysm_value_renders_as_chunk() {
    // A program whose *value* is a renderable noun decodes to a tape chunk.
    let out = rune(&["eval", r#"text("hi")"#]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let s = stdout(&out);
    assert!(s.contains("(# t)"), "expected text-chunk sigil/render, got: {s}");
    assert!(s.contains("hi"), "expected payload, got: {s}");
}

#[test]
fn eval_emit_act_renders_as_chunk() {
    // The act path: `emit` fires through the host and the chunk is shown.
    let out = rune(&["eval", r#"emit(text("live act"))"#]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stdout(&out).contains("live act"), "got: {}", stdout(&out));
}

#[test]
fn eval_col_yields_multiple_chunks() {
    let out = rune(&["eval", r#"col(text("a"), text("b"))"#]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let s = stdout(&out);
    let lines: Vec<_> = s.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected one chunk per element, got: {lines:?}");
}

#[test]
fn eval_parse_error_exits_nonzero() {
    let out = rune(&["eval", "6 *"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("parse error"), "got: {}", stderr(&out));
}

// ── fmt ──────────────────────────────────────────────────────────────────────

#[test]
fn fmt_converts_classic_to_pure_register() {
    let path = std::env::temp_dir().join("rune_cli_fmt_to_rune.rune");
    std::fs::write(&path, "6 * 7").unwrap();
    let out = rune(&["fmt", "--to=rune", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out).trim(), "(mul 6 7)");
}

// ── check ────────────────────────────────────────────────────────────────────

#[test]
fn check_passes_on_well_typed_source() {
    let path = std::env::temp_dir().join("rune_cli_check_ok.rune");
    std::fs::write(&path, "6 * 7").unwrap();
    let out = rune(&["check", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out).trim(), "ok");
}

#[test]
fn check_catches_mold_mismatch() {
    let path = std::env::temp_dir().join("rune_cli_check_bad.rune");
    std::fs::write(&path, "let x: @ud = \"hi\"\nx").unwrap();
    let out = rune(&["check", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert!(!out.status.success(), "expected nonzero exit on mold error");
    assert!(stderr(&out).contains("mold error"), "got: {}", stderr(&out));
}

// ── dispatch ─────────────────────────────────────────────────────────────────

#[test]
fn help_lists_every_command() {
    let out = rune(&["help"]);
    assert!(out.status.success());
    let s = stdout(&out);
    for cmd in ["run", "eval", "fmt", "check"] {
        assert!(s.contains(cmd), "help is missing `{cmd}`: {s}");
    }
}

#[test]
fn unknown_command_exits_nonzero() {
    let out = rune(&["frobnicate"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("unknown command"), "got: {}", stderr(&out));
}
