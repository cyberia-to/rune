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
    run_source(&src);
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
    let mut router = rune_compile::TierRouter::new(10);  // compile after 10 calls
    match router.eval(&subject, &formula) {
        Ok(result) => println!("{}", display_noun(&result)),
        Err(e)     => { eprintln!("eval error: {}", e.message); std::process::exit(1); }
    }
}

fn display_noun(noun: &rune_ast::Noun) -> String {
    match noun {
        rune_ast::Noun::Atom(n) => n.to_string(),
        rune_ast::Noun::Cell(h, t) => format!("[{} {}]", display_noun(h), display_noun(t)),
    }
}

fn print_help() {
    println!("rune — open computation for the cybergraph");
    println!();
    println!("commands:");
    println!("  run <file>    interpret a rune program");
    println!("  eval <expr>   evaluate a single expression");
    println!("  fmt <file>    format (--to=rust | --to=rune)  [M4]");
    println!("  check <file>  type-check without running       [M3]");
}
