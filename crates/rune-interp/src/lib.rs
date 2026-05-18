/// Tree-walking Nox interpreter — the instant-start execution path.
/// Implements all 18 Nox reduction patterns directly over Noun trees.
/// No external dependencies; no build phase; executes the moment parsing is done.
///
/// Nox reduction rules (patterns 0–17):
///
/// Structural (5):
///   [0 n]         axis: n=0 hash-introspect, n=1 identity, n=2 head, n=3 tail,
///                       n=2k → head(axis(k)), n=2k+1 → tail(axis(k))
///   [1 v]         quote: return v unchanged
///   [2 a b]       compose: eval both a,b against subject; reduce(result_a, result_b)
///   [3 a b]       cons: Noun::cell(eval(a), eval(b))
///   [4 test y n]  branch: eval test; if result==0 → eval y else → eval n
///
/// Field arithmetic over F_p where p = GOLDILOCKS_PRIME (6):
///   [5 a b]       add: (eval_a + eval_b) % p
///   [6 a b]       sub: (eval_a - eval_b + p) % p
///   [7 a b]       mul: (eval_a * eval_b) % p
///   [8 a]         inv: modular inverse of eval_a under p (0 maps to 0)
///   [9 a b]       eq:  0 if equal, 1 if not (atoms and cells, structural)
///   [10 a b]      lt:  0 if eval_a < eval_b, 1 otherwise
///
/// Bitwise over u64 (4):
///   [11 a b]      xor: eval_a XOR eval_b
///   [12 a b]      and: eval_a AND eval_b
///   [13 a]        not: bitwise NOT of eval_a
///   [14 a n]      shl: eval_a << eval_n
///
/// Hash + async stubs (3):
///   [15 a]              hash: stub — return eval_a (real hemera hash in M2)
///   [16 [tag sel] body] hint/call: evaluate body; tag/selector are reactive metadata (M5)
///   [17 path]           look: stub for graph scry — return Atom(0) (M2)
///
/// Distribution rule: when formula is [f1 f2] where f1 is itself a cell
/// (not an atom opcode prefix), eval both sides against subject and return
/// [result_f1, result_f2].  This is the "auto-cons" / distribute pattern.

use rune_ast::Noun;

/// Goldilocks prime: 2^64 - 2^32 + 1
const GOLDILOCKS_PRIME: u64 = 0xFFFF_FFFF_0000_0001u64;

#[derive(Debug, Clone, PartialEq)]
pub struct InterpError {
    pub message: String,
}

impl std::fmt::Display for InterpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Evaluate a Nox formula against a subject noun.
///
/// Patterns 2 (compose) and 4 (branch) use a tail-call loop to avoid
/// stack overflow on deeply-nested formulas.
pub fn eval(subject: &Noun, formula: &Noun) -> Result<Noun, InterpError> {
    let mut subj = subject.clone();
    let mut form = formula.clone();

    loop {
        match form {
            // Distribution rule: formula is a cell whose head is also a cell.
            // eval both halves against subject, return [result_h, result_t].
            Noun::Cell(ref fh, ref ft) if matches!(fh.as_ref(), Noun::Cell(..)) => {
                let h = eval(&subj, fh)?;
                let t = eval(&subj, ft)?;
                return Ok(Noun::cell(h, t));
            }

            Noun::Cell(ref op, ref rest) => match op.as_ref() {

                // 0 — axis: navigate the noun tree
                Noun::Atom(0) => return eval_axis(&subj, rest),

                // 1 — quote: return the argument unchanged
                Noun::Atom(1) => return Ok(*rest.clone()),

                // 2 — compose: eval a, eval b, then reduce(result_a, result_b)
                // Tail-call: loop with new subject/formula.
                Noun::Atom(2) => {
                    let (a, b) = pair(rest)?;
                    let new_subj = eval(&subj, &a)?;
                    let new_form = eval(&subj, &b)?;
                    subj = new_subj;
                    form = new_form;
                    continue;
                }

                // 3 — cons: cell(eval(a), eval(b))
                Noun::Atom(3) => {
                    let (a, b) = pair(rest)?;
                    let ha = eval(&subj, &a)?;
                    let ta = eval(&subj, &b)?;
                    return Ok(Noun::cell(ha, ta));
                }

                // 4 — branch: eval test; 0 → yes, nonzero → no
                // Tail-call: loop with chosen branch as new formula.
                Noun::Atom(4) => {
                    let (test_f, ynb) = pair(rest)?;
                    let (yes_f, no_f) = pair(&ynb)?;
                    let test_val = eval(&subj, &test_f)?;
                    let test_n = atom_u64(&test_val, "nox-4 branch: test must be atom")?;
                    form = if test_n == 0 { yes_f } else { no_f };
                    continue;
                }

                // 5 — add: (eval_a + eval_b) % GOLDILOCKS_PRIME
                Noun::Atom(5) => {
                    let (a, b) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-5 add")?;
                    let vb = eval_atom(&subj, &b, "nox-5 add")?;
                    return Ok(Noun::Atom(add_field(va, vb)));
                }

                // 6 — sub: (eval_a - eval_b + p) % p
                Noun::Atom(6) => {
                    let (a, b) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-6 sub")?;
                    let vb = eval_atom(&subj, &b, "nox-6 sub")?;
                    return Ok(Noun::Atom(sub_field(va, vb)));
                }

                // 7 — mul: (eval_a * eval_b) % p
                Noun::Atom(7) => {
                    let (a, b) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-7 mul")?;
                    let vb = eval_atom(&subj, &b, "nox-7 mul")?;
                    return Ok(Noun::Atom(mul_field(va, vb)));
                }

                // 8 — inv: modular inverse under p; 0 maps to 0
                Noun::Atom(8) => {
                    let va = eval_atom(&subj, rest, "nox-8 inv")?;
                    return Ok(Noun::Atom(inv_field(va)));
                }

                // 9 — eq: 0 if equal, 1 if not (structural equality)
                Noun::Atom(9) => {
                    let (a, b) = pair(rest)?;
                    let ra = eval(&subj, &a)?;
                    let rb = eval(&subj, &b)?;
                    return Ok(Noun::Atom(if ra == rb { 0 } else { 1 }));
                }

                // 10 — lt: 0 if eval_a < eval_b, 1 otherwise
                Noun::Atom(10) => {
                    let (a, b) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-10 lt")?;
                    let vb = eval_atom(&subj, &b, "nox-10 lt")?;
                    return Ok(Noun::Atom(if va < vb { 0 } else { 1 }));
                }

                // 11 — xor
                Noun::Atom(11) => {
                    let (a, b) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-11 xor")?;
                    let vb = eval_atom(&subj, &b, "nox-11 xor")?;
                    return Ok(Noun::Atom(va ^ vb));
                }

                // 12 — and
                Noun::Atom(12) => {
                    let (a, b) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-12 and")?;
                    let vb = eval_atom(&subj, &b, "nox-12 and")?;
                    return Ok(Noun::Atom(va & vb));
                }

                // 13 — not (bitwise NOT, u64)
                Noun::Atom(13) => {
                    let va = eval_atom(&subj, rest, "nox-13 not")?;
                    return Ok(Noun::Atom(!va));
                }

                // 14 — shl: eval_a << eval_n
                Noun::Atom(14) => {
                    let (a, n) = pair(rest)?;
                    let va = eval_atom(&subj, &a, "nox-14 shl")?;
                    let vn = eval_atom(&subj, &n, "nox-14 shl")?;
                    return Ok(Noun::Atom(va << (vn & 63)));
                }

                // 15 — hash: stub; real hemera hash lands in M2
                Noun::Atom(15) => {
                    let r = eval(&subj, rest)?;
                    return Ok(r);
                }

                // 16 — call/hint: evaluate body (tag/selector are reactive metadata, ignored at M5)
                Noun::Atom(16) => {
                    // rest = [[tag selector] body]
                    let (_hint_meta, body) = pair(rest)?;
                    form = body;
                    continue; // tail-call: eval body against current subject
                }

                // 17 — look: stub for graph scry (M2)
                Noun::Atom(17) => {
                    return Ok(Noun::Atom(0));
                }

                _ => return Err(err(&format!("nox: unrecognized opcode {:?}", op))),
            },

            Noun::Atom(_) => return Err(err("nox: atom is not a valid formula")),
        }
    }
}

/// Navigate the noun tree by axis address.
/// addr=0 → hash introspection stub (Atom(0) for cells, atom value for atoms)
/// addr=1 → identity
/// addr=2k → head(axis(noun, k))
/// addr=2k+1 → tail(axis(noun, k))
pub fn axis(noun: &Noun, addr: u64) -> Result<Noun, InterpError> {
    match addr {
        0 => match noun {
            Noun::Cell(..) => Ok(Noun::Atom(0)),
            Noun::Atom(v)  => Ok(Noun::Atom(*v)),
        },
        1 => Ok(noun.clone()),
        2 => match noun {
            Noun::Cell(h, _) => Ok(*h.clone()),
            _ => Err(err("nox-0 axis: /2 on atom")),
        },
        3 => match noun {
            Noun::Cell(_, t) => Ok(*t.clone()),
            _ => Err(err("nox-0 axis: /3 on atom")),
        },
        n => {
            let parent = axis(noun, n / 2)?;
            axis(&parent, 2 + (n % 2))
        }
    }
}

// ── internal helpers ──────────────────────────────────────────────────────────

fn eval_axis(subj: &Noun, addr_noun: &Noun) -> Result<Noun, InterpError> {
    let n = match addr_noun {
        Noun::Atom(n) => *n,
        _ => return Err(err("nox-0 axis: address must be an atom")),
    };
    axis(subj, n)
}

/// Destructure a noun into (head, tail); error if atom.
fn pair(noun: &Noun) -> Result<(Noun, Noun), InterpError> {
    match noun {
        Noun::Cell(h, t) => Ok((*h.clone(), *t.clone())),
        _ => Err(err("nox: expected cell")),
    }
}

/// Eval a sub-formula and extract the u64 atom value.
fn eval_atom(subj: &Noun, formula: &Noun, ctx: &str) -> Result<u64, InterpError> {
    let r = eval(subj, formula)?;
    atom_u64(&r, ctx)
}

/// Extract u64 from an atom noun; error on cell.
fn atom_u64(noun: &Noun, ctx: &str) -> Result<u64, InterpError> {
    match noun {
        Noun::Atom(n) => Ok(*n),
        _ => Err(err(&format!("{}: expected atom, got cell", ctx))),
    }
}

// ── Goldilocks field arithmetic ───────────────────────────────────────────────

fn add_field(a: u64, b: u64) -> u64 {
    // Use u128 to avoid overflow before mod.
    ((a as u128 + b as u128) % GOLDILOCKS_PRIME as u128) as u64
}

fn sub_field(a: u64, b: u64) -> u64 {
    ((a as u128 + GOLDILOCKS_PRIME as u128 - b as u128) % GOLDILOCKS_PRIME as u128) as u64
}

fn mul_field(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) % GOLDILOCKS_PRIME as u128) as u64
}

/// Extended Euclidean algorithm for modular inverse.
/// Returns 0 when a == 0 (no inverse for the additive identity).
fn inv_field(a: u64) -> u64 {
    if a == 0 {
        return 0;
    }
    // Fermat: a^(p-2) mod p.  We implement binary exponentiation.
    let p = GOLDILOCKS_PRIME;
    let exp = p - 2;
    let mut base = a as u128;
    let mut result: u128 = 1;
    let mut e = exp;
    let m = p as u128;
    while e > 0 {
        if e & 1 == 1 {
            result = result * base % m;
        }
        base = base * base % m;
        e >>= 1;
    }
    result as u64
}

fn err(msg: &str) -> InterpError {
    InterpError { message: msg.to_string() }
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn a(n: u64) -> Noun { Noun::Atom(n) }
    fn c(h: Noun, t: Noun) -> Noun { Noun::cell(h, t) }

    // ── pattern 0: axis ───────────────────────────────────────────────────────

    #[test]
    fn axis_identity() {
        // [0 1] against atom 42 → 42
        assert_eq!(eval(&a(42), &c(a(0), a(1))).unwrap(), a(42));
    }

    #[test]
    fn axis_head() {
        let s = c(a(1), a(2));
        assert_eq!(eval(&s, &c(a(0), a(2))).unwrap(), a(1));
    }

    #[test]
    fn axis_tail() {
        let s = c(a(1), a(2));
        assert_eq!(eval(&s, &c(a(0), a(3))).unwrap(), a(2));
    }

    #[test]
    fn axis_deep_6() {
        // s = [[1 2] [3 4]]  axis 6 = head(tail) = 3
        let s = c(c(a(1), a(2)), c(a(3), a(4)));
        assert_eq!(eval(&s, &c(a(0), a(6))).unwrap(), a(3));
    }

    #[test]
    fn axis_deep_7() {
        // axis 7 = tail(tail) = 4
        let s = c(c(a(1), a(2)), c(a(3), a(4)));
        assert_eq!(eval(&s, &c(a(0), a(7))).unwrap(), a(4));
    }

    #[test]
    fn axis_deep_14() {
        // s = [[[1 2] [3 4]] [[5 6] [7 8]]]
        // axis 14 binary = 1110 → strip leading 1 → bits 1,1,0 → tail,tail,head
        // tail(s) = [[5 6] [7 8]], tail(that) = [7 8], head(that) = 7
        let s = c(c(c(a(1), a(2)), c(a(3), a(4))), c(c(a(5), a(6)), c(a(7), a(8))));
        assert_eq!(eval(&s, &c(a(0), a(14))).unwrap(), a(7));
    }

    #[test]
    fn axis_zero_hash_cell_stub() {
        let s = c(a(10), a(20));
        assert_eq!(eval(&s, &c(a(0), a(0))).unwrap(), a(0));
    }

    #[test]
    fn axis_zero_hash_atom_stub() {
        // atom → returns atom value itself
        assert_eq!(eval(&a(99), &c(a(0), a(0))).unwrap(), a(99));
    }

    // ── pattern 1: quote ──────────────────────────────────────────────────────

    #[test]
    fn quote_atom() {
        assert_eq!(eval(&a(0), &c(a(1), a(42))).unwrap(), a(42));
    }

    #[test]
    fn quote_cell() {
        let v = c(a(1), a(2));
        assert_eq!(eval(&a(0), &c(a(1), v.clone())).unwrap(), v);
    }

    // ── pattern 2: compose ────────────────────────────────────────────────────

    #[test]
    fn compose_identity_chain() {
        // [2 a b]: new_subj = eval(s,a), new_form = eval(s,b), then eval(new_subj, new_form)
        // s = [99 0], a = [0 1] (identity → s), b = [1 [0 2]] (quote of axis-head formula)
        // new_subj = s = [99 0]
        // new_form = [0 2]
        // eval([99 0], [0 2]) = head([99 0]) = 99
        let s = c(a(99), a(0));
        let id = c(a(0), a(1));
        let quote_head = c(a(1), c(a(0), a(2)));
        let formula = c(a(2), c(id, quote_head));
        assert_eq!(eval(&s, &formula).unwrap(), a(99));
    }

    #[test]
    fn compose_quote_then_identity() {
        // s=5, formula=[2 [1 [0 1]] [0 1]]
        // step1: eval [1 [0 1]] against 5 → [0 1] (the literal formula)
        // step2: eval [0 1] against 5 → 5 (but wait — new_subj = eval(s,[1,[0,1]]) = [0,1])
        // Actually: compose = reduce(eval(s,a), eval(s,b))
        // a=[0 1], eval(5,[0 1])=5  →  new_subj=5
        // b=[1 [0 1]], eval(5,[1 [0 1]])=[0 1]  →  new_form=[0 1]
        // reduce(5, [0 1]) = 5
        let s = a(5);
        let id = c(a(0), a(1));
        let quote_id = c(a(1), id.clone());
        let formula = c(a(2), c(id, quote_id));
        assert_eq!(eval(&s, &formula).unwrap(), s);
    }

    // ── pattern 3: cons ───────────────────────────────────────────────────────

    #[test]
    fn cons_two_literals() {
        // [3 [1 1] [1 2]] against 0 → [1 2]
        let formula = c(a(3), c(c(a(1), a(1)), c(a(1), a(2))));
        assert_eq!(eval(&a(0), &formula).unwrap(), c(a(1), a(2)));
    }

    // ── pattern 4: branch ─────────────────────────────────────────────────────

    #[test]
    fn branch_zero_takes_yes() {
        // [4 [1 0] [1 99] [1 0]] against 0 → test=0 → yes → 99
        let formula = c(a(4), c(c(a(1), a(0)), c(c(a(1), a(99)), c(a(1), a(0)))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(99));
    }

    #[test]
    fn branch_nonzero_takes_no() {
        // [4 [1 1] [1 99] [1 77]] against 0 → test=1 (nonzero) → no → 77
        let formula = c(a(4), c(c(a(1), a(1)), c(c(a(1), a(99)), c(a(1), a(77)))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(77));
    }

    // ── pattern 5: add ────────────────────────────────────────────────────────

    #[test]
    fn add_basic() {
        // [5 [1 5] [1 3]] → 8
        let formula = c(a(5), c(c(a(1), a(5)), c(a(1), a(3))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(8));
    }

    #[test]
    fn add_wraps_goldilocks() {
        // (p - 1) + 1 = 0 mod p
        let p_minus_1 = GOLDILOCKS_PRIME - 1;
        let formula = c(a(5), c(c(a(1), a(p_minus_1)), c(a(1), a(1))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    // ── pattern 6: sub ────────────────────────────────────────────────────────

    #[test]
    fn sub_basic() {
        // [6 [1 10] [1 3]] → 7
        let formula = c(a(6), c(c(a(1), a(10)), c(a(1), a(3))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(7));
    }

    #[test]
    fn sub_wraps_around() {
        // 0 - 1 = p - 1 mod p
        let formula = c(a(6), c(c(a(1), a(0)), c(a(1), a(1))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(GOLDILOCKS_PRIME - 1));
    }

    // ── pattern 7: mul ────────────────────────────────────────────────────────

    #[test]
    fn mul_basic() {
        // [7 [1 6] [1 7]] → 42
        let formula = c(a(7), c(c(a(1), a(6)), c(a(1), a(7))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(42));
    }

    // ── pattern 8: inv ────────────────────────────────────────────────────────

    #[test]
    fn inv_nonzero() {
        // [8 [1 2]] → modular inverse of 2; 2 * inv(2) = 1 mod p
        let formula = c(a(8), c(a(1), a(2)));
        let result = eval(&a(0), &formula).unwrap();
        let Noun::Atom(r) = result else { panic!("expected atom") };
        assert_eq!(mul_field(2, r), 1);
    }

    #[test]
    fn inv_zero() {
        // [8 [1 0]] → 0 (no inverse for 0)
        let formula = c(a(8), c(a(1), a(0)));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    // ── pattern 9: eq ─────────────────────────────────────────────────────────

    #[test]
    fn eq_atoms_equal() {
        // [9 [1 42] [1 42]] → 0
        let formula = c(a(9), c(c(a(1), a(42)), c(a(1), a(42))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    #[test]
    fn eq_atoms_not_equal() {
        // [9 [1 1] [1 2]] → 1
        let formula = c(a(9), c(c(a(1), a(1)), c(a(1), a(2))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(1));
    }

    #[test]
    fn eq_cells_equal() {
        // [9 [1 [1 2]] [1 [1 2]]] → 0
        let cell_val = c(a(1), a(2));
        let formula = c(a(9), c(c(a(1), cell_val.clone()), c(a(1), cell_val)));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    #[test]
    fn eq_cells_not_equal() {
        // [9 [1 [1 2]] [1 [1 3]]] → 1
        let formula = c(a(9), c(c(a(1), c(a(1), a(2))), c(a(1), c(a(1), a(3)))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(1));
    }

    // ── pattern 10: lt ────────────────────────────────────────────────────────

    #[test]
    fn lt_true() {
        // [10 [1 3] [1 7]] → 0 (3 < 7)
        let formula = c(a(10), c(c(a(1), a(3)), c(a(1), a(7))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    #[test]
    fn lt_false_equal() {
        // [10 [1 7] [1 7]] → 1 (not strictly less)
        let formula = c(a(10), c(c(a(1), a(7)), c(a(1), a(7))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(1));
    }

    #[test]
    fn lt_false_greater() {
        // [10 [1 9] [1 3]] → 1
        let formula = c(a(10), c(c(a(1), a(9)), c(a(1), a(3))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(1));
    }

    // ── pattern 11: xor ───────────────────────────────────────────────────────

    #[test]
    fn xor_basic() {
        // [11 [1 0b1010] [1 0b1100]] → 0b0110 = 6
        let formula = c(a(11), c(c(a(1), a(0b1010)), c(a(1), a(0b1100))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0b0110));
    }

    // ── pattern 12: and ───────────────────────────────────────────────────────

    #[test]
    fn and_basic() {
        let formula = c(a(12), c(c(a(1), a(0b1100)), c(a(1), a(0b1010))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0b1000));
    }

    // ── pattern 13: not ───────────────────────────────────────────────────────

    #[test]
    fn not_basic() {
        let formula = c(a(13), c(a(1), a(0)));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(!0u64));
    }

    // ── pattern 14: shl ───────────────────────────────────────────────────────

    #[test]
    fn shl_basic() {
        // [14 [1 1] [1 3]] → 1 << 3 = 8
        let formula = c(a(14), c(c(a(1), a(1)), c(a(1), a(3))));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(8));
    }

    // ── pattern 15: hash stub ─────────────────────────────────────────────────

    #[test]
    fn hash_stub_returns_value() {
        let formula = c(a(15), c(a(1), a(55)));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(55));
    }

    // ── pattern 16: hint/call ─────────────────────────────────────────────────

    #[test]
    fn hint_evaluates_body() {
        // [16 [[1 42] [1 0]] [1 99]] → evaluate body [1 99] → 99
        // Format: [16 [tag selector] body] where tag=[1 42], selector=[1 0], body=[1 99]
        let tag = c(a(1), a(42));
        let selector = c(a(1), a(0));
        let hint_meta = c(tag, selector);
        let body = c(a(1), a(99));
        let formula = c(a(16), c(hint_meta, body));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(99));
    }

    #[test]
    fn hint_evaluates_arithmetic_body() {
        // [16 [[1 0] [1 0]] [5 [1 3] [1 4]]] → eval [5 [1 3] [1 4]] → add(3,4) = 7
        let hint_meta = c(c(a(1), a(0)), c(a(1), a(0)));
        let body = c(a(5), c(c(a(1), a(3)), c(a(1), a(4))));
        let formula = c(a(16), c(hint_meta, body));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(7));
    }

    #[test]
    fn host_call_returns_zero() {
        // [16 [[1 host-tag] args] [1 0]] → body = [1 0] → 0
        let hint_meta = c(c(a(1), a(99)), c(a(1), a(0)));
        let body = c(a(1), a(0)); // [1 0] = quoted zero
        let formula = c(a(16), c(hint_meta, body));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    // ── pattern 17: look stub ─────────────────────────────────────────────────

    #[test]
    fn look_stub_returns_zero() {
        let formula = c(a(17), c(a(1), a(0)));
        assert_eq!(eval(&a(0), &formula).unwrap(), a(0));
    }

    // ── distribution rule ─────────────────────────────────────────────────────

    #[test]
    fn distribution_cell_formula() {
        // formula = [[1 10] [1 20]]  both halves are cells (not atom opcodes)
        // → [eval([1 10]), eval([1 20])] = [10, 20]
        let formula = c(c(a(1), a(10)), c(a(1), a(20)));
        assert_eq!(eval(&a(0), &formula).unwrap(), c(a(10), a(20)));
    }

    #[test]
    fn distribution_nested() {
        // formula = [[1 1] [1 2]]  against 0 → [1 2]
        let formula = c(c(a(1), a(1)), c(a(1), a(2)));
        assert_eq!(eval(&a(0), &formula).unwrap(), c(a(1), a(2)));
    }

    // ── compose chaining ──────────────────────────────────────────────────────

    #[test]
    fn compose_chaining_via_quote() {
        // s=42, formula=[2 [0 1] [1 [0 1]]]
        // eval(s, [0 1]) = 42 = new_subj
        // eval(s, [1 [0 1]]) = [0 1] = new_form
        // eval(42, [0 1]) = 42
        let s = a(42);
        let formula = c(a(2), c(c(a(0), a(1)), c(a(1), c(a(0), a(1)))));
        assert_eq!(eval(&s, &formula).unwrap(), a(42));
    }

    // ── axis public function ───────────────────────────────────────────────────

    #[test]
    fn pub_axis_fn() {
        let s = c(c(a(1), a(2)), c(a(3), a(4)));
        assert_eq!(axis(&s, 1).unwrap(), s);
        assert_eq!(axis(&s, 2).unwrap(), c(a(1), a(2)));
        assert_eq!(axis(&s, 3).unwrap(), c(a(3), a(4)));
        assert_eq!(axis(&s, 6).unwrap(), a(3));
        assert_eq!(axis(&s, 7).unwrap(), a(4));
    }

    // ── field arithmetic helpers ──────────────────────────────────────────────

    #[test]
    fn field_add_sub_inverse() {
        let x = 12345678u64;
        let y = 87654321u64;
        assert_eq!(sub_field(add_field(x, y), y), x);
    }

    #[test]
    fn field_mul_inv() {
        let x = 7u64;
        let ix = inv_field(x);
        assert_eq!(mul_field(x, ix), 1);
    }
}
