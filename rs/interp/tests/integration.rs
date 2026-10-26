/// Integration tests: parse → lower → eval end-to-end.
///
/// The `run` helper takes a rune source string, parses it with the classic
/// register parser, lowers it to a Nox formula, evaluates against a minimal
/// subject, and returns the atom result as u64.  These tests exercise the
/// full pipeline for representative programs.
use rune_interp::eval;
use rune_lower::lower;
use rune_parse::parse;
use rune_subject::Subject;

/// Parse src, lower to Nox formula, eval against minimal subject, return u64.
fn run(src: &str) -> u64 {
    let expr = parse(src).unwrap_or_else(|e| panic!("parse error in {:?}: {}", src, e.message));
    let formula = lower(expr).unwrap_or_else(|e| panic!("lower error in {:?}: {}", src, e.message));
    let subject = Subject::minimal().to_noun();
    let result = eval(&subject, &formula)
        .unwrap_or_else(|e| panic!("eval error in {:?}: {}", src, e.message));
    match result {
        rune_ast::Noun::Atom(n) => n,
        _ => panic!("expected atom result for {:?}, got cell", src),
    }
}

/// Parse + lower + eval `src` against an explicit subject noun, return u64.
fn run_against(src: &str, subject: &rune_ast::Noun) -> u64 {
    let expr = parse(src).unwrap_or_else(|e| panic!("parse error in {:?}: {}", src, e.message));
    let formula = lower(expr).unwrap_or_else(|e| panic!("lower error in {:?}: {}", src, e.message));
    match eval(subject, &formula)
        .unwrap_or_else(|e| panic!("eval error in {:?}: {}", src, e.message))
    {
        rune_ast::Noun::Atom(n) => n,
        _ => panic!("expected atom result for {:?}", src),
    }
}

#[test]
fn literal_42() {
    assert_eq!(run("42"), 42);
}

// ── M2: subject resolution + cybermark scry ─────────────────────────────────

#[test]
fn tilde_self_reads_the_self_slot() {
    let mut s = Subject::minimal();
    s.self_neuron = rune_ast::Noun::Atom(99);
    assert_eq!(run_against("~self", &s.to_noun()), 99);
}

#[test]
fn address_scries_world_for_its_value() {
    // #cyber/truth threads through ~world and resolves to the stored value.
    let mut s = Subject::minimal();
    s.world = rune_subject::world(&[("cyber/truth", rune_ast::Noun::Atom(42))]);
    assert_eq!(run_against("#cyber/truth", &s.to_noun()), 42);
}

#[test]
fn address_miss_returns_zero() {
    let mut s = Subject::minimal();
    s.world = rune_subject::world(&[("cyber/truth", rune_ast::Noun::Atom(42))]);
    assert_eq!(run_against("#cyber/rune", &s.to_noun()), 0);
}

#[test]
fn add_5_plus_3() {
    assert_eq!(run("5 + 3"), 8);
}

#[test]
fn mul_6_times_7() {
    assert_eq!(run("6 * 7"), 42);
}

#[test]
fn let_x_equals_10() {
    // `let x = 10; x` — bind x to 10 in the extended subject, then resolve x
    assert_eq!(run("let x = 10; x"), 10);
}

#[test]
fn if_zero_eq_zero_takes_yes() {
    // `if 0 == 0 { 1 } else { 2 }` — cond is 0==0 → eq returns 0 (equal) → branch takes yes
    assert_eq!(run("if 0 == 0 { 1 } else { 2 }"), 1);
}

#[test]
fn gate_call_single_arg() {
    assert_eq!(run("let double = fn(x) { x * 2 }; double(5)"), 10);
}

#[test]
fn gate_call_two_args() {
    assert_eq!(run("let add_pair = fn(x, y) { x + y }; add_pair(3, 4)"), 7);
}

#[test]
fn closure_captures_outer_var() {
    // Let x = 10, let add_x = fn(y) { x + y }, add_x(5) → 15
    assert_eq!(run("let x = 10; let add_x = fn(y) { x + y }; add_x(5)"), 15);
}

#[test]
fn double_function() {
    assert_eq!(run("let double = fn(x) { x * 2 }; double(5)"), 10);
}

#[test]
fn fibonacci_10() {
    // Iterative fibonacci using trap+rebind+loop.
    // fib(10) = 55
    let src = r#"
let fib = fn(n) {
    let a = 0;
    let b = 1;
    let i = 0;
    loop {
        if i == n {
            a
        } else {
            let next_b = a + b;
            rebind a = b;
            rebind b = next_b;
            rebind i = i + 1;
            continue
        }
    }
};
fib(10)
"#;
    assert_eq!(run(src), 55);
}

#[test]
fn factorial_5() {
    // Iterative factorial using trap+rebind+loop.
    // factorial(5) = 120
    let src = r#"
let factorial = fn(n) {
    let acc = 1;
    let i = 1;
    loop {
        if i > n {
            acc
        } else {
            rebind acc = acc * i;
            rebind i = i + 1;
            continue
        }
    }
};
factorial(5)
"#;
    assert_eq!(run(src), 120);
}

#[test]
fn lte_operator() {
    // 3 <= 5 → true (0), 5 <= 3 → false (1), 3 <= 3 → true (0)
    assert_eq!(run("3 <= 5"), 0);
    assert_eq!(run("5 <= 3"), 1);
    assert_eq!(run("3 <= 3"), 0);
}

#[test]
fn gte_operator() {
    // 5 >= 3 → true (0), 3 >= 5 → false (1), 3 >= 3 → true (0)
    assert_eq!(run("5 >= 3"), 0);
    assert_eq!(run("3 >= 5"), 1);
    assert_eq!(run("3 >= 3"), 0);
}

#[test]
fn door_arm_call_no_args() {
    // impl { fn get(self) { 42 } }.get()
    let src = "let counter = impl { fn get(self) { 42 } }; counter.get()";
    assert_eq!(run(src), 42);
}

#[test]
fn door_arm_call_with_arg() {
    // impl { fn double(self, x) { x * 2 } }.double(5) → 10
    let src = "let calc = impl { fn double(self, x) { x * 2 } }; calc.double(5)";
    assert_eq!(run(src), 10);
}

#[test]
fn infix_precedence_and_logical_inequality() {
    assert_eq!(run("2 + 3 * 4"), 14);
    assert_eq!(run("(2 + 3) * 4"), 20);
    assert_eq!(run("5 - 3 - 1"), 1);
    assert_eq!(run("7 != 7"), 1);
    assert_eq!(run("7 != 8"), 0);
    assert!(parse("42 ignored").is_err());
    assert!(rune_parse::parse_bounded("let x = 1; x", 3).is_err());
    assert!(parse(&format!("{}0{}", "(".repeat(80), ")".repeat(80))).is_err());
}

#[test]
fn subject_slots_survive_nested_bindings_and_gates() {
    let mut s = Subject::minimal();
    s.mem = rune_ast::Noun::Atom(10);
    s.here = rune_ast::Noun::Atom(20);
    s.world = rune_subject::world(&[("cyber/truth", rune_ast::Noun::Atom(42))]);
    assert_eq!(
        run_against("let x = 2; let y = 3; ~mem + x * y", &s.to_noun()),
        16
    );
    assert_eq!(
        run_against("let f = fn(x) { ~here + x }; f(4)", &s.to_noun()),
        24
    );
    assert_eq!(run_against("let x = 1; #cyber/truth", &s.to_noun()), 42);
    let formula = lower(parse("let x = host(7); ~mem + x").unwrap()).unwrap();
    let mut machine =
        rune_interp::machine::Machine::new(s.to_noun(), formula, Default::default()).unwrap();
    assert!(matches!(
        machine.run(100).unwrap(),
        rune_interp::machine::Step::Act {
            tag: rune_ast::act::HOST,
            ..
        }
    ));
    machine.resume(rune_ast::Noun::Atom(32)).unwrap();
    assert!(matches!(
        machine.run(100).unwrap(),
        rune_interp::machine::Step::Done(rune_ast::Noun::Atom(42))
    ));
}
