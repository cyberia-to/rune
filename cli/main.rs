//! rune CLI
//!
//! Commands:
//!   rune                         start the REPL
//!   rune run <file>              interpret a program (hints yield; send a noun per line)
//!   rune eval <expr>             evaluate one expression — acts render as prysm chunks
//!   rune fmt [--to=rust|--to=rune] <file>   format source between registers
//!   rune check <file>            type-check (mold) without running

mod banner;
mod render;
mod repl;

use std::io::IsTerminal;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str);
    let rest = if args.len() > 2 { &args[2..] } else { &[] };
    let color = std::io::stdout().is_terminal();

    match cmd {
        None | Some("repl") => repl::run(color),
        Some("run")   => cmd_run(rest),
        Some("eval")  => cmd_eval(rest, color),
        Some("fmt")   => cmd_fmt(rest),
        Some("check") => cmd_check(rest),
        Some("help") | Some("-h") | Some("--help") => print!("{}", banner::help(color)),
        Some("--version") | Some("-V") => println!("rune {}", env!("CARGO_PKG_VERSION")),
        Some(c) => { eprintln!("unknown command: {c}"); std::process::exit(1); }
    }
}

fn cmd_run(args: &[String]) {
    let path = args.first().unwrap_or_else(|| { eprintln!("usage: rune run <file>"); std::process::exit(1); });
    let src = read_file(path);
    run_source_event(&src);
}

/// `rune eval <expr>` — evaluate a single expression.
///
/// Host-aware, so it doubles as the rune↔prysm demo surface. `emit` and the
/// other acts fire as the program runs and are shown as the tape chunks prysm
/// would render (the same chunks feed `dispatch` over wgpu/ansi/html). A
/// program whose value is itself a renderable noun — `text("hi")`, `col(...)` —
/// is decoded the same way. Anything else (arithmetic, plain cells) prints as a
/// noun.
fn cmd_eval(args: &[String], color: bool) {
    let expr = args.first().unwrap_or_else(|| { eprintln!("usage: rune eval <expr>"); std::process::exit(1); });
    let formula = lower_ast(parse_ast(expr));
    let subject = rune_subject::Subject::minimal().to_noun();

    // Collect acts as they fire; decode the final value too. This is the act
    // path end-to-end without the GUI — see `cyb/root/ward.md` for the seam.
    let mut host = CollectHost { emitted: Vec::new() };
    let result = match rune_interp::eval_with_host(&subject, &formula, &mut host) {
        Ok(n)  => n,
        Err(e) => { eprintln!("eval error: {}", e.message); std::process::exit(1); }
    };

    let mut chunks = Vec::new();
    for n in &host.emitted { chunks.extend(rune_prysm::noun_to_chunks(n)); }
    chunks.extend(rune_prysm::noun_to_chunks(&result));

    if chunks.is_empty() {
        println!("{}", display_noun(&result));
    } else {
        println!("{}", render::ansi_chunks(&chunks, color));
    }
}

fn cmd_fmt(args: &[String]) {
    let (target, file) = match args.first().map(String::as_str) {
        Some("--to=rune") => ("rune", args.get(1)),
        Some("--to=rust") => ("rust", args.get(1)),
        _                 => ("rust", args.first()),
    };
    let path = file.unwrap_or_else(|| {
        eprintln!("usage: rune fmt [--to=rust|--to=rune] <file>");
        std::process::exit(1);
    });
    let src = read_file(path);
    let ast = parse_ast(&src);
    if target == "rune" {
        println!("{}", rune_parse_pure::format_expr(&ast));
    } else {
        println!("{}", rune_parse::format_expr(&ast));
    }
}

fn cmd_check(args: &[String]) {
    let path = args.first().unwrap_or_else(|| {
        eprintln!("usage: rune check <file>");
        std::process::exit(1);
    });
    let src = read_file(path);
    let ast = parse_ast(&src);
    let errors = rune_mold::MoldChecker::check(&ast);
    if errors.is_empty() {
        println!("ok");
    } else {
        for e in &errors {
            eprintln!("{}", e.message);
        }
        std::process::exit(1);
    }
}

// ── shared front-end ─────────────────────────────────────────────────────────

/// Read a source file or exit with its IO error.
fn read_file(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1); })
}

/// Parse source to an AST or exit with the parse error.
fn parse_ast(src: &str) -> rune_ast::Expr {
    match rune_parse::parse(src) {
        Ok(a)  => a,
        Err(e) => { eprintln!("parse error: {}", e.message); std::process::exit(1); }
    }
}

/// Lower an AST to a Nox formula or exit with the lower error.
fn lower_ast(ast: rune_ast::Expr) -> rune_ast::Noun {
    match rune_lower::lower(ast) {
        Ok(n)  => n,
        Err(e) => { eprintln!("lower error: {}", e.message); std::process::exit(1); }
    }
}

/// Event-loop runner for `rune run`. Drives the hint-parking event loop,
/// reading one noun per line from stdin when the program yields.
fn run_source_event(src: &str) {
    use rune_interp::event::{eval_step, resume, StepResult};

    let formula = lower_ast(parse_ast(src));
    let subject = rune_subject::Subject::minimal().to_noun();

    let mut step = eval_step(&subject, &formula);
    loop {
        match step {
            StepResult::Done(n) => { println!("{}", display_noun(&n)); return; }
            StepResult::Err(e) => { eprintln!("eval error: {}", e.message); std::process::exit(1); }
            StepResult::Yield { tag, selector, continuation } => {
                eprintln!("yield tag={tag} selector={}", display_noun(&selector));
                let event = read_event_noun();
                step = resume(continuation, event);
            }
        }
    }
}

/// A host that collects `emit` act payloads — the CLI's stand-in for a surface.
pub(crate) struct CollectHost {
    pub(crate) emitted: Vec<rune_ast::Noun>,
}

impl rune_interp::Host for CollectHost {
    fn perform(
        &mut self,
        act: u64,
        args: &rune_ast::Noun,
        _caps: &rune_ast::Noun,
    ) -> Result<rune_ast::Noun, rune_interp::InterpError> {
        if act == rune_ast::act::EMIT {
            self.emitted.push(args.clone());
        }
        Ok(rune_ast::Noun::Atom(0))
    }
}

/// Read one line from stdin and parse it as a noun literal.
/// Empty line or EOF → Atom(0).
fn read_event_noun() -> rune_ast::Noun {
    use std::io::BufRead;
    let line = std::io::stdin().lock().lines().next()
        .and_then(|r| r.ok())
        .unwrap_or_default();
    parse_noun_literal(line.trim()).unwrap_or(rune_ast::Noun::Atom(0))
}

/// Parse a noun literal in display format: a decimal atom or `[head tail]`.
fn parse_noun_literal(s: &str) -> Option<rune_ast::Noun> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u64>() {
        return Some(rune_ast::Noun::Atom(n));
    }
    if s.starts_with('[') && s.ends_with(']') {
        let inner = s[1..s.len() - 1].trim();
        let (head_s, tail_s) = split_noun_pair(inner)?;
        let head = parse_noun_literal(head_s)?;
        let tail = parse_noun_literal(tail_s)?;
        return Some(rune_ast::Noun::cell(head, tail));
    }
    None
}

/// Find the split point between head and tail in `h t` (respecting brackets).
fn split_noun_pair(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            ' ' if depth == 0 => {
                let tail = s[i + 1..].trim_start();
                return Some((&s[..i], tail));
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn display_noun(noun: &rune_ast::Noun) -> String {
    match noun {
        rune_ast::Noun::Atom(n) => n.to_string(),
        rune_ast::Noun::Cell(h, t) => format!("[{} {}]", display_noun(h), display_noun(t)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(n: u64) -> rune_ast::Noun { rune_ast::Noun::Atom(n) }
    fn c(h: rune_ast::Noun, t: rune_ast::Noun) -> rune_ast::Noun { rune_ast::Noun::cell(h, t) }

    #[test]
    fn parse_atom() {
        assert_eq!(parse_noun_literal("42"), Some(a(42)));
        assert_eq!(parse_noun_literal("0"), Some(a(0)));
    }

    #[test]
    fn parse_flat_cell() {
        assert_eq!(parse_noun_literal("[1 2]"), Some(c(a(1), a(2))));
    }

    #[test]
    fn parse_nested_cell() {
        assert_eq!(parse_noun_literal("[1 [2 3]]"), Some(c(a(1), c(a(2), a(3)))));
        assert_eq!(parse_noun_literal("[[1 2] 3]"), Some(c(c(a(1), a(2)), a(3))));
    }

    #[test]
    fn parse_invalid_returns_none() {
        assert_eq!(parse_noun_literal("abc"), None);
        assert_eq!(parse_noun_literal("[1]"), None);
    }

    #[test]
    fn display_roundtrip() {
        let noun = c(a(42), c(a(1), a(2)));
        let s = display_noun(&noun);
        assert_eq!(parse_noun_literal(&s), Some(noun));
    }
}
