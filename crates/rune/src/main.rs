/// rune CLI
///
/// Commands:
///   rune run <file>              parse, lower, and interpret a rune program
///   rune eval <expr>             evaluate a single expression
///   rune fmt [--to=rust|--to=rune] <file>   format source
///   rune check <file>            type-check without running
use rune_parse_pure;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str);
    let rest = if args.len() > 2 { &args[2..] } else { &[] };

    match cmd {
        Some("run")   => cmd_run(rest),
        Some("eval")  => cmd_eval(rest),
        Some("fmt")   => cmd_fmt(rest),
        Some("check") => cmd_check(rest),
        Some("help") | None => print_help(),
        Some(c) => { eprintln!("unknown command: {c}"); std::process::exit(1); }
    }
}

fn cmd_run(args: &[String]) {
    let path = args.first().unwrap_or_else(|| { eprintln!("usage: rune run <file>"); std::process::exit(1); });
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1); });
    run_source_event(&src);
}

fn cmd_eval(args: &[String]) {
    let expr = args.first().unwrap_or_else(|| { eprintln!("usage: rune eval <expr>"); std::process::exit(1); });
    run_source(expr);
}

fn cmd_fmt(args: &[String]) {
    let (target, file) = if args.first().map(String::as_str) == Some("--to=rune") {
        ("rune", args.get(1))
    } else if args.first().map(String::as_str) == Some("--to=rust") {
        ("rust", args.get(1))
    } else {
        ("rust", args.first())
    };
    let path = file.unwrap_or_else(|| {
        eprintln!("usage: rune fmt [--to=rust|--to=rune] <file>");
        std::process::exit(1);
    });
    let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    let ast = match rune_parse::parse(&src) {
        Ok(a)  => a,
        Err(e) => { eprintln!("parse error: {}", e.message); std::process::exit(1); }
    };
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

    let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let ast = match rune_parse::parse(&src) {
        Ok(a)  => a,
        Err(e) => { eprintln!("parse error: {}", e.message); std::process::exit(1); }
    };

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

/// Event-loop runner for `rune run`.  Drives the hint-parking event loop,
/// reading one noun per line from stdin when the program yields.
fn run_source_event(src: &str) {
    use rune_interp::event::{eval_step, resume, StepResult};

    let ast = match rune_parse::parse(src) {
        Ok(a)  => a,
        Err(e) => { eprintln!("parse error: {}", e.message); std::process::exit(1); }
    };
    let formula = match rune_lower::lower(ast) {
        Ok(n)  => n,
        Err(e) => { eprintln!("lower error: {}", e.message); std::process::exit(1); }
    };
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

/// Synchronous runner for `rune eval` (one-shot, no event loop).
fn run_source(src: &str) {
    let ast = match rune_parse::parse(src) {
        Ok(a)  => a,
        Err(e) => { eprintln!("parse error: {}", e.message); std::process::exit(1); }
    };
    let formula = match rune_lower::lower(ast) {
        Ok(n)  => n,
        Err(e) => { eprintln!("lower error: {}", e.message); std::process::exit(1); }
    };
    let subject = rune_subject::Subject::minimal().to_noun();
    let mut router = rune_compile::TierRouter::new(10);
    match router.eval(&subject, &formula) {
        Ok(result) => println!("{}", display_noun(&result)),
        Err(e)     => { eprintln!("eval error: {}", e.message); std::process::exit(1); }
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

fn display_noun(noun: &rune_ast::Noun) -> String {
    match noun {
        rune_ast::Noun::Atom(n) => n.to_string(),
        rune_ast::Noun::Cell(h, t) => format!("[{} {}]", display_noun(h), display_noun(t)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_ast::Noun::{Atom, Cell};

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

fn print_help() {
    println!("rune — open computation for the cybergraph");
    println!();
    println!("commands:");
    println!("  run <file>    interpret a rune program (hints yield; send noun per line)");
    println!("  eval <expr>   evaluate a single expression");
    println!("  fmt <file>    format (--to=rust | --to=rune)  [M4]");
    println!("  check <file>  type-check without running       [M3]");
}
