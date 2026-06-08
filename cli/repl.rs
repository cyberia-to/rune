//! The rune REPL — instant start, made interactive.
//!
//! Each line evaluates against a subject that accumulates the `let` bindings
//! from earlier lines. Persistence is textual: prior bindings are re-parsed as
//! a prelude on every line, so the REPL holds no interpreter state — the same
//! instant-start path (parse → lower → reduce) runs each time.

use std::io::{self, Write};
use std::time::Instant;

use crate::{banner, display_noun, render, CollectHost};

const GRAY: &str = "\x1b[90m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";
const RESET: &str = "\x1b[0m";

fn paint(color: bool, style: &str, s: &str) -> String {
    if color {
        format!("{style}{s}{RESET}")
    } else {
        s.to_string()
    }
}

/// Start the REPL loop. `color` gates ANSI styling.
pub fn run(color: bool) {
    print!("{}", banner::banner(color));
    println!();
    println!("{}", paint(color, GRAY, "  :help for commands · Ctrl-D to leave"));
    println!();

    let mut binds: Vec<String> = Vec::new();
    let stdin = io::stdin();

    loop {
        print!("{}", paint(color, MAGENTA, "rune"));
        print!("{}", paint(color, GRAY, " ▸ "));
        let _ = io::stdout().flush();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            } // EOF (Ctrl-D)
            Ok(_) => {}
            Err(_) => break,
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            ":quit" | ":q" | ":exit" => break,
            ":help" | ":h" => {
                print!("{}", banner::repl_help(color));
                continue;
            }
            ":reset" | ":clear" => {
                binds.clear();
                println!("{}", paint(color, GRAY, "bindings cleared"));
                continue;
            }
            ":env" | ":subject" => {
                print_env(&binds, color);
                continue;
            }
            _ => {}
        }

        if line.starts_with("let ") {
            accept_binding(&mut binds, line, color);
        } else {
            eval_line(&binds, line, color);
        }
    }
}

/// Print the accumulated bindings.
fn print_env(binds: &[String], color: bool) {
    if binds.is_empty() {
        println!("{}", paint(color, GRAY, "(no bindings)"));
        return;
    }
    for b in binds {
        println!("{}", paint(color, GRAY, &format!("  {b}")));
    }
}

/// Validate a `let` line by evaluating the prelude with a dummy body; accept it
/// into the binding set only if it compiles and runs.
fn accept_binding(binds: &mut Vec<String>, line: &str, color: bool) {
    let mut trial = binds.clone();
    trial.push(line.to_string());
    let program = format!("{}\n0", trial.join("\n"));
    match eval_program(&program) {
        Ok(_) => {
            binds.push(line.to_string());
            println!("{}", paint(color, GRAY, line));
        }
        Err(e) => println!("{}", paint(color, RED, &format!("✗ {e}"))),
    }
}

/// Evaluate an expression line against the accumulated bindings.
fn eval_line(binds: &[String], line: &str, color: bool) {
    let program = if binds.is_empty() {
        line.to_string()
    } else {
        format!("{}\n{}", binds.join("\n"), line)
    };
    let t0 = Instant::now();
    match eval_program(&program) {
        Ok((chunks, noun)) => {
            let us = t0.elapsed().as_micros();
            if chunks.is_empty() {
                let val = paint(color, GREEN, &format!("= {}", display_noun(&noun)));
                let time = paint(color, GRAY, &format!("  [{us}µs]"));
                println!("{val}{time}");
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

/// Parse → lower → eval with a collecting host. Returns (chunks, result noun)
/// or a human error string. Never panics or exits — the REPL must survive bad
/// input.
fn eval_program(src: &str) -> Result<(Vec<tape::Chunk>, rune_ast::Noun), String> {
    let ast = rune_parse::parse(src).map_err(|e| format!("parse: {}", e.message))?;
    let formula = rune_lower::lower(ast).map_err(|e| format!("lower: {}", e.message))?;
    let subject = rune_subject::Subject::minimal().to_noun();
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
