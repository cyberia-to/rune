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

use crate::InterpError;
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

/// A mock event source for development and testing (queued nouns).
pub struct MockEventSource {
    events: std::collections::VecDeque<Noun>,
}

impl MockEventSource {
    pub fn new(events: Vec<Noun>) -> Self {
        MockEventSource { events: events.into() }
    }
    pub fn next(&mut self) -> Option<Noun> {
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
        continuation: Continuation { subject: subject.clone(), formula: body },
    }
}

fn pair(noun: &Noun) -> Result<(Noun, Noun), InterpError> {
    match noun {
        Noun::Cell(h, t) => Ok((*h.clone(), *t.clone())),
        _ => Err(InterpError { message: "event: expected cell".into() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune_ast::Noun::Atom;

    fn a(n: u64) -> Noun { Noun::Atom(n) }
    fn c(h: Noun, t: Noun) -> Noun { Noun::cell(h, t) }

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
            StepResult::Yield { tag, continuation, .. } => {
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
}
