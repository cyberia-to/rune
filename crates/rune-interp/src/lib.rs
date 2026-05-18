/// Tree-walking Nox interpreter — the instant-start execution path.
/// Implements all 12 Nox reduction patterns directly over Noun trees.
/// No external dependencies; no build phase; executes the moment parsing is done.
///
/// Nox reduction rules:
///   /[1 a]          = a                              (slot self)
///   /[2 [a b]]      = a                              (slot head)
///   /[3 [a b]]      = b                              (slot tail)
///   /[(2n) a]       = /[2 /[n a]]                   (slot even)
///   /[(2n+1) a]     = /[3 /[n a]]                   (slot odd)
///
///   *[a [b c] d]    = [*[a b c] *[a d]]             (distribute)
///   *[a 0 b]        = /[b a]                         (slot)
///   *[a 1 b]        = b                              (literal)
///   *[a 2 b c]      = *[*[a b] *[a c]]              (eval)
///   *[a 3 b]        = ?(*[a b])                     (cell test)
///   *[a 4 b]        = .+(*[a b])                    (increment)
///   *[a 5 b c]      = =(*[a b] *[a c])              (equality)
///   *[a 6 b c d]    = *[a 2 [0 1] 2 [1 c d] [1 0] 2 [1 2 3] [1 0] 4 4 b]  (if)
///   *[a 7 b c]      = *[*[a b] c]                   (compose)
///   *[a 8 b c]      = *[[*[a b] a] c]               (push)
///   *[a 9 b c]      = *[*[a c] 2 [0 1] 0 b]        (call)
///   *[a 10 [b c] d] = *[a d]  (hint, c evaluated for side effect)
///   *[a 11 b]       = !                              (scry — runtime hook required)
use rune_ast::Noun;

#[derive(Debug, Clone)]
pub struct InterpError {
    pub message: String,
}

impl std::fmt::Display for InterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Evaluate a Nox formula against a subject noun: `*[subject formula]`
pub fn eval(subject: &Noun, formula: &Noun) -> Result<Noun, InterpError> {
    // Tail-call loop to avoid stack overflow on deep recursion
    let mut subj = subject.clone();
    let mut form = formula.clone();
    loop {
        match &form {
            // Distribute: [formula formula] applied to subject
            Noun::Cell(head, tail) if matches!(head.as_ref(), Noun::Cell(..)) => {
                let h = eval(&subj, head)?;
                let t = eval(&subj, tail)?;
                return Ok(Noun::cell(h, t));
            }

            Noun::Cell(op, rest) => match op.as_ref() {
                // [0 axis] — slot
                Noun::Atom(0) => return slot(&subj, rest),

                // [1 value] — literal
                Noun::Atom(1) => return Ok(*rest.clone()),

                // [2 subject-formula formula-formula] — eval
                Noun::Atom(2) => {
                    let (sf, ff) = cell2(rest)?;
                    let new_subj = eval(&subj, &sf)?;
                    let new_form = eval(&subj, &ff)?;
                    subj = new_subj;
                    form = new_form;
                    continue; // tail call
                }

                // [3 formula] — cell test: 0=yes cell, 1=atom
                Noun::Atom(3) => {
                    let r = eval(&subj, rest)?;
                    return Ok(Noun::Atom(if matches!(r, Noun::Cell(..)) { 0 } else { 1 }));
                }

                // [4 formula] — increment
                Noun::Atom(4) => {
                    let r = eval(&subj, rest)?;
                    return match r {
                        Noun::Atom(n) => Ok(Noun::Atom(n.saturating_add(1))),
                        _ => Err(err("nox-4: increment requires atom")),
                    };
                }

                // [5 a b] — equality: 0=equal, 1=not equal
                Noun::Atom(5) => {
                    let (af, bf) = cell2(rest)?;
                    let a = eval(&subj, &af)?;
                    let b = eval(&subj, &bf)?;
                    return Ok(Noun::Atom(if a == b { 0 } else { 1 }));
                }

                // [6 cond yes no] — if-then-else
                Noun::Atom(6) => {
                    let (cf, rest2) = cell2(rest)?;
                    let (yf, nf) = cell2(&rest2)?;
                    let cond = eval(&subj, &cf)?;
                    form = match cond {
                        Noun::Atom(0) => yf,
                        Noun::Atom(1) => nf,
                        _ => return Err(err("nox-6: condition must be 0 or 1")),
                    };
                    continue; // tail call
                }

                // [7 a b] — compose: eval b against result of a
                Noun::Atom(7) => {
                    let (af, bf) = cell2(rest)?;
                    subj = eval(&subj, &af)?;
                    form = bf;
                    continue; // tail call
                }

                // [8 a b] — push: eval b against [eval(a) subject]
                Noun::Atom(8) => {
                    let (af, bf) = cell2(rest)?;
                    let head = eval(&subj, &af)?;
                    subj = Noun::cell(head, subj);
                    form = bf;
                    continue; // tail call
                }

                // [9 axis core] — call arm at axis in core
                Noun::Atom(9) => {
                    let (axis, core_f) = cell2(rest)?;
                    let core = eval(&subj, &core_f)?;
                    let arm = slot(&core, &axis)?;
                    subj = core;
                    form = arm;
                    continue; // tail call
                }

                // [10 [tag hint-f] body] — hint (trace/event tag)
                Noun::Atom(10) => {
                    if let Noun::Cell(hint_cell, body) = rest.as_ref() {
                        if let Noun::Cell(_tag, hint_f) = hint_cell.as_ref() {
                            // evaluate hint formula for side effect (ignored for now)
                            let _ = eval(&subj, hint_f);
                        }
                        form = *body.clone();
                        continue; // tail call
                    }
                    return Err(err("nox-10: malformed hint"));
                }

                // [11 b] — scry (runtime hook; stub returns crash)
                Noun::Atom(11) => {
                    return Err(err("nox-11: scry requires runtime graph hook"));
                }

                _ => return Err(err(&format!("nox: unrecognized opcode: {:?}", op))),
            },

            Noun::Atom(_) => return Err(err("nox: atom is not a valid formula")),
        }
    }
}

/// Tree-address a noun by axis: `/[axis noun]`
pub fn slot(noun: &Noun, axis: &Noun) -> Result<Noun, InterpError> {
    let n = match axis {
        Noun::Atom(n) => *n,
        _ => return Err(err("slot: axis must be an atom")),
    };
    match n {
        0 => Err(err("slot: axis 0 is undefined")),
        1 => Ok(noun.clone()),
        2 => match noun {
            Noun::Cell(h, _) => Ok(*h.clone()),
            _ => Err(err("slot: /2 on atom")),
        },
        3 => match noun {
            Noun::Cell(_, t) => Ok(*t.clone()),
            _ => Err(err("slot: /3 on atom")),
        },
        n => {
            let parent = slot(noun, &Noun::Atom(n / 2))?;
            slot(&parent, &Noun::Atom(2 + (n % 2)))
        }
    }
}

fn cell2(noun: &Noun) -> Result<(Noun, Noun), InterpError> {
    match noun {
        Noun::Cell(h, t) => Ok((*h.clone(), *t.clone())),
        _ => Err(err("expected cell")),
    }
}

fn err(msg: &str) -> InterpError {
    InterpError { message: msg.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(n: u64) -> Noun { Noun::Atom(n) }
    fn cell(h: Noun, t: Noun) -> Noun { Noun::cell(h, t) }

    #[test]
    fn literal() {
        // *[0 [1 42]] = 42
        let result = eval(&atom(0), &cell(atom(1), atom(42))).unwrap();
        assert_eq!(result, atom(42));
    }

    #[test]
    fn slot_head() {
        // *[[1 2] [0 2]] = 1
        let subj = cell(atom(1), atom(2));
        let result = eval(&subj, &cell(atom(0), atom(2))).unwrap();
        assert_eq!(result, atom(1));
    }

    #[test]
    fn slot_tail() {
        // *[[1 2] [0 3]] = 2
        let subj = cell(atom(1), atom(2));
        let result = eval(&subj, &cell(atom(0), atom(3))).unwrap();
        assert_eq!(result, atom(2));
    }

    #[test]
    fn increment() {
        // *[42 [4 [0 1]]] = 43
        let result = eval(&atom(42), &cell(atom(4), cell(atom(0), atom(1)))).unwrap();
        assert_eq!(result, atom(43));
    }

    #[test]
    fn equality_true() {
        // *[0 [5 [1 42] [1 42]]] = 0 (equal)
        let result = eval(
            &atom(0),
            &cell(atom(5), cell(cell(atom(1), atom(42)), cell(atom(1), atom(42)))),
        ).unwrap();
        assert_eq!(result, atom(0));
    }

    #[test]
    fn equality_false() {
        // *[0 [5 [1 1] [1 2]]] = 1 (not equal)
        let result = eval(
            &atom(0),
            &cell(atom(5), cell(cell(atom(1), atom(1)), cell(atom(1), atom(2)))),
        ).unwrap();
        assert_eq!(result, atom(1));
    }

    #[test]
    fn if_true() {
        // *[0 [6 [1 0] [1 99] [1 0]]] = 99  (cond=0 means yes)
        let result = eval(
            &atom(0),
            &cell(atom(6), cell(cell(atom(1), atom(0)), cell(cell(atom(1), atom(99)), cell(atom(1), atom(0))))),
        ).unwrap();
        assert_eq!(result, atom(99));
    }
}
