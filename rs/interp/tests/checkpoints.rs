use rune_ast::{act, Noun};
use rune_interp::{
    codec,
    machine::{Limits, Machine, Step},
};
fn a(n: u64) -> Noun {
    Noun::Atom(n)
}
fn c(h: Noun, t: Noun) -> Noun {
    Noun::cell(h, t)
}
fn quote(n: u64) -> Noun {
    c(a(1), a(n))
}
fn emit(n: u64) -> Noun {
    c(a(16), c(c(a(act::EMIT), quote(n)), c(a(0), a(2))))
}

#[test]
fn nested_tool_continuation_survives_serialization_and_consumes_input_once() {
    let formula = c(a(5), c(quote(10), emit(7)));
    let limits = Limits::default();
    let mut machine = Machine::new(a(0), formula, limits).unwrap();
    assert!(matches!(
        machine.run(100).unwrap(),
        Step::Act {
            arguments: Noun::Atom(7),
            ..
        }
    ));
    let before = machine.used_steps();
    let bytes = machine.checkpoint().unwrap();
    let mut restored = Machine::restore(&bytes, limits).unwrap();
    assert_eq!(restored.used_steps(), before);
    assert!(matches!(restored.run(100).unwrap(), Step::Act { .. }));
    restored.resume(a(32)).unwrap();
    assert!(restored.resume(a(99)).is_err());
    assert!(matches!(
        restored.run(100).unwrap(),
        Step::Done(Noun::Atom(42))
    ));
}

#[test]
fn every_slice_can_restore_without_changing_pure_results() {
    for source in [
        "1 + 2 * 3",
        "let x = 5; x * x",
        "if 1 == 1 { 42 } else { 9 }",
        "[3, 4]",
    ] {
        let formula = rune_lower::lower(rune_parse::parse(source).unwrap()).unwrap();
        let expected = rune_interp::eval(&a(0), &formula).unwrap();
        let mut machine = Machine::new(a(0), formula, Limits::default()).unwrap();
        loop {
            match machine.run(1).unwrap() {
                Step::Done(actual) => {
                    assert_eq!(actual, expected, "{source}");
                    break;
                }
                Step::Yield => {
                    machine = Machine::restore(&machine.checkpoint().unwrap(), Limits::default())
                        .unwrap();
                }
                _ => panic!("pure formula requested an effect"),
            }
        }
    }
}

/// The node budget is only summed over the whole state at slice boundaries,
/// so each produced value is charged on its own: a value over the budget must
/// still be refused in the middle of a slice.
#[test]
fn the_node_budget_bites_inside_a_slice() {
    let limits = Limits {
        nodes: 24,
        ..Limits::default()
    };
    // a 13-node subject, doubled to 27 nodes by `[3 [0 1] [0 1]]`
    let subject = c(a(1), c(a(2), c(a(3), c(a(4), c(a(5), c(a(6), a(0)))))));
    let formula = c(a(3), c(c(a(0), a(1)), c(a(0), a(1))));
    let mut machine = Machine::new(subject, formula, limits).unwrap();
    assert!(machine.run(100).is_err());
}

/// The machine's ABI pins the interpreter's arithmetic, including operands
/// above the Goldilocks prime, which the unreduced borrow used to panic on.
#[test]
fn subtraction_above_the_prime_agrees_with_the_interpreter() {
    let formula = c(a(6), c(quote(0), quote(u64::MAX)));
    let expected = rune_interp::eval(&a(0), &formula).unwrap();
    let mut machine = Machine::new(a(0), formula, Limits::default()).unwrap();
    match machine.run(100).unwrap() {
        Step::Done(actual) => assert_eq!(actual, expected),
        other => panic!("pure formula did not complete: {other:?}"),
    }
}

#[test]
fn budget_cannot_reset_through_checkpoint_and_decode_is_bounded() {
    let limits = Limits {
        steps: 1,
        ..Limits::default()
    };
    let mut machine = Machine::new(a(0), c(a(5), c(quote(1), quote(2))), limits).unwrap();
    assert!(matches!(machine.run(1).unwrap(), Step::Yield));
    let mut restored = Machine::restore(&machine.checkpoint().unwrap(), limits).unwrap();
    assert!(restored.run(10).is_err());
    assert!(codec::decode(&[1; 5000], 100, 16).is_err());
    assert!(codec::decode(&[0; 10], 100, 16).is_err());
    assert!(codec::decode(&[7], 100, 16).is_err());
}

#[test]
fn completion_at_the_exact_limit_does_not_need_an_extra_slice() {
    let limits = Limits {
        steps: 1,
        ..Limits::default()
    };
    let mut machine = Machine::new(a(0), quote(42), limits).unwrap();
    assert!(matches!(
        machine.run(1).unwrap(),
        Step::Done(Noun::Atom(42))
    ));
    let mut machine = Machine::restore(&machine.checkpoint().unwrap(), limits).unwrap();
    assert!(matches!(
        machine.run(1).unwrap(),
        Step::Done(Noun::Atom(42))
    ));
    assert_eq!(machine.used_steps(), 1);
}
