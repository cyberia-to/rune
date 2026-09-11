//! Event-driven execution for rune's hint/reactive tier.
//!
//! When eval hits opcode 16 (call/hint), it can park and return a Yield
//! rather than evaluating synchronously.  The caller drives the event loop,
//! delivering events to resume continuations.
//!
//! Parking model (shallow, M5):
//!   eval_step() yields on the first opcode-16 encountered at the top level.
//!   The body formula becomes the continuation; the event is prepended to the
//!   subject on resume.  Deeply nested hints inside the body run synchronously
//!   on the resumed step; full deep parking is M5b (later).

use crate::{eval_with_authority, Host, InterpError};
use rune_ast::Noun;

/// Result of a single evaluation step.
pub enum StepResult {
    /// Evaluation completed — final value.
    Done(Noun),
    /// Evaluation parked — waiting for an event matching this tag/selector.
    Yield {
        tag: u64,
        selector: Noun,
        continuation: Continuation,
    },
    /// Error during evaluation.
    Err(InterpError),
}

/// A suspended computation: subject + body formula to resume with.
pub struct Continuation {
    pub subject: Noun,
    pub formula: Noun,
}

/// Evaluate one step.  Yields on a top-level opcode-16 hint; otherwise
/// delegates to the synchronous interpreter and wraps the result.
pub fn eval_step(subject: &Noun, formula: &Noun) -> StepResult {
    if let Noun::Cell(op, rest) = formula {
        if matches!(op.as_ref(), Noun::Atom(16)) {
            return step_hint(subject, rest);
        }
    }
    match crate::eval(subject, formula) {
        Ok(n) => StepResult::Done(n),
        Err(e) => StepResult::Err(e),
    }
}

/// Resume a parked continuation by injecting an event noun at axis 2 (head).
pub fn resume(cont: Continuation, event: Noun) -> StepResult {
    let new_subject = Noun::cell(event, cont.subject);
    eval_step(&new_subject, &cont.formula)
}

/// Run a formula to completion, driving the event loop with events from
/// `source`.  Returns the final value or an error.  If the program yields
/// more times than events are available, the last event (Atom 0) is reused.
pub fn run_with_events(
    subject: &Noun,
    formula: &Noun,
    source: &mut MockEventSource,
) -> Result<Noun, InterpError> {
    let mut step = eval_step(subject, formula);
    loop {
        match step {
            StepResult::Done(n) => return Ok(n),
            StepResult::Err(e) => return Err(e),
            StepResult::Yield { continuation, .. } => {
                let event = source.next().unwrap_or(Noun::Atom(0));
                step = resume(continuation, event);
            }
        }
    }
}

/// Drive a **cell** — the reactive entrypoint. A cell is a rune gate invoked
/// once per event, emitting chunks (via `host`) as it runs and returning its
/// next state. The driver threads that state forward and feeds the next event,
/// until the source is exhausted; the final state is returned.
///
/// Each invocation sees the subject `[event [state base]]`, so the gate reads
/// the event at axis 2, its state at axis 6, and the mounted base (caps, world,
/// …) at axis 7. Unlike hint parking, this needs no continuation capture — each
/// tick runs to completion, performing its acts — which is exactly the shape of
/// a surface that re-renders on input.
pub fn run_cell(
    base: &Noun,
    gate: &Noun,
    init_state: Noun,
    host: &mut dyn Host,
    source: &mut MockEventSource,
) -> Result<Noun, InterpError> {
    let mut state = init_state;
    let caps = crate::axis(base, rune_ast::act::CAPS_AXIS).unwrap_or(Noun::Atom(0));
    for event in source.by_ref() {
        let subject = Noun::cell(event, Noun::cell(state.clone(), base.clone()));
        state = eval_with_authority(&subject, gate, host, &caps)?;
    }
    Ok(state)
}

/// A mock event source for development and testing (queued nouns).
pub struct MockEventSource {
    events: std::collections::VecDeque<Noun>,
}

impl MockEventSource {
    pub fn new(events: Vec<Noun>) -> Self {
        MockEventSource {
            events: events.into(),
        }
    }
}

impl Iterator for MockEventSource {
    type Item = Noun;
    fn next(&mut self) -> Option<Noun> {
        self.events.pop_front()
    }
}

// ── helpers ────────────────────────────────────────────────────────────────────

fn step_hint(subject: &Noun, rest: &Noun) -> StepResult {
    // rest = [[tag selector_formula] body_formula]
    let (hint_meta, body) = match pair(rest) {
        Ok(p) => p,
        Err(e) => return StepResult::Err(e),
    };
    let (tag_noun, selector_f) = match pair(&hint_meta) {
        Ok(p) => p,
        Err(e) => return StepResult::Err(e),
    };
    let tag = match tag_noun {
        Noun::Atom(t) => t,
        _ => 0,
    };
    let selector = match crate::eval(subject, &selector_f) {
        Ok(s) => s,
        Err(e) => return StepResult::Err(e),
    };
    StepResult::Yield {
        tag,
        selector,
        continuation: Continuation {
            subject: subject.clone(),
            formula: body,
        },
    }
}

fn pair(noun: &Noun) -> Result<(Noun, Noun), InterpError> {
    match noun {
        Noun::Cell(h, t) => Ok((*h.clone(), *t.clone())),
        _ => Err(InterpError {
            message: "event: expected cell".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_ast::Noun::Atom;

    fn a(n: u64) -> Noun {
        Noun::Atom(n)
    }
    fn c(h: Noun, t: Noun) -> Noun {
        Noun::cell(h, t)
    }

    #[test]
    fn pure_formula_completes() {
        // [1 42] against Atom(0) → Done(42)
        let formula = c(a(1), a(42));
        let result = eval_step(&a(0), &formula);
        assert!(matches!(result, StepResult::Done(Atom(42))));
    }

    #[test]
    fn hint_yields_then_resumes() {
        // formula: [16 [99 [1 0]] [1 42]]
        //   tag=99 (direct atom), selector_f=[1 0]→0, body=[1 42]
        // Step 1: yields with tag=99, selector=0
        // Resume with event=Atom(7): new_subject = [7 original_subj]
        //   body=[1 42] evaluated against [7 0] = Done(42)
        let hint_meta = c(a(99), c(a(1), a(0)));
        let body = c(a(1), a(42));
        let formula = c(a(16), c(hint_meta, body));

        let step = eval_step(&a(0), &formula);
        match step {
            StepResult::Yield {
                tag, continuation, ..
            } => {
                assert_eq!(tag, 99);
                let resumed = resume(continuation, a(7));
                assert!(matches!(resumed, StepResult::Done(Atom(42))));
            }
            _ => panic!("expected Yield"),
        }
    }

    #[test]
    fn run_with_events_drives_loop() {
        // formula: [16 [[1 1] [1 0]] [1 55]]  (hint body returns 55)
        let hint_meta = c(c(a(1), a(1)), c(a(1), a(0)));
        let body = c(a(1), a(55));
        let formula = c(a(16), c(hint_meta, body));
        let mut src = MockEventSource::new(vec![a(0)]);
        let result = run_with_events(&a(0), &formula, &mut src).unwrap();
        assert_eq!(result, a(55));
    }

    #[test]
    fn event_injected_at_head_of_subject() {
        // hint body = [0 2]  (axis 2 = head of subject)
        // subject = Atom(0), event = Atom(99)
        // After resume: subject = [99 0], axis 2 = 99
        let hint_meta = c(c(a(1), a(0)), c(a(1), a(0)));
        let body = c(a(0), a(2)); // [0 2] = fetch axis 2
        let formula = c(a(16), c(hint_meta, body));
        let mut src = MockEventSource::new(vec![a(99)]);
        let result = run_with_events(&a(0), &formula, &mut src).unwrap();
        assert_eq!(result, a(99));
    }

    #[test]
    fn run_cell_reacts_per_event() {
        use rune_ast::act;
        struct Rec {
            emitted: Vec<Noun>,
        }
        impl Host for Rec {
            fn perform(
                &mut self,
                _act: u64,
                args: &Noun,
                _caps: &Noun,
            ) -> Result<Noun, InterpError> {
                self.emitted.push(args.clone());
                Ok(Noun::Atom(0))
            }
        }
        // cell gate: emit(event) then return state+1
        //   subject after emit = [result [event [state base]]] → state at axis 14
        //   [16 [EMIT [0 2]] [5 [0 14] [1 1]]]
        let gate = c(
            a(16),
            c(
                c(a(act::EMIT), c(a(0), a(2))),
                c(a(5), c(c(a(0), a(14)), c(a(1), a(1)))),
            ),
        );
        let mut host = Rec { emitted: vec![] };
        let mut src = MockEventSource::new(vec![a(10), a(20), a(30)]);
        let final_state = run_cell(&a(0), &gate, a(0), &mut host, &mut src).unwrap();
        assert_eq!(host.emitted, vec![a(10), a(20), a(30)]); // emitted each event
        assert_eq!(final_state, a(3)); // state advanced 0→3
    }
}
