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
use rune_ast::{Address, Expr, Noun, act, tag};
use cyber_hemera as hemera;

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

/// An arm definition inside a door (for method call inlining).
#[derive(Clone)]
struct ArmDef {
    name: String,
    params: Vec<String>,
    body_expr: Expr,
}

/// A door definition stored for method-call inlining.
#[derive(Clone)]
struct DoorDef {
    door_name: String,
    arms: Vec<ArmDef>,
}

#[derive(Default, Clone)]
struct Env {
    /// Bound names, newest at index 0.
    stack: Vec<String>,
    /// Known gate definitions, for inlining at call sites.
    gates: Vec<GateDef>,
    /// Known door definitions, for method-call inlining.
    doors: Vec<DoorDef>,
    /// True when compiling inside a Trap body (enables Loop/recur).
    in_trap: bool,
}

impl Env {
    /// Return a new Env with `name` pushed to the front (depth 0).
    /// `in_trap` is preserved by clone, so trap context propagates into nested exprs.
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

    /// Push `name` as a stack binding AND record a door definition for method-call inlining.
    fn push_door(&self, name: &str, arms: Vec<ArmDef>) -> Self {
        let mut e = self.push(name);
        e.doors.push(DoorDef { door_name: name.to_string(), arms });
        e
    }

    /// Find a door definition by name (last one wins).
    fn find_door(&self, name: &str) -> Option<&DoorDef> {
        self.doors.iter().rev().find(|d| d.door_name == name)
    }

    /// Look up `name`; returns Some(axis) where axis = stack_axis(index).
    fn lookup(&self, name: &str) -> Option<u64> {
        self.stack
            .iter()
            .position(|n| n == name)
            .map(stack_axis)
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

        // Text → quote of a `tape` noun (null-terminated byte list).
        Expr::Text(s) => Ok(Noun::cell(Noun::Atom(1), tape_noun(&s))),

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
            let is_door = matches!(*value, Expr::Door(_));

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
            } else if is_door {
                // Extract arm definitions for method dispatch
                let door_arms = match *value {
                    Expr::Door(arm_exprs) => arm_exprs,
                    _ => unreachable!(),
                };
                let mut arm_defs = Vec::new();
                for arm_expr in &door_arms {
                    if let Expr::Arm { name: arm_name, body: arm_body } = arm_expr {
                        let (params, body_expr) = match arm_body.as_ref() {
                            Expr::Gate { sample, body: gb } => (extract_params(sample), *gb.clone()),
                            other => (vec![], other.clone()),
                        };
                        arm_defs.push(ArmDef { name: arm_name.clone(), params, body_expr });
                    }
                }
                // Lower the door formula (battery + payload)
                let door_formula = lower_in(Expr::Door(door_arms), env)?;
                let env2 = env.push_door(&name, arm_defs);
                let extend = op3(door_formula, axis(1));
                let bf = lower_in(*body, &env2)?;
                let quoted_body = Noun::cell(Noun::Atom(1), bf);
                return Ok(op2(extend, quoted_body));
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
            // host act: [16 [act::HOST args-f] [0 2]] — perform, return spliced result
            let args_f = lower_in(*args, env)?;
            Ok(act_formula(act::HOST, args_f))
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
                    // prysm element constructors — lower to chunk-noun `[tag …]`.
                    // Each evaluates to a value the prysm bridge decodes to a Chunk.
                    "text"   => return prysm_unary(tag::TEXT,  &args, env),
                    "anno"   => return prysm_unary(tag::ANNO,  &args, env),
                    "error"  => return prysm_unary(tag::ERROR, &args, env),
                    "log"    => return prysm_unary(tag::LOG,   &args, env),
                    "button" => return prysm_button(&args, env),
                    "col"    => return prysm_col(&args, env),
                    // act builtins — request a world-touch through the host/ward.
                    // `emit(x)` → [16 [act args-f] [0 2]] → perform, return result.
                    "emit"      => return act_unary(act::EMIT,      &args, env),
                    "query"     => return act_unary(act::QUERY,     &args, env),
                    "link"      => return act_unary(act::LINK,      &args, env),
                    "seal"      => return act_unary(act::SEAL,      &args, env),
                    "subscribe" => return act_unary(act::SUBSCRIBE, &args, env),
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
        // Trap — self-referential loop body with tail-call recur.
        //
        // Push "$" so all outer vars shift by 1 (axis-shift matches cons-shift).
        // Set in_trap = true so nested Loop nodes know they are in a trap.
        //
        // Emitted formula: [2 [3 [1 body_f] [0 1]] [1 body_f]]
        //   Step 1 (new_subj): eval [3 [1 body_f] [0 1]] against s
        //                    = [body_f  s]   (cons body_f onto s)
        //   Step 2 (new_form): eval [1 body_f] against s = body_f (quoted)
        //   Step 3: eval body_f against [body_f  s]
        //           → body_f lives at axis 2 of the new subject; Loop uses [0 2].
        // ------------------------------------------------------------------
        Expr::Trap(body) => {
            let trap_env = Env { in_trap: true, ..env.push("$") };
            let body_f = lower_in(*body, &trap_env)?;
            // [3 [1 body_f] [0 1]]  — new_subj formula: cons body_f onto current subject
            let setup = op3(Noun::cell(Noun::Atom(1), body_f.clone()), axis(1));
            // [1 body_f]            — new_form formula: the quoted body itself
            let quoted_body = Noun::cell(Noun::Atom(1), body_f);
            Ok(op2(setup, quoted_body))
        }

        // ------------------------------------------------------------------
        // Loop (recur) — tail-call back to the enclosing Trap.
        //
        // `$` may not be at axis 2 when inner `let` bindings inside the trap
        // body have extended the subject.  Look up `$` dynamically:
        //   body_f_axis  = stack_axis(depth_of_$)  = where body_f lives now
        //   trap_subj_axis = body_f_axis / 2        = the trap subject itself
        //
        // [2 [0 trap_subj_axis] [0 body_f_axis]]:
        //   new_subj = trap subject (strips any inner let extensions)
        //   new_form = body_f (head of trap subject)
        //   → restarts body_f against the clean trap subject
        //
        // Special case: no inner lets → body_f_axis=2, trap_subj_axis=1
        //   → [2 [0 1] [0 2]] (same as before, unchanged for existing tests).
        // ------------------------------------------------------------------
        Expr::Loop => {
            if env.in_trap {
                let body_f_axis = env.lookup("$")
                    .ok_or_else(|| lower_err("internal: '$' missing from trap env"))?;
                let trap_subj_axis = body_f_axis / 2;
                Ok(op2(axis(trap_subj_axis), axis(body_f_axis)))
            } else {
                Err(lower_err("recur ($) outside trap"))
            }
        }

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
        // Door — multi-arm core: [battery  payload].
        //
        // Battery = right-nested cons of quoted arm formulas, nil-terminated:
        //   [3 [1 arm0_f] [3 [1 arm1_f] [1 0]]]
        // Door formula = [3 battery_f [0 1]]
        //   → produces [battery  current_subject] at runtime.
        //
        // Arm axis in the door noun:
        //   arm 0 → axis 4 (head of battery = head of door head)
        //   arm 1 → axis 10, arm 2 → axis 22, …
        // ------------------------------------------------------------------
        Expr::Door(arms) => {
            if arms.is_empty() {
                return Err(lower_err("door with no arms"));
            }
            // Build battery from right to left; nil = [1 0]
            let mut battery_f: Noun = Noun::cell(Noun::Atom(1), Noun::Atom(0));
            for arm in arms.into_iter().rev() {
                let arm_body_f = match arm {
                    Expr::Arm { body, .. } => lower_in(*body, env)?,
                    other => lower_in(other, env)?,
                };
                // Store arm formula quoted so it's a value in the battery cons-list
                let quoted_arm = Noun::cell(Noun::Atom(1), arm_body_f);
                battery_f = op3(quoted_arm, battery_f);
            }
            // Door = cons battery onto current subject
            Ok(op3(battery_f, axis(1)))
        }

        // ------------------------------------------------------------------
        // Arm — standalone (should normally appear only inside Door).
        // ------------------------------------------------------------------
        Expr::Arm { name: _, body } => {
            lower_in(*body, env)
        }

        // ------------------------------------------------------------------
        // Rebind { slot, value, body } — update a let-bound slot in-place.
        //
        // Rebuilds the subject with the named slot replaced by value_f,
        // keeping all other slots intact.  Subject structure is preserved
        // (body_f is still at axis 2 of the trap subject), enabling Loop.
        //
        // Formula emitted: [2 new_subj_f [1 body_f]]
        //   where new_subj_f replaces slot at `depth` and keeps all others.
        //
        // depth 0 (axis 2): new_subj_f = [3 value_f [0 3]]
        // depth 1 (axis 6): new_subj_f = [3 [0 2] [3 value_f [0 7]]]
        // depth 2 (axis 14): new_subj_f = [3 [0 2] [3 [0 6] [3 value_f [0 15]]]]
        // ------------------------------------------------------------------
        Expr::Rebind { slot, value, body } => {
            let depth = env.stack.iter().position(|n| n == &slot)
                .ok_or_else(|| lower_err(format!("rebind: unbound name '{slot}'")))?;
            let value_f = lower_in(*value, env)?;
            let new_subj_f = rebind_subj_formula(depth, value_f)?;
            // body compiled against same env: slot is still at same depth
            let body_f = lower_in(*body, env)?;
            let quoted_body = Noun::cell(Noun::Atom(1), body_f);
            Ok(op2(new_subj_f, quoted_body))
        }

        // ------------------------------------------------------------------
        // MethodCall — inline a door arm at the call site.
        //
        // Looks up the door definition, finds the named arm, then inlines
        // the arm body with args cons-ed onto the subject (same as gate call
        // inlining).  Formula: [2 chain [1 body_f]]
        // ------------------------------------------------------------------
        Expr::MethodCall { obj, method, args } => {
            // Find the door definition to get the arm body for inlining.
            // Clone everything needed before any mutable borrow of env.
            let (arm_params, arm_body) = {
                let door_def = env.find_door(&obj)
                    .ok_or_else(|| lower_err(format!("no door named '{obj}'")))?;
                let arm = door_def.arms.iter().find(|a| a.name == method)
                    .ok_or_else(|| lower_err(format!("door '{obj}' has no arm '{method}'")))?;
                if args.len() != arm.params.len() {
                    return Err(lower_err(format!(
                        "arm '{}.{}' takes {} args, got {}",
                        obj, method, arm.params.len(), args.len()
                    )));
                }
                (arm.params.clone(), arm.body_expr.clone())
            };

            // Build body env: push params in reverse order onto call-site env.
            let mut call_body_env = env.clone();
            for p in arm_params.iter().rev() {
                call_body_env = call_body_env.push(p);
            }
            let body_f = lower_in(arm_body, &call_body_env)?;

            // Build args chain: [3 a1-f [3 a2-f [... [0 1]]]]
            let mut chain = axis(1);
            for arg in args.into_iter().rev() {
                let af = lower_in(arg, env)?;
                chain = op3(af, chain);
            }
            let quoted_body = Noun::cell(Noun::Atom(1), body_f);
            Ok(op2(chain, quoted_body))
        }

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

/// Build a formula that reconstructs the subject with the slot at `depth` replaced by `value_f`.
///
/// The subject is a right-nested cons list: [v0 [v1 [v2 ... rest]]].
/// depth 0 value is at axis 2; depth 1 at axis 6; depth d at axis (1<<(d+2))-2.
/// The tail after depth d is at axis (1<<(d+2))-1 = stack_axis(d) + 1.
///
/// Pattern:
///   depth 0: [3 value_f [0 3]]
///   depth 1: [3 [0 2] [3 value_f [0 7]]]
///   depth 2: [3 [0 2] [3 [0 6] [3 value_f [0 15]]]]
fn rebind_subj_formula(depth: usize, value_f: Noun) -> Result<Noun, LowerError> {
    if depth > 7 {
        return Err(lower_err("rebind depth > 7 not supported"));
    }
    // tail_axis at depth d = stack_axis(d) + 1 = (1<<(d+2)) - 1
    let tail_axis = (1u64 << (depth + 2)) - 1;
    // Innermost piece: [3 value_f [0 tail_axis]]
    let mut formula = op3(value_f, axis(tail_axis));
    // Wrap with head-preserving ops for depths 0..depth (in reverse)
    // e.g. depth=2: wrap with [3 [0 6] …] then [3 [0 2] …]
    for d in (0..depth).rev() {
        let head_axis = stack_axis(d);
        formula = op3(axis(head_axis), formula);
    }
    Ok(formula)
}

fn lower_err(msg: impl Into<String>) -> LowerError {
    LowerError { message: msg.into() }
}

// ---------------------------------------------------------------------------
// prysm element lowering — `text("hi")`, `error("boom")`, `button(..)`,
// `col(..)` lower to chunk-noun formulas `[3 [1 tag] payload-f]`, which
// evaluate to `[tag <payload-value>]`. `rune-prysm::noun_to_chunks` decodes.
// ---------------------------------------------------------------------------

/// `[1 <tape>]`-ready tape noun: a null-terminated cons list of byte atoms.
/// "" → `0`; "hi" → `[104 [105 0]]`.
fn tape_noun(s: &str) -> Noun {
    let mut n = Noun::Atom(0);
    for &b in s.as_bytes().iter().rev() {
        n = Noun::cell(Noun::Atom(b as u64), n);
    }
    n
}

/// A one-payload prysm element: `[3 [1 tag] payload-f]` → `[tag <payload>]`.
fn prysm_unary(t: u64, args: &[Expr], env: &Env) -> Result<Noun, LowerError> {
    if args.len() != 1 {
        return Err(lower_err(format!("prysm tag {t} requires 1 argument, got {}", args.len())));
    }
    let body = lower_in(args[0].clone(), env)?;
    Ok(op3(quote(t), body))
}

/// `button(label, target)` → `[BUTTON [label target]]`.
fn prysm_button(args: &[Expr], env: &Env) -> Result<Noun, LowerError> {
    if args.len() != 2 {
        return Err(lower_err(format!("button requires 2 arguments, got {}", args.len())));
    }
    let label  = lower_in(args[0].clone(), env)?;
    let target = lower_in(args[1].clone(), env)?;
    Ok(op3(quote(tag::BUTTON), op3(label, target)))
}

/// An act: `[16 [act-tag args-f] [0 2]]` — perform the act, then return its
/// result (spliced by the interpreter at axis 2 = subject head).
fn act_formula(act_tag: u64, args_f: Noun) -> Noun {
    Noun::cell(
        Noun::Atom(16),
        Noun::cell(
            Noun::cell(Noun::Atom(act_tag), args_f),
            axis(2),
        ),
    )
}

/// A one-argument act builtin: `emit(x)`, `query(x)`, `link(x)`, …
fn act_unary(act_tag: u64, args: &[Expr], env: &Env) -> Result<Noun, LowerError> {
    if args.len() != 1 {
        return Err(lower_err(format!("act {act_tag:#x} requires 1 argument, got {}", args.len())));
    }
    let args_f = lower_in(args[0].clone(), env)?;
    Ok(act_formula(act_tag, args_f))
}

/// `col(e1, .., en)` → `[LIST e1 [e2 [.. 0]]]` — a null-terminated element list.
fn prysm_col(args: &[Expr], env: &Env) -> Result<Noun, LowerError> {
    let mut list_f = quote(0); // null terminator value
    for a in args.iter().rev() {
        let ef = lower_in(a.clone(), env)?;
        list_f = op3(ef, list_f);
    }
    Ok(op3(quote(tag::LIST), list_f))
}

// ---------------------------------------------------------------------------
// Name→atom encoding via Poseidon2 (hemera).
// Takes the first 8 bytes of the 32-byte digest as a little-endian u64.
// ---------------------------------------------------------------------------
fn string_to_atom(s: &str) -> u64 {
    let digest = hemera::hash(s.as_bytes());
    u64::from_le_bytes(digest.as_bytes()[..8].try_into().unwrap())
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

    #[test]
    fn trap_executes_body() {
        // loop { 42 }  → (executes once, no recur) → 42
        let expr = Expr::Trap(Box::new(Expr::Atom(42)));
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(42));
    }

    #[test]
    fn trap_loop_recurs() {
        // let counter = 0; loop { if counter == 3 { counter } else { rebind counter = counter+1; continue } }
        // Counts from 0 to 3, returning 3.
        let loop_body = Expr::If {
            cond: Box::new(Expr::Eq(
                Box::new(Expr::Sym("counter".into())),
                Box::new(Expr::Atom(3)),
            )),
            yes: Box::new(Expr::Sym("counter".into())),
            no: Box::new(Expr::Rebind {
                slot: "counter".into(),
                value: Box::new(Expr::Call {
                    gate: Box::new(Expr::Sym("add".into())),
                    args: vec![Expr::Sym("counter".into()), Expr::Atom(1)],
                }),
                body: Box::new(Expr::Loop),
            }),
        };
        let expr = Expr::Let {
            name: "counter".into(),
            mold: None,
            value: Box::new(Expr::Atom(0)),
            body: Box::new(Expr::Trap(Box::new(loop_body))),
        };
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(3));
    }

    #[test]
    fn rebind_depth_0() {
        // let x = 5; rebind x = 10; x → 10
        let expr = Expr::Let {
            name: "x".into(),
            mold: None,
            value: Box::new(Expr::Atom(5)),
            body: Box::new(Expr::Rebind {
                slot: "x".into(),
                value: Box::new(Expr::Atom(10)),
                body: Box::new(Expr::Sym("x".into())),
            }),
        };
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(10));
    }

    #[test]
    fn rebind_depth_1() {
        // let a = 1; let b = 2; rebind a = 99; a + b → 101
        let expr = Expr::Let {
            name: "a".into(),
            mold: None,
            value: Box::new(Expr::Atom(1)),
            body: Box::new(Expr::Let {
                name: "b".into(),
                mold: None,
                value: Box::new(Expr::Atom(2)),
                body: Box::new(Expr::Rebind {
                    slot: "a".into(),
                    value: Box::new(Expr::Atom(99)),
                    body: Box::new(Expr::Call {
                        gate: Box::new(Expr::Sym("add".into())),
                        args: vec![Expr::Sym("a".into()), Expr::Sym("b".into())],
                    }),
                }),
            }),
        };
        let f = lower(expr).unwrap();
        let result = rune_interp::eval(&Noun::Atom(0), &f).unwrap();
        assert_eq!(result, Noun::Atom(101));
    }
}
