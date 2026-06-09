/// Mold system for rune.
/// A mold is a gate (Noun → Noun) that idempotently normalizes values to their type.
/// Molds are runtime values: composable, storable, passable.
/// The compiler also evaluates them statically for type checking.
use rune_ast::{Expr, Noun};

/// A mold: a normalizing function from Noun to Noun.
/// `normalize` returns Ok(canonical) or Err if the value is outside the mold.
pub trait Mold: Send + Sync {
    fn name(&self) -> &str;
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError>;
    fn check(&self, noun: &Noun) -> bool { self.normalize(noun).is_ok() }
}

#[derive(Debug)]
pub struct MoldError {
    pub message: String,
}

/// `@ud` — unsigned decimal: any atom is valid
pub struct UdMold;
impl Mold for UdMold {
    fn name(&self) -> &str { "@ud" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@ud: expected atom".into() }),
        }
    }
}

/// `@da` — absolute date: atom representing unix timestamp
pub struct DaMold;
impl Mold for DaMold {
    fn name(&self) -> &str { "@da" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@da: expected atom".into() }),
        }
    }
}

/// `#` — particle: any noun (content-addressed, always valid)
pub struct ParticleMold;
impl Mold for ParticleMold {
    fn name(&self) -> &str { "#" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        Ok(noun.clone()) // any noun is a valid particle
    }
}

/// `@neuron` — neuron identity: atom (hemera hash of public key)
pub struct NeuronMold;
impl Mold for NeuronMold {
    fn name(&self) -> &str { "@neuron" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@neuron: expected atom".into() }),
        }
    }
}

/// `@p` — phonetic: any atom, displayed in phonetic syllables
pub struct PMold;
impl Mold for PMold {
    fn name(&self) -> &str { "@p" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@p: expected atom".into() }),
        }
    }
}

/// `@tas` — term/symbol: any atom representing a symbolic name
pub struct TasMold;
impl Mold for TasMold {
    fn name(&self) -> &str { "@tas" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@tas: expected atom".into() }),
        }
    }
}

/// `@t` — text cord: any atom representing UTF-8 bytes
pub struct TMold;
impl Mold for TMold {
    fn name(&self) -> &str { "@t" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@t: expected atom".into() }),
        }
    }
}

/// `@flag` — boolean: must be atom 0 or 1
pub struct FlagMold;
impl Mold for FlagMold {
    fn name(&self) -> &str { "@flag" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(0) | Noun::Atom(1) => Ok(noun.clone()),
            Noun::Atom(n) => Err(MoldError { message: format!("@flag: expected 0 or 1, got {n}") }),
            _ => Err(MoldError { message: "@flag: expected atom".into() }),
        }
    }
}

// ─── Strata algebra molds ────────────────────────────────────────────────────

/// Goldilocks prime p = 2^64 − 2^32 + 1
pub const GOLDILOCKS_PRIME: u64 = 0xFFFF_FFFF_0000_0001u64;

/// `@nebu` — Goldilocks scalar field element (value in [0, p))
pub struct NebuMold;
impl Mold for NebuMold {
    fn name(&self) -> &str { "@nebu" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(n) => {
                if *n < GOLDILOCKS_PRIME {
                    Ok(noun.clone())
                } else {
                    Ok(Noun::Atom(n % GOLDILOCKS_PRIME))
                }
            }
            _ => Err(MoldError { message: "@nebu: expected atom (Goldilocks scalar)".into() }),
        }
    }
}

/// `@kuro` — F₂ tower element over Goldilocks, represented as a pair [a b] (a + b·i).
/// A bare atom n is coerced to [n 0].
pub struct KuroMold;
impl Mold for KuroMold {
    fn name(&self) -> &str { "@kuro" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Cell(h, t) => {
                let nebu = NebuMold;
                let a = nebu.normalize(h)?;
                let b = nebu.normalize(t)?;
                Ok(Noun::cell(a, b))
            }
            Noun::Atom(n) => {
                let a = NebuMold.normalize(&Noun::Atom(*n))?;
                Ok(Noun::cell(a, Noun::Atom(0)))
            }
        }
    }
}

/// `@jali` — ring element (integral domain).  Stub: accept any noun.
/// Full validation deferred to strata integration.
pub struct JaliMold;
impl Mold for JaliMold {
    fn name(&self) -> &str { "@jali" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        Ok(noun.clone())
    }
}

/// Sentinel value representing ∞ in the tropical semiring.
pub const TROP_INF: u64 = u64::MAX;

/// `@trop` — tropical semiring element (ℝ ∪ {∞}, min, +).
/// Stored as a fixed-point atom (value × 1000), or TROP_INF for ∞.
pub struct TropMold;
impl Mold for TropMold {
    fn name(&self) -> &str { "@trop" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@trop: expected atom (tropical semiring element)".into() }),
        }
    }
}

/// `@genies` — CSIDH-512 isogeny group element.
/// Stub: accept any atom (compressed point hash) until strata is integrated.
pub struct GeniesMold;
impl Mold for GeniesMold {
    fn name(&self) -> &str { "@genies" }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        match noun {
            Noun::Atom(_) => Ok(noun.clone()),
            _ => Err(MoldError { message: "@genies: expected atom (CSIDH-512 element)".into() }),
        }
    }
}

// ─── Mold composition ────────────────────────────────────────────────────────

/// Intersection mold: normalizes through `first`, then `second`.
pub struct MoldAnd {
    first: Box<dyn Mold>,
    second: Box<dyn Mold>,
    mold_name: String,
}

impl MoldAnd {
    pub fn new(first: Box<dyn Mold>, second: Box<dyn Mold>) -> Self {
        let n = format!("({} & {})", first.name(), second.name());
        MoldAnd { first, second, mold_name: n }
    }
}

impl Mold for MoldAnd {
    fn name(&self) -> &str { &self.mold_name }
    fn normalize(&self, noun: &Noun) -> Result<Noun, MoldError> {
        let r = self.first.normalize(noun)?;
        self.second.normalize(&r)
    }
}

/// Construct an intersection mold that normalizes through both `first` and `second`.
pub fn mold_and(first: Box<dyn Mold>, second: Box<dyn Mold>) -> Box<dyn Mold> {
    Box::new(MoldAnd::new(first, second))
}

// ─── Resolver ────────────────────────────────────────────────────────────────

/// Resolve a mold name to its implementation.
pub fn resolve(name: &str) -> Option<Box<dyn Mold>> {
    match name {
        "@ud"     | "ud"     => Some(Box::new(UdMold)),
        "@da"     | "da"     => Some(Box::new(DaMold)),
        "#"                  => Some(Box::new(ParticleMold)),
        "@neuron" | "neuron" => Some(Box::new(NeuronMold)),
        "@p"      | "p"      => Some(Box::new(PMold)),
        "@tas"    | "tas"    => Some(Box::new(TasMold)),
        "@t"      | "t"      => Some(Box::new(TMold)),
        "@flag"   | "flag"   => Some(Box::new(FlagMold)),
        // Strata algebra molds (stubs until strata crate is integrated)
        "@nebu"   | "nebu"   => Some(Box::new(NebuMold)),
        "@kuro"   | "kuro"   => Some(Box::new(KuroMold)),
        "@jali"   | "jali"   => Some(Box::new(JaliMold)),
        "@trop"   | "trop"   => Some(Box::new(TropMold)),
        "@genies" | "genies" => Some(Box::new(GeniesMold)),
        _ => None,
    }
}

/// Convert a literal expression (atom or nested cell of literals) to a noun for
/// mold normalization. Non-literals (names, calls, text) return `None` — they
/// are not statically known. Text is handled separately (tape representation).
fn literal_noun(e: &Expr) -> Option<Noun> {
    match e {
        Expr::Atom(n) => Some(Noun::Atom(*n)),
        Expr::Cell(h, t) => Some(Noun::cell(literal_noun(h)?, literal_noun(t)?)),
        _ => None,
    }
}

/// Static mold checker — walks AST and emits soft warnings (non-blocking).
pub struct MoldChecker {
    bindings: Vec<(String, String)>, // name → mold_name
    errors: Vec<MoldError>,
}

impl MoldChecker {
    /// Walk `expr` and return all mold violations found.
    pub fn check(expr: &Expr) -> Vec<MoldError> {
        let mut checker = MoldChecker { bindings: Vec::new(), errors: Vec::new() };
        checker.check_expr(expr);
        checker.errors
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Let { name, mold, value, body } => {
                self.check_expr(value);
                // If the let has a mold annotation, record the binding
                if let Some(mold_expr) = mold {
                    let mold_name = self.mold_name(mold_expr);
                    self.check_literal_mold(value, &mold_name);
                    self.bindings.push((name.clone(), mold_name));
                }
                self.check_expr(body);
                // Pop binding after body (shadowing safe via stack)
                if mold.is_some() {
                    self.bindings.pop();
                }
            }

            Expr::Call { gate: gate_expr, args } => {
                // Check arithmetic builtins: require exactly 2 args
                if let Expr::Sym(op) = gate_expr.as_ref() {
                    match op.as_str() {
                        "add" | "sub" | "mul" => {
                            if args.len() != 2 {
                                self.errors.push(MoldError {
                                    message: format!(
                                        "mold warning: `{op}` expects 2 arguments, got {}",
                                        args.len()
                                    ),
                                });
                            }
                            // Check that args are @ud if their types are known
                            for arg in args {
                                self.check_arg_ud(arg);
                                self.check_expr(arg);
                            }
                        }
                        _ => {
                            for arg in args { self.check_expr(arg); }
                            self.check_expr(gate_expr);
                        }
                    }
                } else {
                    self.check_expr(gate_expr);
                    for arg in args { self.check_expr(arg); }
                }
            }

            Expr::Gate { sample, body } => {
                self.check_expr(sample);
                self.check_expr(body);
            }
            Expr::If { cond, yes, no } => {
                self.check_expr(cond);
                self.check_expr(yes);
                self.check_expr(no);
            }
            Expr::Eq(a, b) => { self.check_expr(a); self.check_expr(b); }
            Expr::Inc(x)   => { self.check_expr(x); }
            Expr::Cell(h, t) => { self.check_expr(h); self.check_expr(t); }
            Expr::Trap(body) => { self.check_expr(body); }
            Expr::Compose(l, r) => { self.check_expr(l); self.check_expr(r); }
            Expr::Hint { selector, body, .. } => {
                self.check_expr(selector);
                self.check_expr(body);
            }
            // Leaves — nothing to check
            Expr::Atom(_) | Expr::Text(_) | Expr::Sym(_) | Expr::Address(_) | Expr::Loop => {}
            // Remaining variants — recurse if possible, skip otherwise
            _ => {}
        }
    }

    /// Check a literal against a mold at a typed site (`let x: @mold = <lit>`,
    /// `<lit> as @mold`). Atom and cell literals run the resolved mold for real,
    /// so every mold enforces its own constraint (`@flag = 2`, `@ud = [1 2]`, …).
    /// Text literals are checked structurally — they lower to a tape cell, which
    /// the atom-based molds don't model, so a text mold is required by name.
    fn check_literal_mold(&mut self, value: &Expr, mold_name: &str) {
        const TEXT_MOLDS: [&str; 6] = ["@t", "t", "@tas", "tas", "@p", "p"];
        if let Expr::Text(_) = value {
            if !TEXT_MOLDS.contains(&mold_name) {
                self.errors.push(MoldError {
                    message: format!(
                        "mold error: text literal bound to `{mold_name}`, expected a text mold (@t)"
                    ),
                });
            }
            return;
        }
        if let (Some(mold), Some(noun)) = (resolve(mold_name), literal_noun(value)) {
            if let Err(e) = mold.normalize(&noun) {
                self.errors.push(MoldError {
                    message: format!("mold error: {}", e.message),
                });
            }
        }
    }

    /// Check that an argument is consistent with @ud (atom) if its type is known.
    fn check_arg_ud(&mut self, arg: &Expr) {
        match arg {
            Expr::Cell(_, _) => {
                self.errors.push(MoldError {
                    message: "mold warning: arithmetic argument appears to be a cell, expected @ud".into(),
                });
            }
            Expr::Sym(name) => {
                // Look up in bindings; warn if known to be non-@ud
                if let Some(mold_name) = self.lookup_mold(name) {
                    // Algebra molds carry numeric semantics — suppress the @ud warning for them.
                    let algebra_molds = [
                        "@nebu", "nebu", "@kuro", "kuro",
                        "@jali", "jali", "@trop", "trop",
                        "@genies", "genies",
                    ];
                    if algebra_molds.contains(&mold_name.as_str()) {
                        return;
                    }
                    if mold_name != "@ud" && mold_name != "ud" {
                        self.errors.push(MoldError {
                            message: format!(
                                "mold warning: arithmetic argument `{name}` has mold `{mold_name}`, expected @ud"
                            ),
                        });
                    }
                }
            }
            _ => {} // atom literals and unknown expressions are fine
        }
    }

    fn lookup_mold(&self, name: &str) -> Option<String> {
        self.bindings.iter().rev().find(|(n, _)| n == name).map(|(_, m)| m.clone())
    }

    /// Extract a mold name string from a mold expression.
    fn mold_name(&self, expr: &Expr) -> String {
        match expr {
            Expr::Address(rune_ast::Address::Neuron(name)) => format!("@{name}"),
            Expr::Sym(s) => s.clone(),
            _ => "<unknown>".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ud_accepts_atom() {
        assert!(UdMold.check(&Noun::Atom(42)));
    }

    #[test]
    fn flag_accepts_zero_one() {
        assert!(FlagMold.check(&Noun::Atom(0)));
        assert!(FlagMold.check(&Noun::Atom(1)));
        assert!(!FlagMold.check(&Noun::Atom(2)));
    }

    #[test]
    fn flag_rejects_cell() {
        assert!(!FlagMold.check(&Noun::Cell(Box::new(Noun::Atom(0)), Box::new(Noun::Atom(1)))));
    }

    #[test]
    fn resolve_new_molds() {
        assert!(resolve("@p").is_some());
        assert!(resolve("@tas").is_some());
        assert!(resolve("@t").is_some());
        assert!(resolve("@flag").is_some());
    }

    /// `let x: <mold> = <value>` and return the checker's errors.
    fn checked(mold: &str, value: Expr) -> Vec<MoldError> {
        let e = Expr::Let {
            name: "x".into(),
            mold: Some(Box::new(Expr::Sym(mold.into()))),
            value: Box::new(value),
            body: Box::new(Expr::Sym("x".into())),
        };
        MoldChecker::check(&e)
    }

    #[test]
    fn checker_runs_the_real_mold_on_atom_literals() {
        // @flag enforces 0/1 — caught via the registry, not a heuristic.
        assert!(!checked("@flag", Expr::Atom(2)).is_empty());
        assert!(checked("@flag", Expr::Atom(1)).is_empty());
        assert!(checked("@flag", Expr::Atom(0)).is_empty());
    }

    #[test]
    fn checker_rejects_cell_for_atom_mold() {
        let cell = Expr::Cell(Box::new(Expr::Atom(1)), Box::new(Expr::Atom(2)));
        assert!(!checked("@ud", cell).is_empty());
        assert!(checked("@ud", Expr::Atom(5)).is_empty());
    }

    #[test]
    fn checker_text_requires_a_text_mold() {
        assert!(!checked("@ud", Expr::Text("hi".into())).is_empty());
        assert!(checked("@t", Expr::Text("hi".into())).is_empty());
    }

    #[test]
    fn checker_warns_on_bad_arg_count() {
        // add(1) — wrong arg count
        let expr = Expr::Call {
            gate: Box::new(Expr::Sym("add".into())),
            args: vec![Expr::Atom(1)],
        };
        let errs = MoldChecker::check(&expr);
        assert!(!errs.is_empty());
    }

    #[test]
    fn checker_ok_on_correct_add() {
        let expr = Expr::Call {
            gate: Box::new(Expr::Sym("add".into())),
            args: vec![Expr::Atom(1), Expr::Atom(2)],
        };
        let errs = MoldChecker::check(&expr);
        assert!(errs.is_empty());
    }

    #[test]
    fn checker_warns_cell_in_arithmetic() {
        let expr = Expr::Call {
            gate: Box::new(Expr::Sym("mul".into())),
            args: vec![
                Expr::Cell(Box::new(Expr::Atom(1)), Box::new(Expr::Atom(2))),
                Expr::Atom(3),
            ],
        };
        let errs = MoldChecker::check(&expr);
        assert!(!errs.is_empty());
    }

    // ── text-literal mold checks ───────────────────────────────────────────────

    fn typed_let(mold: &str, value: Expr) -> Expr {
        Expr::Let {
            name: "x".into(),
            mold: Some(Box::new(Expr::Address(rune_ast::Address::Neuron(mold.into())))),
            value: Box::new(value),
            body: Box::new(Expr::Atom(0)),
        }
    }

    #[test]
    fn text_literal_rejects_numeric_mold() {
        // let x: @ud = "hi"  → error
        let errs = MoldChecker::check(&typed_let("ud", Expr::Text("hi".into())));
        assert!(!errs.is_empty(), "text bound to @ud should error");
    }

    #[test]
    fn text_literal_accepts_text_mold() {
        // let x: @t = "hi"  → ok
        let errs = MoldChecker::check(&typed_let("t", Expr::Text("hi".into())));
        assert!(errs.is_empty(), "text bound to @t should pass, got {errs:?}");
    }

    #[test]
    fn cell_rejects_ud_mold() {
        // let x: @ud = [1 2]  → error
        let cell = Expr::Cell(Box::new(Expr::Atom(1)), Box::new(Expr::Atom(2)));
        let errs = MoldChecker::check(&typed_let("ud", cell));
        assert!(!errs.is_empty(), "cell bound to @ud should error");
    }

    // ── @nebu tests ──────────────────────────────────────────────────────────

    #[test]
    fn nebu_accepts_valid() {
        assert!(NebuMold.check(&Noun::Atom(0)));
        assert!(NebuMold.check(&Noun::Atom(42)));
        assert!(NebuMold.check(&Noun::Atom(GOLDILOCKS_PRIME - 1)));
    }

    #[test]
    fn nebu_normalizes_overflow() {
        // GOLDILOCKS_PRIME mod p == 0
        let result = NebuMold.normalize(&Noun::Atom(GOLDILOCKS_PRIME)).unwrap();
        assert_eq!(result, Noun::Atom(0));
    }

    #[test]
    fn nebu_rejects_cell() {
        let cell = Noun::cell(Noun::Atom(1), Noun::Atom(2));
        assert!(!NebuMold.check(&cell));
    }

    // ── @kuro tests ──────────────────────────────────────────────────────────

    #[test]
    fn kuro_accepts_pair() {
        let pair = Noun::cell(Noun::Atom(3), Noun::Atom(7));
        assert!(KuroMold.check(&pair));
    }

    #[test]
    fn kuro_accepts_atom() {
        assert!(KuroMold.check(&Noun::Atom(42)));
    }

    // ── resolver ─────────────────────────────────────────────────────────────

    #[test]
    fn resolve_all_strata_molds() {
        for name in &["@nebu", "@kuro", "@jali", "@trop", "@genies"] {
            assert!(resolve(name).is_some(), "missing mold: {}", name);
        }
    }

    // ── mold_and composition ─────────────────────────────────────────────────

    #[test]
    fn mold_and_composition() {
        // ud AND flag: must satisfy both — only 0 and 1 pass
        let combined = mold_and(resolve("@ud").unwrap(), resolve("@flag").unwrap());
        assert!(combined.check(&Noun::Atom(0)));
        assert!(combined.check(&Noun::Atom(1)));
        assert!(!combined.check(&Noun::Atom(5))); // valid @ud but not @flag
    }
}
