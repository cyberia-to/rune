use rune_ast::{act, Noun};
use rune_interp::event::{run_cell, MockEventSource};
use rune_interp::{eval_with_authority, eval_with_host, Host, InterpError};
use rune_subject::Subject;

fn a(n: u64) -> Noun {
    Noun::Atom(n)
}
fn c(h: Noun, t: Noun) -> Noun {
    Noun::cell(h, t)
}
fn emit(body: Noun) -> Noun {
    c(a(16), c(c(a(act::EMIT), c(a(1), a(7))), body))
}
struct Recorder(Vec<Noun>);
impl Host for Recorder {
    fn perform(&mut self, _: u64, _: &Noun, caps: &Noun) -> Result<Noun, InterpError> {
        self.0.push(caps.clone());
        Ok(a(0))
    }
}
fn subject() -> Noun {
    let mut subject = Subject::minimal();
    subject.now = a(11);
    subject.here = a(12);
    subject.caps = a(13);
    subject.to_noun()
}

#[test]
fn sequential_nested_and_reactive_acts_keep_host_authority() {
    let gate = emit(emit(c(a(1), a(42))));
    let mut host = Recorder(Vec::new());
    eval_with_host(&subject(), &gate, &mut host).unwrap();
    assert_eq!(host.0, vec![a(13), a(13)]);
    host.0.clear();
    let mut events = MockEventSource::new(vec![a(1), a(2)]);
    run_cell(&subject(), &gate, a(0), &mut host, &mut events).unwrap();
    assert_eq!(host.0, vec![a(13); 4]);
    host.0.clear();
    let compose = c(a(2), c(c(a(1), a(999)), c(a(1), gate)));
    eval_with_authority(&subject(), &compose, &mut host, &a(123)).unwrap();
    assert_eq!(host.0, vec![a(123), a(123)]);
}
