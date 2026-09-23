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

/// Chunk-noun ABI — the wire between rune result nouns and prysm/tade chunks.
///
/// A prysm element is a noun `[tag payload]` where `tag` is one of these small
/// atoms. `rune-lower` emits these from `text(..)`/`button(..)`/`col(..)` builtins;
/// `rune-prysm::noun_to_chunks` decodes them into `tade::Chunk`s, which prysm's
/// `dispatch` renders. This is the only contract between the two; rune never
/// touches Bevy.
pub mod tag {
    /// `[0 e1 [e2 ... 0]]` — a column/list of elements (null-terminated).
    pub const LIST: u64 = 0;
    /// `[1 <tape>]` — body text → `(#, t)` HAX/TEXT
    pub const TEXT: u64 = 1;
    /// `[2 <tape>]` — annotation → `(~, t)` SIG/TEXT
    pub const ANNO: u64 = 2;
    /// `[3 <tape>]` — error → `(!, e)` ZAP/ERROR
    pub const ERROR: u64 = 3;
    /// `[4 <tape>]` — log line → `(., l)` DOT/LOG
    pub const LOG: u64 = 4;
    /// `[5 [<label-tape> <target-tape>]]` — action → `(!, c)` ZAP/COMPONENT
    pub const BUTTON: u64 = 5;
    /// `[6 [<value-tape> <caption-tape>]]` — census card → `($, c)` BUC/COMPONENT
    pub const STAT: u64 = 6;
    /// `[7 e1 [e2 … 0]]` — horizontal row of elements → `(+, c)` LUS/COMPONENT
    pub const ROW: u64 = 7;
    /// `[8 row0 [row1 … 0]]` — table of `row`s → `(#, T)` HAX/TABLE
    pub const TABLE: u64 = 8;
}

/// Act ABI — the world-touching effects a runtime requests through the ward.
///
/// An act is encoded as opcode-16: `[16 [act-tag args-f] cont]`. The interpreter
/// (`eval_with_host`) reads `~caps` (axis 30), hands `(act, args, caps)` to a
/// `Host`, splices the result, and continues `cont`. Pure `eval` no-ops acts.
/// Non-act opcode-16 tags remain hints (event parking). See `cyb/root/ward.md`.
pub mod act {
    /// Act tags live in a high reserved window, clear of user hint tags
    /// (small ints / string-hash tags won't realistically land here).
    pub const BASE: u64 = 0xAC75_0000_0000_0000;
    pub const EMIT: u64 = BASE + 1; // render prysm chunks to a surface
    pub const QUERY: u64 = BASE + 2; // read the cybergraph (one-shot)
    pub const LINK: u64 = BASE + 3; // write a cyberlink
    pub const SEAL: u64 = BASE + 4; // finalize an intent
    pub const SUBSCRIBE: u64 = BASE + 5; // stream graph events
    pub const HOST: u64 = BASE + 6; // invoke another runtime

    /// The subject slot holding the requester's capabilities.
    pub const CAPS_AXIS: u64 = 30;

    /// True if `tag` names an act (vs an ordinary hint).
    pub fn is_act(tag: u64) -> bool {
        (BASE..=BASE + 0xFF).contains(&tag)
    }
}

/// High-level expression AST — shared between classic and pure registers.
/// Both registers parse to this; lowering produces a Noun.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Unsigned integer literal
    Atom(u64),
    /// UTF-8 text literal — lowers to a `tape` noun (null-terminated byte list)
    Text(String),
    /// Name reference — resolved against the subject
    Sym(String),
    /// Ordered pair
    Cell(Box<Expr>, Box<Expr>),
    /// Gate definition: sample type + body
    Gate { sample: Box<Expr>, body: Box<Expr> },
    /// Let binding: name, value, continuation
    Let {
        name: String,
        mold: Option<Box<Expr>>,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    /// Function call
    Call { gate: Box<Expr>, args: Vec<Expr> },
    /// Conditional
    If {
        cond: Box<Expr>,
        yes: Box<Expr>,
        no: Box<Expr>,
    },
    /// Pattern match (switch)
    Match {
        subject: Box<Expr>,
        arms: Vec<(Expr, Expr)>,
    },
    /// Trap: loop with `$` as recur point
    Trap(Box<Expr>),
    /// Recur to enclosing trap
    Loop,
    /// Rebind a subject slot and continue
    Rebind {
        slot: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
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
    Eval {
        subject: Box<Expr>,
        formula: Box<Expr>,
    },
    /// Scry — read from the cybergraph (`.^`)
    Scry { mold: Box<Expr>, path: Box<Expr> },
    /// Hint: event-reactive yield
    Hint {
        tag: String,
        selector: Box<Expr>,
        body: Box<Expr>,
    },
    /// Host call: escape to WASM / wGPU / glia
    Host { target: String, args: Box<Expr> },
    /// Arm definition (within a door/core)
    Arm { name: String, body: Box<Expr> },
    /// Door: multi-arm core
    Door(Vec<Expr>),
    /// Method call on a door: obj.method(args)
    MethodCall {
        obj: String,
        method: String,
        args: Vec<Expr>,
    },
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
