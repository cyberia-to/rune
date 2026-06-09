//! The rune REPL — instant start, made interactive.
//!
//! Built on reedline (the nushell line editor, = the cyb terminal stack): line
//! editing, persistent history, Ctrl-R search, tab-completion, syntax
//! highlighting, and multi-line input. Each expression evaluates against a
//! subject that accumulates the `let` bindings from earlier lines — persistence
//! is textual (prior bindings are re-parsed as a prelude each line), so the
//! REPL holds no interpreter state and runs the same instant-start path every
//! time.

use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use reedline::{
    default_emacs_keybindings, ColumnarMenu, Emacs, FileBackedHistory, KeyCode, KeyModifiers,
    MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
};

use crate::repl_line::{Binds, RuneCompleter, RuneHighlighter, RunePrompt, RuneValidator};
use crate::{banner, display_noun, render, CollectHost};

const GRAY: &str = "\x1b[90m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn paint(color: bool, style: &str, s: &str) -> String {
    if color {
        format!("{style}{s}{RESET}")
    } else {
        s.to_string()
    }
}

/// Start the REPL. On a terminal this is the full reedline experience; off one
/// (piped input) it falls back to a plain line processor so `… | rune` works as
/// a filter. `color` gates ANSI styling on output.
pub fn run(color: bool) {
    let binds: Binds = Arc::new(Mutex::new(Vec::new()));
    if std::io::stdin().is_terminal() {
        run_interactive(binds, color);
    } else {
        run_plain(binds, color);
    }
}

/// The reedline-driven interactive loop.
fn run_interactive(binds: Binds, color: bool) {
    let mut editor = build_editor(&binds);
    let prompt = RunePrompt;

    print!("{}", banner::banner(color));
    println!();
    println!("{}", paint(color, GRAY, "  :help · Tab completes · ↑ history · Ctrl-R search · Ctrl-D leaves"));
    println!();

    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                if handle_line(&binds, &line, color) {
                    break; // :quit
                }
            }
            Ok(Signal::CtrlC) => continue, // cancel the current line
            Ok(Signal::CtrlD) => break,
            Err(_) => break,
        }
    }
}

/// The plain line loop for piped (non-tty) input.
fn run_plain(binds: Binds, color: bool) {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                if handle_line(&binds, &l, color) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

/// Dispatch one input line. Returns true to quit the REPL.
fn handle_line(binds: &Binds, line: &str, color: bool) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return false;
    }
    if line.starts_with(':') {
        return meta(binds, line, color);
    }
    if line.starts_with("let ") {
        accept_binding(binds, line, color);
    } else {
        eval_line(binds, line, color);
    }
    false
}

/// Assemble the reedline editor with completion, highlighting, validation,
/// history, and a Tab-driven completion menu.
fn build_editor(binds: &Binds) -> Reedline {
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    let menu = ColumnarMenu::default().with_name("completion_menu");

    let mut editor = Reedline::create()
        .with_completer(Box::new(RuneCompleter { binds: binds.clone() }))
        .with_highlighter(Box::new(RuneHighlighter { binds: binds.clone() }))
        .with_validator(Box::new(RuneValidator))
        .with_menu(ReedlineMenu::EngineCompleter(Box::new(menu)))
        .with_edit_mode(Box::new(Emacs::new(keybindings)));

    if let Some(h) = history() {
        editor = editor.with_history(h);
    }
    editor
}

/// A file-backed history at `~/.config/rune/history`, if HOME is set.
fn history() -> Option<Box<FileBackedHistory>> {
    let home = std::env::var("HOME").ok()?;
    let dir = std::path::Path::new(&home).join(".config").join("rune");
    std::fs::create_dir_all(&dir).ok()?;
    FileBackedHistory::with_file(2000, dir.join("history")).ok().map(Box::new)
}

// ── meta-commands ────────────────────────────────────────────────────────────

/// Handle a `:` meta-command. Returns true to quit the REPL.
fn meta(binds: &Binds, line: &str, color: bool) -> bool {
    let (cmd, arg) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let arg = arg.trim();
    match cmd {
        ":quit" | ":q" | ":exit" => return true,
        ":help" | ":h" => print!("{}", banner::repl_help(color)),
        ":env" | ":subject" => print_env(binds, color),
        ":reset" | ":clear" => {
            binds.lock().unwrap().clear();
            println!("{}", paint(color, GRAY, "bindings cleared"));
        }
        ":nox" => cmd_nox(binds, arg, color),
        ":type" | ":mold" => cmd_type(arg, color),
        ":ast" => cmd_ast(arg, color),
        ":load" => cmd_load(binds, arg, color),
        other => println!("{}", paint(color, RED, &format!("unknown command: {other}  (:help)"))),
    }
    false
}

/// `:nox <expr>` — show the Nox formula the expression lowers to.
fn cmd_nox(binds: &Binds, expr: &str, color: bool) {
    match lower_only(&prelude(binds, expr)) {
        Ok(formula) => println!("{}", paint(color, GRAY, &display_noun(&formula))),
        Err(e) => println!("{}", paint(color, RED, &format!("✗ {e}"))),
    }
}

/// `:ast <expr>` — show the parsed AST.
fn cmd_ast(expr: &str, color: bool) {
    match rune_parse::parse(expr) {
        Ok(ast) => println!("{}", paint(color, GRAY, &rune_parse::format_expr(&ast))),
        Err(e) => println!("{}", paint(color, RED, &format!("✗ parse: {}", e.message))),
    }
}

/// `:type <expr>` — mold-check the expression.
fn cmd_type(expr: &str, color: bool) {
    match rune_parse::parse(expr) {
        Ok(ast) => {
            let errors = rune_mold::MoldChecker::check(&ast);
            if errors.is_empty() {
                println!("{}", paint(color, GREEN, "ok"));
            } else {
                for e in &errors {
                    println!("{}", paint(color, RED, &format!("✗ {}", e.message)));
                }
            }
        }
        Err(e) => println!("{}", paint(color, RED, &format!("✗ parse: {}", e.message))),
    }
}

/// `:load <file>` — run each line of a file into the session.
fn cmd_load(binds: &Binds, path: &str, color: bool) {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            println!("{}", paint(color, RED, &format!("✗ {e}")));
            return;
        }
    };
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("::")) {
        if line.starts_with("let ") {
            accept_binding(binds, line, color);
        } else {
            eval_line(binds, line, color);
        }
    }
}

// ── evaluation ───────────────────────────────────────────────────────────────

fn print_env(binds: &Binds, color: bool) {
    let binds = binds.lock().unwrap();
    if binds.is_empty() {
        println!("{}", paint(color, GRAY, "(no bindings)"));
        return;
    }
    for b in binds.iter() {
        println!("{}", paint(color, GRAY, &format!("  {b}")));
    }
}

/// Compose the prelude of accumulated bindings with a trailing expression.
fn prelude(binds: &Binds, expr: &str) -> String {
    let binds = binds.lock().unwrap();
    if binds.is_empty() {
        expr.to_string()
    } else {
        format!("{}\n{}", binds.join("\n"), expr)
    }
}

/// Validate a `let` line (eval the prelude with a dummy body) and accept it.
fn accept_binding(binds: &Binds, line: &str, color: bool) {
    let mut trial = binds.lock().unwrap().clone();
    trial.push(line.to_string());
    let program = format!("{}\n0", trial.join("\n"));
    match eval_program(&program) {
        Ok(_) => {
            binds.lock().unwrap().push(line.to_string());
            println!("{}", paint(color, GRAY, line));
        }
        Err(e) => println!("{}", paint(color, RED, &format!("✗ {e}"))),
    }
}

/// Evaluate an expression against the accumulated bindings.
fn eval_line(binds: &Binds, line: &str, color: bool) {
    let program = prelude(binds, line);
    let t0 = Instant::now();
    match eval_program(&program) {
        Ok((chunks, noun)) => {
            let us = t0.elapsed().as_micros();
            if chunks.is_empty() {
                let val = paint(color, GREEN, &format!("= {}", display_noun(&noun)));
                println!("{val}{}", paint(color, GRAY, &format!("  [{us}µs]")));
            } else {
                println!("{}", render::ansi_chunks(&chunks, color));
                if color {
                    println!("{GRAY}[{us}µs]{RESET}");
                }
            }
        }
        Err(e) => println!("{}", paint(color, RED, &format!("✗ {e}"))),
    }
}

/// Parse → lower a program to its Nox formula (no evaluation).
fn lower_only(src: &str) -> Result<rune_ast::Noun, String> {
    let ast = rune_parse::parse(src).map_err(|e| format!("parse: {}", e.message))?;
    rune_lower::lower(ast).map_err(|e| format!("lower: {}", e.message))
}

/// Parse → lower → eval with a collecting host. Returns (chunks, result noun) or
/// a human error string. Never panics — the REPL must survive bad input.
fn eval_program(src: &str) -> Result<(Vec<tape::Chunk>, rune_ast::Noun), String> {
    let formula = lower_only(src)?;
    let subject = crate::subject();
    let mut host = CollectHost { emitted: Vec::new() };
    let result = rune_interp::eval_with_host(&subject, &formula, &mut host)
        .map_err(|e| format!("eval: {}", e.message))?;
    let mut chunks = Vec::new();
    for n in &host.emitted {
        chunks.extend(rune_prysm::noun_to_chunks(n));
    }
    chunks.extend(rune_prysm::noun_to_chunks(&result));
    Ok((chunks, result))
}
