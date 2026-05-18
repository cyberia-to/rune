//! Event-driven execution for rune's hint/reactive tier.
//! When eval hits opcode 16 (call/hint), it returns a Yield rather than a value.
//! The caller drives the event loop, delivering events to resume continuations.

use crate::InterpError;
use rune_ast::Noun;

/// Result of a single evaluation step.
pub enum StepResult {
    /// Evaluation completed — this is the final value.
    Done(Noun),
    /// Evaluation yielded — waiting for an event matching this tag/selector.
    Yield {
        tag: u64,
        selector: Noun,
        continuation: Continuation,
    },
    /// Error during evaluation.
    Err(InterpError),
}

/// A suspended computation waiting for an event.
/// Stores everything needed to resume: the remaining formula and subject.
pub struct Continuation {
    pub subject: Noun,
    pub formula: Noun,  // the body formula to run once event arrives
}

/// Resume a continuation with an event noun.
pub fn resume(cont: Continuation, event: Noun) -> StepResult {
    // Inject event noun at axis 2 of the continuation subject (prepend to subject)
    let new_subject = Noun::cell(event, cont.subject);
    match crate::eval(&new_subject, &cont.formula) {
        Ok(n) => StepResult::Done(n),
        Err(e) => StepResult::Err(e),
    }
}

/// A mock event source for development and testing.
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

/// Run an expression to completion, driving the event loop with a MockEventSource.
/// Returns the final value or an error.
pub fn run_with_events(
    subject: &Noun,
    formula: &Noun,
    _events: &mut MockEventSource,
) -> Result<Noun, InterpError> {
    match crate::eval(subject, formula)? {
        n => Ok(n),  // completed without yielding (pure path)
        // Note: actual yield detection happens at opcode 16 in eval.
        // For now this just runs the pure path. Full yield/resume in M5b.
    }
}
