/// A Nox noun — the universal data type.
/// Atom = unsigned integer. Cell = ordered pair.
/// Both registers parse to Expr which lowers to Noun.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Noun {
    Atom(u64),
    Cell(Box<Noun>, Box<Noun>),
}

impl Noun {
    pub fn cell(head: Noun, tail: Noun) -> Self {
        Noun::Cell(Box::new(head), Box::new(tail))
    }
}

/// High-level expression AST — shared between classic and pure registers.
/// Both registers parse to this; lowering produces a Noun.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Unsigned integer literal
    Atom(u64),
    /// Name reference — resolved against the subject
    Sym(String),
    /// Ordered pair
    Cell(Box<Expr>, Box<Expr>),
    /// Gate definition: sample type + body
    Gate { sample: Box<Expr>, body: Box<Expr> },
    /// Let binding: name, value, continuation
    Let { name: String, mold: Option<Box<Expr>>, value: Box<Expr>, body: Box<Expr> },
    /// Function call
    Call { gate: Box<Expr>, args: Vec<Expr> },
    /// Conditional
    If { cond: Box<Expr>, yes: Box<Expr>, no: Box<Expr> },
    /// Pattern match (switch)
    Match { subject: Box<Expr>, arms: Vec<(Expr, Expr)> },
    /// Trap: loop with `$` as recur point
    Trap(Box<Expr>),
    /// Recur to enclosing trap
    Loop,
    /// Rebind a subject slot and continue
    Rebind { slot: String, value: Box<Expr>, body: Box<Expr> },
    /// Compose: evaluate right against left
    Compose(Box<Expr>, Box<Expr>),
    /// Cast to mold
    Cast { mold: Box<Expr>, expr: Box<Expr> },
    /// Increment (Nox pattern 4)
    Inc(Box<Expr>),
    /// Equality test (Nox pattern 9)
    Eq(Box<Expr>, Box<Expr>),
    /// Field addition (Nox pattern 5)
    Add(Box<Expr>, Box<Expr>),
    /// Field subtraction (Nox pattern 6)
    Sub(Box<Expr>, Box<Expr>),
    /// Field multiplication (Nox pattern 7)
    Mul(Box<Expr>, Box<Expr>),
    /// Evaluate formula against subject (Nox pattern 2 / `.*`)
    Eval { subject: Box<Expr>, formula: Box<Expr> },
    /// Scry — read from the cybergraph (`.^`)
    Scry { mold: Box<Expr>, path: Box<Expr> },
    /// Hint: event-reactive yield
    Hint { tag: String, selector: Box<Expr>, body: Box<Expr> },
    /// Host call: escape to WASM / wGPU / glia
    Host { target: String, args: Box<Expr> },
    /// Arm definition (within a door/core)
    Arm { name: String, body: Box<Expr> },
    /// Door: multi-arm core
    Door(Vec<Expr>),
    /// Method call on a door: obj.method(args)
    MethodCall { obj: String, method: String, args: Vec<Expr> },
    /// A cybermark address embedded in expression context
    Address(Address),
}

/// A cybermark address — any sigil form resolves to a subject slot or graph lookup.
#[derive(Debug, Clone)]
pub enum Address {
    /// `#path/to/concept` or `#QmHash` — particle reference
    Particle(String),
    /// `@alice` or `@QmNeuron` — neuron reference
    Neuron(String),
    /// `~name` or `~/@alice/name` — named particle
    Name(String),
    /// `/path/segment` — explicit path
    Path(Vec<String>),
    /// `$TOKEN` — token reference
    Token(String),
    /// `^concept` — abstract root concept
    Abstract(String),
    /// `~/` — home namespace root
    Home,
}
