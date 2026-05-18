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
// Environment: stack of let-bound names, most-recent first, plus gate defs.
// ---------------------------------------------------------------------------

/// A gate definition stored for call-site inlining.
///
/// The body is stored as an *uncompiled* Expr so it can be re-compiled at
/// every call site with the full call-site environment.  This is required for
/// closures: a gate that captures an outer variable must re-resolve the
/// captured name against whichever subject is current at the call site.
#[derive(Clone)]
struct GateDef {
    name: String,
    params: Vec<String>,
    /// Uncompiled body — compiled fresh at each call site.
    body_expr: Expr,
}

#[derive(Default, Clone)]
struct Env {
    /// Bound names, newest at index 0.
    stack: Vec<String>,
    /// Known gate definitions, for inlining at call sites.
    gates: Vec<GateDef>,
}

impl Env {
    /// Return a new Env with `name` pushed to the front (depth 0).
    fn push(&self, name: &str) -> Self {
        let mut e = self.clone();
        e.stack.insert(0, name.to_string());
        e
    }

    /// Push `name` as a stack binding AND record a gate definition for inlining.
    fn push_gate(&self, name: &str, params: Vec<String>, body_expr: Expr) -> Self {
        let mut e = self.push(name);
        e.gates.push(GateDef { name: name.to_string(), params, body_expr });
        e
    }

    /// Look up `name`; returns Some(axis) where axis = stack_axis(index).
    fn lookup(&self, name: &str) -> Option<u64> {
        self.stack
            .iter()
            .position(|n| n == name)
            .map(|i| stack_axis(i))
    }

    /// Find a gate definition by name (last one wins).
    fn find_gate(&self, name: &str) -> Option<GateDef> {
        self.gates.iter().rev().find(|g| g.name == name).cloned()
    }
}

/// Axis for a let-bound name at stack depth `i` (0 = most recently bound).
/// depth 0 → 2, 1 → 6, 2 → 14, 3 → 30, …  (same formula as subject slots)
fn stack_axis(i: usize) -> u64 {
    (1u64 << (i + 2)) - 2
}

// ---------------------------------------------------------------------------
// Param extraction from a Gate sample expression
// ---------------------------------------------------------------------------

/// Extract parameter names from a gate sample expression.
///
/// Conventions understood:
///   - `Expr::Sym("param:x;y")` → `["x", "y"]`   (semicolon-separated list)
///   - `Expr::Sym(s)` (any other symbol)           → `[s]`    (single param)
///   - `Expr::Atom(0)`                             → `[]`     (no-arg gate)
fn extract_params(sample: &Expr) -> Vec<String> {
    match sample {
        Expr::Atom(0) => vec![],
        Expr::Sym(s) if s.starts_with("param:") => {
            s["param:".len()..]
                .split(';')
                .filter(|p| !p.is_empty())
                .map(|p| p.to_string())
                .collect()
        }
        Expr::Sym(s) => vec![s.clone()],
        _ => vec![],
    }
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
        //   [2 [3 value-formula [0 1]] [1 body-formula]]
        //
        // Special case: when the bound value is a Gate, store the uncompiled
        // body expr in the env for call-site inlining (so closures resolve
        // captured variables against the correct call-site subject).
        // ------------------------------------------------------------------
        Expr::Let { name, value, body, .. } => {
            let is_gate = matches!(*value, Expr::Gate { .. });

            let (vf, env2) = if is_gate {
                // Destructure the Gate variant
                let (sample, gate_body) = match *value {
                    Expr::Gate { sample, body: gb } => (sample, gb),
                    _ => unreachable!(),
                };

                let params = extract_params(&sample);

                // Emit a quoted placeholder as the runtime value for the let
                // slot.  All call sites are inlined so this slot is never
                // actually evaluated; it exists only to keep the subject layout
                // correct (env.push ensures existing axes shift by one).
                let placeholder = quote(0);

                // Store the raw (uncompiled) body so it can be compiled fresh
                // at each call site against the call-site env.
                let env2 = env.push_gate(&name, params, *gate_body);
                (placeholder, env2)
            } else {
                let vf = lower_in(*value, env)?;
                let env2 = env.push(&name);
                (vf, env2)
            };

            // [3 vf [0 1]]  — cons value onto current subject
            let extend = op3(vf, axis(1));
            // body sees the new binding at depth 0 (axis 2), rest shift
            let bf = lower_in(*body, &env2)?;
            // [2 extend [1 bf]]
            let quoted_body = Noun::cell(Noun::Atom(1), bf);
            Ok(op2(extend, quoted_body))
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
        //        user-defined gate names are inlined if a body is known.
        //
        // Gate call inlining (N params, N args):
        //   - Build args chain: [3 a1-f [3 a2-f [... [0 1]]]]
        //     → new_subj = [a1 [a2 ... call_site_subj]]
        //   - Compile gate body against call-site env with params pushed
        //     in REVERSE order (so param[0] → axis 2, param[1] → axis 6 …)
        //   - Emit: [2 chain [1 body-f]]
        //
        // By compiling the body at the call site (not the definition site),
        // captured outer variables resolve to the correct axes in call_site_subj.
        // ------------------------------------------------------------------
        Expr::Call { gate, args } => {
            // Peek at gate to detect built-in or known gate names
            if let Expr::Sym(ref name) = *gate {
                let name_str = name.clone();
                match name_str.as_str() {
                    "add"  => return binary_op(5,  &args, env),
                    "sub"  => return binary_op(6,  &args, env),
                    "mul"  => return binary_op(7,  &args, env),
                    "inv"  => return unary_op(8,   &args, env),
                    "lt"   => return binary_op(10, &args, env),
                    "xor"  => return binary_op(11, &args, env),
                    "and"  => return binary_op(12, &args, env),
                    "not"  => return unary_op(13,  &args, env),
                    "shl"  => return binary_op(14, &args, env),
                    "hash" => return unary_op(15,  &args, env),
                    // neg(x) → [6 [1 0] x-f]  (0 - x)
                    "neg" => {
                        if args.len() != 1 {
                            return Err(lower_err("neg requires 1 argument"));
                        }
                        let mut it = args.into_iter();
                        let xf = lower_in(it.next().unwrap(), env)?;
                        return Ok(Noun::cell(Noun::Atom(6), Noun::cell(quote(0), xf)));
                    }
                    // gt(a, b) → [10 b-f a-f]  (lt with args swapped: a > b ≡ lt(b,a))
                    "gt" => {
                        if args.len() != 2 {
                            return Err(lower_err("gt requires 2 arguments"));
                        }
                        let mut it = args.into_iter();
                        let af = lower_in(it.next().unwrap(), env)?;
                        let bf = lower_in(it.next().unwrap(), env)?;
                        return Ok(Noun::cell(Noun::Atom(10), Noun::cell(bf, af)));
                    }
                    _ => {}
                }

                // Check for an inlineable user-defined gate.
                // Compile the body fresh here, using the call-site env extended
                // with params, so that captured variables resolve correctly.
                if let Some(gdef) = env.find_gate(&name_str) {
                    if args.len() != gdef.params.len() {
                        return Err(lower_err(format!(
                            "gate '{}' takes {} args, got {}",
                            name_str, gdef.params.len(), args.len()
                        )));
                    }

                    // Build body env: push params in reverse order onto the
                    // call-site env.  After the call cons [a1 [a2 ... subj]],
                    // param[0]=a1 → axis 2, param[1]=a2 → axis 6, …
                    let mut call_body_env = env.clone();
                    for p in gdef.params.iter().rev() {
                        call_body_env = call_body_env.push(p);
                    }
                    let body_f = lower_in(gdef.body_expr, &call_body_env)?;

                    // Build args chain: [3 a1-f [3 a2-f [... [0 1]]]]
                    // Fold right so a1 is the outermost (head after cons).
                    let mut chain = axis(1); // [0 1] — tail = call_site_subj
                    for arg in args.into_iter().rev() {
                        let af = lower_in(arg, env)?;
                        chain = op3(af, chain);
                    }
                    let quoted_body = Noun::cell(Noun::Atom(1), body_f);
                    return Ok(op2(chain, quoted_body));
                }
            }

            // General gate call: lower gate formula.
            // Args are ignored in the simplified (non-inlined) path.
            let gf = lower_in(*gate, env)?;
            let _ = args;
            Ok(gf)
        }

        // ------------------------------------------------------------------
        // Gate — emit quoted body (used when a gate appears as a value
        // outside of a Let binding, e.g. passed as a first-class value).
        // ------------------------------------------------------------------
        Expr::Gate { sample, body } => {
            let params = extract_params(&sample);
            let mut body_env = env.clone();
            for p in params.iter().rev() {
                body_env = body_env.push(p);
            }
            let bf = lower_in(*body, &body_env)?;
            Ok(Noun::cell(Noun::Atom(1), bf))
        }

        // ------------------------------------------------------------------
        // Trap — execute body once.
        // Full tail-recursive recur ($ syntax) is not yet implemented.
        // ------------------------------------------------------------------
        Expr::Trap(body) => {
            lower_in(*body, env)
        }

        // ------------------------------------------------------------------
        // Loop (recur) — not yet implemented
        // ------------------------------------------------------------------
        Expr::Loop => Err(lower_err("loop recur ($) not yet implemented")),

        // ------------------------------------------------------------------
        // Match — pattern switch with equality arms.
        //
        // Wraps the subject in a let so it is evaluated once:
        //   [2 [3 sf [0 1]] [1 nested-branch-f]]
        // Inside, subject is at axis 2 ($match).  Arms are tested in order;
        // a non-exhaustive match falls through to atom 0.
        // ------------------------------------------------------------------
        Expr::Match { subject, arms } => {
            if arms.is_empty() {
                return Err(lower_err("match with no arms"));
            }

            let sf = lower_in(*subject, env)?;

            // Compile arms against an env where the matched subject is at depth 0
            let inner_env = env.push("$match");
            let subject_access = axis(2); // slot 2 = head = matched subject value

            // Build branch chain from last arm to first (fold right).
            // Non-exhaustive fallthrough: literal 0.
            let mut chain_noun: Noun = quote(0);

            for (pat, arm_body) in arms.into_iter().rev() {
                let pf = lower_in(pat, &inner_env)?;
                let bf = lower_in(arm_body, &inner_env)?;
                // eq test: [9 subject_access pf]  →  0 if equal
                let test = Noun::cell(
                    Noun::Atom(9),
                    Noun::cell(subject_access.clone(), pf),
                );
                // branch: if eq=0 (equal), take bf, else continue with chain
                chain_noun = Noun::cell(
                    Noun::Atom(4),
                    Noun::cell(test, Noun::cell(bf, chain_noun)),
                );
            }

            // Wrap: cons subject onto current subject to extend, then eval chain
            let extend = op3(sf, axis(1));
            let quoted_chain = Noun::cell(Noun::Atom(1), chain_noun);
            Ok(op2(extend, quoted_chain))
        }

        // ------------------------------------------------------------------
        // Door / Arm — not yet implemented
        // ------------------------------------------------------------------
        Expr::Door(_) | Expr::Arm { .. } => {
            Err(lower_err("Door not yet implemented"))
        }

        // ------------------------------------------------------------------
        // Rebind — not yet implemented
        // ------------------------------------------------------------------
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
        // → [2 [3 [1 99] [0 1]] [1 [0 2]]]
        // compose: new_subj = [99 s], new_form = [0 2] (quoted), eval → head = 99
        let expr = Expr::Let {
            name: "x".into(),
            mold: None,
            value: Box::new(atom_expr(99)),
            body: Box::new(sym("x")),
        };
        let f = lower(expr).unwrap();
        // [2 [3 [1 99] [0 1]] [1 [0 2]]]
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
                cell_noun(atom_noun(1), cell_noun(atom_noun(0), atom_noun(2))),
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

    // ------------------------------------------------------------------
    // Gate call tests (unit — no parser needed, build AST directly)
    // ------------------------------------------------------------------

    #[test]
    fn lower_gate_single_arg_call() {
        // let double = fn(x) { x * 2 }; double(5)  → 10
        let expr = Expr::Let {
            name: "double".into(),
            mold: None,
            value: Box::new(Expr::Gate {
                sample: Box::new(Expr::Sym("param:x".into())),
                body: Box::new(Expr::Call {
                    gate: Box::new(Expr::Sym("mul".into())),
                    args: vec![Expr::Sym("x".into()), Expr::Atom(2)],
                }),
            }),
            body: Box::new(Expr::Call {
                gate: Box::new(Expr::Sym("double".into())),
                args: vec![Expr::Atom(5)],
            }),
        };
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(10));
    }

    #[test]
    fn lower_gate_two_arg_call() {
        // let add_pair = fn(x, y) { x + y }; add_pair(3, 4)  → 7
        let expr = Expr::Let {
            name: "add_pair".into(),
            mold: None,
            value: Box::new(Expr::Gate {
                sample: Box::new(Expr::Sym("param:x;y".into())),
                body: Box::new(Expr::Call {
                    gate: Box::new(Expr::Sym("add".into())),
                    args: vec![Expr::Sym("x".into()), Expr::Sym("y".into())],
                }),
            }),
            body: Box::new(Expr::Call {
                gate: Box::new(Expr::Sym("add_pair".into())),
                args: vec![Expr::Atom(3), Expr::Atom(4)],
            }),
        };
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(7));
    }

    #[test]
    fn lower_gt() {
        // gt(7, 3) → 0  (7 > 3 is true → 0 in Nox truth convention)
        let expr = Expr::Call {
            gate: Box::new(sym("gt")),
            args: vec![atom_expr(7), atom_expr(3)],
        };
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(0)); // 0 = true
    }
}
