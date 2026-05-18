/// Lowers rune AST (Expr) to cyber Nox nouns (formulas).
/// The output is a cyber Nox formula ready for the tree-walking interpreter.
///
/// Cyber Nox opcode table:
///   0  = axis   [0 n]            — slot n from subject
///   1  = quote  [1 v]            — literal value v
///   2  = compose [2 f g]         — eval g against (eval f against subject)
///   3  = cons   [3 f g]          — produce cell [eval(f) eval(g)]
///   4  = branch [4 c y n]        — if eval(c)=0 then y else n
///   5  = add    [5 a b]
///   6  = sub    [6 a b]
///   7  = mul    [7 a b]
///   8  = inv    [8 a]
///   9  = eq     [9 a b]          — 0=equal, 1=not-equal
///  10  = lt     [10 a b]
///  11  = xor    [11 a b]
///  12  = and    [12 a b]
///  13  = not    [13 a]
///  14  = shl    [14 a b]
///  15  = hash   [15 a]
///  16  = call/hint [16 [tag sel] body]
///  17  = look   [17 [path [0 255]]]
///
/// Subject slot axes (right-nested cons list):
///   ~self  = 2   ~now   = 6    ~here  = 14   ~caps  = 30
///   ~code  = 62  ~libs  = 126  ~mem   = 254  ~world = 255
use rune_ast::{Address, Expr, Noun};

#[derive(Debug)]
pub struct LowerError {
    pub message: String,
}

/// Lowers a rune Expr to a cyber Nox formula noun.
pub fn lower(expr: Expr) -> Result<Noun, LowerError> {
    lower_in(expr, &Env::default())
}

// ---------------------------------------------------------------------------
// Environment: stack of let-bound names, most-recent first.
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
struct Env {
    /// Bound names, newest at index 0.
    stack: Vec<String>,
}

impl Env {
    /// Return a new Env with `name` pushed to the front (depth 0).
    fn push(&self, name: &str) -> Self {
        let mut e = self.clone();
        e.stack.insert(0, name.to_string());
        e
    }

    /// Look up `name`; returns Some(axis) where axis = stack_axis(index).
    fn lookup(&self, name: &str) -> Option<u64> {
        self.stack
            .iter()
            .position(|n| n == name)
            .map(|i| stack_axis(i))
    }
}

/// Axis for a let-bound name at stack depth `i` (0 = most recently bound).
/// depth 0 → 2, 1 → 6, 2 → 14, 3 → 30, …  (same formula as subject slots)
fn stack_axis(i: usize) -> u64 {
    (1u64 << (i + 2)) - 2
}

// ---------------------------------------------------------------------------
// Core lowering function
// ---------------------------------------------------------------------------

fn lower_in(expr: Expr, env: &Env) -> Result<Noun, LowerError> {
    match expr {
        // ------------------------------------------------------------------
        // Atom literal → [1 n]  (quote)
        // ------------------------------------------------------------------
        Expr::Atom(n) => Ok(quote(n)),

        // ------------------------------------------------------------------
        // Symbol → slot lookup
        // ------------------------------------------------------------------
        Expr::Sym(name) => {
            match name.as_str() {
                // Subject slot names
                "~self"  | "self"  => Ok(axis(2)),
                "~now"   | "now"   => Ok(axis(6)),
                "~here"  | "here"  => Ok(axis(14)),
                "~caps"  | "caps"  => Ok(axis(30)),
                "~code"  | "code"  => Ok(axis(62)),
                "~libs"  | "libs"  => Ok(axis(126)),
                "~mem"   | "mem"   => Ok(axis(254)),
                "~world" | "world" => Ok(axis(255)),
                _ => {
                    if let Some(ax) = env.lookup(&name) {
                        Ok(axis(ax))
                    } else {
                        Err(lower_err(format!("unbound name: {name}")))
                    }
                }
            }
        }

        // ------------------------------------------------------------------
        // Cell literal → [3 head-f tail-f]  (cons)
        // ------------------------------------------------------------------
        Expr::Cell(h, t) => {
            let hf = lower_in(*h, env)?;
            let tf = lower_in(*t, env)?;
            Ok(op3(hf, tf))
        }

        // ------------------------------------------------------------------
        // Let binding — subject extension via cons+compose:
        //   [2 [3 value-formula [0 1]] body-formula]
        // Conses value onto the subject head; body sees it at axis 2.
        // ------------------------------------------------------------------
        Expr::Let { name, value, body, .. } => {
            let vf = lower_in(*value, env)?;
            // [3 value-formula [0 1]]  — cons value onto current subject
            let extend = op3(vf, axis(1));
            // body sees the new binding at depth 0 (axis 2), rest shift
            let env2 = env.push(&name);
            let bf = lower_in(*body, &env2)?;
            // [2 extend body-formula]  — compose
            Ok(op2(extend, bf))
        }

        // ------------------------------------------------------------------
        // If → [4 cond-f yes-f no-f]  (branch)
        // ------------------------------------------------------------------
        Expr::If { cond, yes, no } => {
            let cf = lower_in(*cond, env)?;
            let yf = lower_in(*yes, env)?;
            let nf = lower_in(*no, env)?;
            Ok(Noun::cell(
                Noun::Atom(4),
                Noun::cell(cf, Noun::cell(yf, nf)),
            ))
        }

        // ------------------------------------------------------------------
        // Equality → [9 a-f b-f]
        // ------------------------------------------------------------------
        Expr::Eq(a, b) => {
            let af = lower_in(*a, env)?;
            let bf = lower_in(*b, env)?;
            Ok(Noun::cell(Noun::Atom(9), Noun::cell(af, bf)))
        }

        // ------------------------------------------------------------------
        // Inc → [5 x-f [1 1]]  (add 1; no native increment in cyber Nox)
        // ------------------------------------------------------------------
        Expr::Inc(inner) => {
            let f = lower_in(*inner, env)?;
            Ok(Noun::cell(Noun::Atom(5), Noun::cell(f, quote(1))))
        }

        // ------------------------------------------------------------------
        // Compose → [2 left-f right-f]
        // ------------------------------------------------------------------
        Expr::Compose(left, right) => {
            let lf = lower_in(*left, env)?;
            let rf = lower_in(*right, env)?;
            Ok(op2(lf, rf))
        }

        // ------------------------------------------------------------------
        // Eval → [2 subject-f formula-f]
        // ------------------------------------------------------------------
        Expr::Eval { subject, formula } => {
            let sf = lower_in(*subject, env)?;
            let ff = lower_in(*formula, env)?;
            Ok(op2(sf, ff))
        }

        // ------------------------------------------------------------------
        // Hint → [16 [tag-atom selector-f] body-f]
        // ------------------------------------------------------------------
        Expr::Hint { tag, selector, body } => {
            let tag_atom = Noun::Atom(string_to_atom(&tag));
            let sf = lower_in(*selector, env)?;
            let bf = lower_in(*body, env)?;
            Ok(Noun::cell(
                Noun::Atom(16),
                Noun::cell(Noun::cell(tag_atom, sf), bf),
            ))
        }

        // ------------------------------------------------------------------
        // Host → [16 [host-tag-atom args-f] [1 0]]
        // ------------------------------------------------------------------
        Expr::Host { target: _, args } => {
            let host_tag = Noun::Atom(string_to_atom("host"));
            let args_f = lower_in(*args, env)?;
            Ok(Noun::cell(
                Noun::Atom(16),
                Noun::cell(
                    Noun::cell(host_tag, args_f),
                    quote(0),
                ),
            ))
        }

        // ------------------------------------------------------------------
        // Address → [17 [path-atom [0 255]]]
        // where 255 is ~world axis, path encodes the address string
        // ------------------------------------------------------------------
        Expr::Address(addr) => {
            let path = match addr {
                Address::Particle(p) => p,
                Address::Neuron(n)   => format!("@{n}"),
                Address::Name(n)     => format!("~{n}"),
                Address::Path(ps)    => ps.join("/"),
                Address::Token(t)    => format!("${t}"),
                Address::Abstract(a) => format!("^{a}"),
                Address::Home        => "~/".into(),
            };
            Ok(Noun::cell(
                Noun::Atom(17),
                Noun::cell(Noun::Atom(string_to_atom(&path)), axis(255)),
            ))
        }

        // ------------------------------------------------------------------
        // Call — built-in names map to cyber Nox arithmetic/logic opcodes;
        //        other calls are general gate invocations.
        // ------------------------------------------------------------------
        Expr::Call { gate, args } => {
            // Peek at gate to detect built-in names
            if let Expr::Sym(ref name) = *gate {
                let name = name.clone();
                match name.as_str() {
                    "add" => return binary_op(5,  &args, env),
                    "sub" => return binary_op(6,  &args, env),
                    "mul" => return binary_op(7,  &args, env),
                    "inv" => return unary_op(8,   &args, env),
                    "lt"  => return binary_op(10, &args, env),
                    "xor" => return binary_op(11, &args, env),
                    "and" => return binary_op(12, &args, env),
                    "not" => return unary_op(13,  &args, env),
                    "shl" => return binary_op(14, &args, env),
                    "hash" => return unary_op(15, &args, env),
                    // neg(x) → [6 [1 0] x-f]  (0 - x)
                    "neg" => {
                        if args.len() != 1 {
                            return Err(lower_err("neg requires 1 argument"));
                        }
                        let mut it = args.into_iter();
                        let xf = lower_in(it.next().unwrap(), env)?;
                        return Ok(Noun::cell(Noun::Atom(6), Noun::cell(quote(0), xf)));
                    }
                    _ => {}
                }
            }
            // General gate call: lower gate formula, not implemented as proper
            // call mechanism yet — emit the gate formula directly for now.
            let gf = lower_in(*gate, env)?;
            let _ = args; // args unused in simplified path
            Ok(gf)
        }

        // ------------------------------------------------------------------
        // Gate — emit body only (simplified; proper call mechanism in M2+)
        // [1 body-f]
        // ------------------------------------------------------------------
        Expr::Gate { body, .. } => {
            let bf = lower_in(*body, env)?;
            Ok(Noun::cell(Noun::Atom(1), bf))
        }

        // ------------------------------------------------------------------
        // Trap — push quoted body onto subject head, then take slot 2.
        // [2 [3 [1 body-f] [0 1]] [0 2]]
        // ------------------------------------------------------------------
        Expr::Trap(body) => {
            let bf = lower_in(*body, env)?;
            // [3 [1 body-f] [0 1]]  — cons quoted body onto subject
            let extend = op3(Noun::cell(Noun::Atom(1), bf), axis(1));
            // [2 extend [0 2]]  — compose; slot 2 = head = body
            Ok(op2(extend, axis(2)))
        }

        // ------------------------------------------------------------------
        // Door / Arm — not yet implemented
        // ------------------------------------------------------------------
        Expr::Door(_) | Expr::Arm { .. } => {
            Err(lower_err("Door not yet implemented"))
        }

        // ------------------------------------------------------------------
        // Match / Rebind — not yet implemented
        // ------------------------------------------------------------------
        Expr::Match { .. } => Err(lower_err("Match not yet implemented")),
        Expr::Rebind { .. } => Err(lower_err("Rebind not yet implemented")),

        // ------------------------------------------------------------------
        // Everything else
        // ------------------------------------------------------------------
        _ => Err(lower_err(format!("lowering not yet implemented for: {expr:?}"))),
    }
}

// ---------------------------------------------------------------------------
// Helpers for constructing cyber Nox formula shapes
// ---------------------------------------------------------------------------

/// `[0 n]` — slot n
fn axis(n: u64) -> Noun {
    Noun::cell(Noun::Atom(0), Noun::Atom(n))
}

/// `[1 n]` — literal atom n
fn quote(n: u64) -> Noun {
    Noun::cell(Noun::Atom(1), Noun::Atom(n))
}

/// `[2 f g]` — compose
fn op2(f: Noun, g: Noun) -> Noun {
    Noun::cell(Noun::Atom(2), Noun::cell(f, g))
}

/// `[3 f g]` — cons
fn op3(f: Noun, g: Noun) -> Noun {
    Noun::cell(Noun::Atom(3), Noun::cell(f, g))
}

/// Emit a binary opcode: `[op arg0-f arg1-f]`
fn binary_op(op: u64, args: &[Expr], env: &Env) -> Result<Noun, LowerError> {
    if args.len() != 2 {
        return Err(lower_err(format!("op {op} requires 2 arguments, got {}", args.len())));
    }
    let mut it = args.iter();
    let af = lower_in(it.next().unwrap().clone(), env)?;
    let bf = lower_in(it.next().unwrap().clone(), env)?;
    Ok(Noun::cell(Noun::Atom(op), Noun::cell(af, bf)))
}

/// Emit a unary opcode: `[op arg0-f]`
fn unary_op(op: u64, args: &[Expr], env: &Env) -> Result<Noun, LowerError> {
    if args.len() != 1 {
        return Err(lower_err(format!("op {op} requires 1 argument, got {}", args.len())));
    }
    let xf = lower_in(args[0].clone(), env)?;
    Ok(Noun::cell(Noun::Atom(op), xf))
}

fn lower_err(msg: impl Into<String>) -> LowerError {
    LowerError { message: msg.into() }
}

// ---------------------------------------------------------------------------
// Minimal name→atom encoding (placeholder until hemera lands in M2)
// ---------------------------------------------------------------------------
fn string_to_atom(s: &str) -> u64 {
    s.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn sym(s: &str) -> Expr { Expr::Sym(s.to_string()) }
    fn atom_expr(n: u64) -> Expr { Expr::Atom(n) }
    fn atom_noun(n: u64) -> Noun { Noun::Atom(n) }
    fn cell_noun(h: Noun, t: Noun) -> Noun { Noun::cell(h, t) }

    #[test]
    fn lower_literal() {
        // Expr::Atom(42) → [1 42]
        let f = lower(atom_expr(42)).unwrap();
        assert_eq!(f, cell_noun(atom_noun(1), atom_noun(42)));
    }

    #[test]
    fn lower_subject_slots() {
        assert_eq!(lower(sym("~self")).unwrap(),  cell_noun(atom_noun(0), atom_noun(2)));
        assert_eq!(lower(sym("~now")).unwrap(),   cell_noun(atom_noun(0), atom_noun(6)));
        assert_eq!(lower(sym("~here")).unwrap(),  cell_noun(atom_noun(0), atom_noun(14)));
        assert_eq!(lower(sym("~caps")).unwrap(),  cell_noun(atom_noun(0), atom_noun(30)));
        assert_eq!(lower(sym("~code")).unwrap(),  cell_noun(atom_noun(0), atom_noun(62)));
        assert_eq!(lower(sym("~libs")).unwrap(),  cell_noun(atom_noun(0), atom_noun(126)));
        assert_eq!(lower(sym("~mem")).unwrap(),   cell_noun(atom_noun(0), atom_noun(254)));
        assert_eq!(lower(sym("~world")).unwrap(), cell_noun(atom_noun(0), atom_noun(255)));
    }

    #[test]
    fn lower_eq() {
        // Eq(Atom(1), Atom(2)) → [9 [1 1] [1 2]]
        let f = lower(Expr::Eq(
            Box::new(atom_expr(1)),
            Box::new(atom_expr(2)),
        )).unwrap();
        assert_eq!(
            f,
            cell_noun(
                atom_noun(9),
                cell_noun(
                    cell_noun(atom_noun(1), atom_noun(1)),
                    cell_noun(atom_noun(1), atom_noun(2)),
                ),
            )
        );
    }

    #[test]
    fn lower_inc() {
        // Inc(Atom(5)) → [5 [1 5] [1 1]]
        let f = lower(Expr::Inc(Box::new(atom_expr(5)))).unwrap();
        assert_eq!(
            f,
            cell_noun(
                atom_noun(5),
                cell_noun(
                    cell_noun(atom_noun(1), atom_noun(5)),
                    cell_noun(atom_noun(1), atom_noun(1)),
                ),
            )
        );
    }

    #[test]
    fn lower_let_binding() {
        // Let { name: "x", value: Atom(99), body: Sym("x") }
        // → [2 [3 [1 99] [0 1]] [0 2]]
        let expr = Expr::Let {
            name: "x".into(),
            mold: None,
            value: Box::new(atom_expr(99)),
            body: Box::new(sym("x")),
        };
        let f = lower(expr).unwrap();
        // [2 [3 [1 99] [0 1]] [0 2]]
        let expected = cell_noun(
            atom_noun(2),
            cell_noun(
                cell_noun(
                    atom_noun(3),
                    cell_noun(
                        cell_noun(atom_noun(1), atom_noun(99)),
                        cell_noun(atom_noun(0), atom_noun(1)),
                    ),
                ),
                cell_noun(atom_noun(0), atom_noun(2)),
            ),
        );
        assert_eq!(f, expected);
    }

    #[test]
    fn lower_builtin_add() {
        // Call { gate: Sym("add"), args: [Atom(3), Atom(4)] } → [5 [1 3] [1 4]]
        let f = lower(Expr::Call {
            gate: Box::new(sym("add")),
            args: vec![atom_expr(3), atom_expr(4)],
        }).unwrap();
        assert_eq!(
            f,
            cell_noun(
                atom_noun(5),
                cell_noun(
                    cell_noun(atom_noun(1), atom_noun(3)),
                    cell_noun(atom_noun(1), atom_noun(4)),
                ),
            )
        );
    }

    #[test]
    fn lower_builtin_neg() {
        // neg(Atom(7)) → [6 [1 0] [1 7]]
        let f = lower(Expr::Call {
            gate: Box::new(sym("neg")),
            args: vec![atom_expr(7)],
        }).unwrap();
        assert_eq!(
            f,
            cell_noun(
                atom_noun(6),
                cell_noun(
                    cell_noun(atom_noun(1), atom_noun(0)),
                    cell_noun(atom_noun(1), atom_noun(7)),
                ),
            )
        );
    }

    #[test]
    fn lower_if() {
        // If { cond: Atom(0), yes: Atom(1), no: Atom(2) } → [4 [1 0] [1 1] [1 2]]
        let f = lower(Expr::If {
            cond: Box::new(atom_expr(0)),
            yes:  Box::new(atom_expr(1)),
            no:   Box::new(atom_expr(2)),
        }).unwrap();
        assert_eq!(
            f,
            cell_noun(
                atom_noun(4),
                cell_noun(
                    cell_noun(atom_noun(1), atom_noun(0)),
                    cell_noun(
                        cell_noun(atom_noun(1), atom_noun(1)),
                        cell_noun(atom_noun(1), atom_noun(2)),
                    ),
                ),
            )
        );
    }
}
