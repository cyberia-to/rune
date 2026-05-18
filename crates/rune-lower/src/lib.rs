/// Lowers rune AST (Expr) to Nox nouns (formulas).
/// The output is a Nox formula ready for the tree-walking interpreter.
///
/// Nox formula encoding:
///   [0 axis]         slot
///   [1 value]        literal
///   [2 subj form]    eval
///   [3 form]         cell test
///   [4 form]         increment
///   [5 a b]          equality
///   [6 t y n]        if-then-else
///   [7 a b]          compose: eval b against (eval a against subject)
///   [8 a b]          push: eval b against [a subject]
///   [9 axis core]    call arm at axis in core
///   [10 [tag hint] b] hint
///   [11 a]           scry
use rune_ast::{Address, Expr, Noun};

#[derive(Debug)]
pub struct LowerError {
    pub message: String,
}

/// Lowers a rune Expr to a Nox formula noun.
/// Subject conventions: ~self=2, ~now=6, ~here=10, ~world=22, ~code=14.
pub fn lower(expr: Expr) -> Result<Noun, LowerError> {
    lower_in(expr, &Env::default())
}

#[derive(Default, Clone)]
struct Env {
    // name → subject axis mapping for let-bound variables
    bindings: Vec<(String, u64)>,
}

impl Env {
    fn with_binding(&self, name: &str, axis: u64) -> Self {
        let mut e = self.clone();
        e.bindings.push((name.to_string(), axis));
        e
    }

    fn lookup(&self, name: &str) -> Option<u64> {
        self.bindings.iter().rev().find(|(n, _)| n == name).map(|(_, a)| *a)
    }
}

fn lower_in(expr: Expr, env: &Env) -> Result<Noun, LowerError> {
    match expr {
        // Atom literal → [1 atom]
        Expr::Atom(n) => Ok(cell(1, n)),

        // Symbol → slot lookup
        Expr::Sym(name) => {
            match name.as_str() {
                // subject slots (axis by convention)
                "~self"  | "self"  => Ok(slot(2)),
                "~now"   | "now"   => Ok(slot(6)),
                "~here"  | "here"  => Ok(slot(10)),
                "~caps"  | "caps"  => Ok(slot(14)),
                "~code"  | "code"  => Ok(slot(18)),
                "~libs"  | "libs"  => Ok(slot(22)),
                "~mem"   | "mem"   => Ok(slot(26)),
                "~world" | "world" => Ok(slot(30)),
                _ => {
                    if let Some(axis) = env.lookup(&name) {
                        Ok(slot(axis))
                    } else {
                        Err(LowerError { message: format!("unbound name: {name}") })
                    }
                }
            }
        }

        // Cell literal → distribute both sides
        Expr::Cell(h, t) => {
            let hn = lower_in(*h, env)?;
            let tn = lower_in(*t, env)?;
            Ok(Noun::cell(hn, tn))
        }

        // Let binding: push value onto subject, evaluate body against extended subject
        // `[8 value-formula body-formula]` — push
        Expr::Let { name, value, body, .. } => {
            let vf = lower_in(*value, env)?;
            // body sees the let-bound name at axis 2 (head of pushed cell)
            let env2 = env.with_binding(&name, 2);
            let bf = lower_in(*body, &env2)?;
            Ok(Noun::cell(Noun::Atom(8), Noun::cell(vf, bf)))
        }

        // If-then-else → [6 cond yes no]
        Expr::If { cond, yes, no } => {
            let cf = lower_in(*cond, env)?;
            let yf = lower_in(*yes, env)?;
            let nf = lower_in(*no, env)?;
            Ok(Noun::cell(Noun::Atom(6), Noun::cell(cf, Noun::cell(yf, nf))))
        }

        // Equality → [5 a b]
        Expr::Eq(a, b) => {
            let af = lower_in(*a, env)?;
            let bf = lower_in(*b, env)?;
            Ok(Noun::cell(Noun::Atom(5), Noun::cell(af, bf)))
        }

        // Increment → [4 formula]
        Expr::Inc(inner) => {
            let f = lower_in(*inner, env)?;
            Ok(Noun::cell(Noun::Atom(4), f))
        }

        // Eval → [2 subject-formula formula-formula]
        Expr::Eval { subject, formula } => {
            let sf = lower_in(*subject, env)?;
            let ff = lower_in(*formula, env)?;
            Ok(Noun::cell(Noun::Atom(2), Noun::cell(sf, ff)))
        }

        // Compose: eval right against result of left → [7 left right]
        Expr::Compose(left, right) => {
            let lf = lower_in(*left, env)?;
            let rf = lower_in(*right, env)?;
            Ok(Noun::cell(Noun::Atom(7), Noun::cell(lf, rf)))
        }

        // Hint: [10 [tag hint-formula] body-formula]
        Expr::Hint { tag, selector, body } => {
            let tag_atom = Noun::Atom(string_to_atom(&tag));
            let sf = lower_in(*selector, env)?;
            let bf = lower_in(*body, env)?;
            Ok(Noun::cell(
                Noun::Atom(10),
                Noun::cell(Noun::cell(tag_atom, sf), bf),
            ))
        }

        // Trap (loop with `$` recur) → construct gate then call it
        Expr::Trap(body) => {
            let bf = lower_in(*body, env)?;
            // [8 [1 0] [9 2 [0 1]]] — push a gate, call arm 2 (body)
            Ok(Noun::cell(
                Noun::Atom(8),
                Noun::cell(
                    Noun::cell(Noun::Atom(1), Noun::Atom(0)),
                    Noun::cell(Noun::Atom(9), Noun::cell(Noun::Atom(2), bf)),
                ),
            ))
        }

        // Gate definition → [1 [battery sample]] — literal core
        Expr::Gate { sample, body } => {
            let sf = lower_in(*sample, env)?;
            let bf = lower_in(*body, env)?;
            Ok(Noun::cell(Noun::Atom(1), Noun::cell(bf, sf)))
        }

        // Call: push gate onto subject, invoke arm 2
        Expr::Call { gate, args } => {
            let gf = lower_in(*gate, env)?;
            // TODO: handle multiple args properly
            let arg_f = if args.len() == 1 {
                lower_in(args.into_iter().next().unwrap(), env)?
            } else {
                // pack into cell
                args.into_iter()
                    .map(|a| lower_in(a, env))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .rev()
                    .reduce(|t, h| Noun::cell(h, t))
                    .unwrap_or(Noun::Atom(0))
            };
            // [9 2 [8 gate [1 args] [0 1]]]
            Ok(Noun::cell(Noun::Atom(9), Noun::cell(Noun::Atom(2),
                Noun::cell(Noun::Atom(8), Noun::cell(gf,
                    Noun::cell(Noun::cell(Noun::Atom(1), arg_f),
                               Noun::cell(Noun::Atom(0), Noun::Atom(1))))))))
        }

        // Address forms → slot through ~world (axis 30)
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
            // encode as scry through ~world: [11 [path-atom [0 30]]]
            Ok(Noun::cell(
                Noun::Atom(11),
                Noun::cell(Noun::Atom(string_to_atom(&path)), slot(30)),
            ))
        }

        _ => Err(LowerError { message: format!("lowering not yet implemented for: {expr:?}") }),
    }
}

// Helpers for constructing common Nox formula shapes
fn cell(op: u64, arg: u64) -> Noun { Noun::cell(Noun::Atom(op), Noun::Atom(arg)) }
fn slot(axis: u64) -> Noun { cell(0, axis) }

// Minimal name→atom encoding (not hemera — placeholder until M2)
fn string_to_atom(s: &str) -> u64 {
    s.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}
