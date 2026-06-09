# rune implementation plan

rune = open-computation language for the cyber soft3 stack.
Full spec: `~/cyber/cyber/root/rune.md`.

## architecture

```
cybermark sigils (13)
    ↓  address + inline computation
rune inline  (sigil[expr] — reactive prose, living documents)
    ↓  + block grammar
rune         (gates, doors, kernels, agents — full language)
    ↓  pure subset
trident      (consensus-critical, proof-mandatory, frozen)
    ↓  jets
Rs           (native-speed primitives)
```

Two registers, one AST, one Nox target:
- `rust` — classic, Rust-familiar surface
- `rune` — pure, Hoon-style sigil surface

Three proof tiers:
- pure: unconditional
- reactive (hint): conditional on event log
- host: conditional on host results (only genuine trust boundary)

## crate map

```
rune-ast       Expr + Noun — shared representation
rune-lex       tokenizer (both registers share one lexer)
rune-parse     classic register parser [pure: M4]
rune-lower     Expr → Nox noun
rune-interp    tree-walking Nox interpreter (all 12 patterns)
rune-subject   subject model (~self ~now ~here ~caps ~code ~libs ~mem ~world)
rune-mold      mold system (types as runtime normalizing functions)
rune           CLI: run / eval / fmt / check
```

External deps (added per milestone, not upfront):
- `nox/rs`    — M6 (compile path, jets)
- `hemera/rs` — M2 (particle hashing)
- `strata`    — M3 (algebra molds: @nebu, @kuro, @jali…)
- `trident`   — M6 (TIR compile path)
- `zheng`     — M9 (lazy proof generation)

## milestones

### M1 — instant start proof-of-concept

Goal: `rune eval '42'` and `rune run file.rune` work end-to-end.
Every Nox pattern (0–11) covered in the interpreter.

Work packages:
- `rune-interp`: implement remaining Nox patterns (6–11: if, compose, push, call, hint, scry-stub)
- `rune-parse`: implement full classic gate syntax (`fn`, `let`, `if`, `loop`, function calls)
- `rune-lower`: complete lowering for all Expr variants (Gate, Let, If, Trap, Call, Eq, Inc, Eval, Compose)
- `rune`: `rune eval` as REPL-on-one-expression (read line, eval, print, loop)
- tests: `double(5) = 10`, `fibonacci(10) = 55`, `if 0 == 0 { 1 } else { 0 }` = 1

Completion signal: all Nox patterns tested; double and fibonacci work end-to-end.

### M2 — subject model + cybermark addresses

Goal: `~self`, `~now`, `~here`, `~world` resolve correctly; `#path` addresses thread through subject.

Work packages:
- `rune-subject`: flesh out Subject construction; add graph-slice stub for ~world
- `rune-lower`: wire subject-slot axis constants from `rune-subject::axis`
- `rune-interp`: implement scry (pattern 11) against subject's ~world slot
- `hemera`: add `hemera/rs` as dep; replace `string_to_atom` stub with real hemera hash
- `rune-parse`: parse `~hint:%tag.selector` hint form; parse `~host:%target.args` host form
- tests: `~self` resolves to subject slot 2; `#cyber/truth` threaded through ~world stub

Completion signal: `rune eval '~self'` returns the neuron atom from the constructed subject.

### M3 — mold system + type checking

Goal: `@ud`, `@da`, `#`, `@neuron` work as runtime mold-gates; static mold checking catches type errors.

Work packages:
- `rune-mold`: add `@p` (phonetic name), `@tas` (term), `@t` (text cord) base molds
- `strata`: add `strata` dep; wire `@nebu`, `@kuro`, `@jali`, `@trop`, `@genies`
- `rune-interp`: apply mold normalization at cast sites (`^-` and `^+` equivalents)
- `rune-parse`: parse mold annotation syntax (`name: @mold` in function params)
- `rune-lower`: emit cast formulas for annotated bindings
- `rune check` command: run static mold pass, report mismatches as errors
- tests: `let x: @ud = "bad"` fails check; `let x: @ud = 42` passes

Completion signal: `rune check` catches `@ud` / cell mismatches; algebra molds (@nebu) available.

### M4 — pure register (rune sigil syntax)

Goal: both registers parse to the same AST; `rune fmt --to=rune` and `--to=rust` round-trip.

Work packages:
- new crate `rune-parse-pure`: parser for sigil-form syntax
  - tall form: `|= sample  body` (layout-sensitive, two-space continuation)
  - flat form: `(|= sample body)`
  - all 25 digraphs recognized and mapped to Expr variants
  - cybermark address forms (`#`, `@`, `~`, `/`, `$`, `^`, `!`, `.`) in expression position
- `rune-parse`: expose `parse_register(src, Register::Classic | Register::Pure)` entry point
- file-level pragma: `:: register: rune` at top of file selects pure register
- `rune fmt --to=rust`: pretty-print AST in classic register
- `rune fmt --to=rune`: pretty-print AST in pure register
- tests: round-trip every Expr variant through fmt in both directions

Completion signal: the `double` example parses from both `fn double(x: @ud) { x * 2 }` and `|= x=@ud  (mul x 2)` to the same AST.

### M5 — dynamic extensions (hint + host + eval)

Goal: graph reactor example works; hint parks and resumes; host calls WASM; eval runs dynamic formulas.

Work packages:
- `rune-interp`: implement hint as parking mechanism
  - `HintHandle` returned when `~hint:` form is reached
  - runtime drives the event loop: deliver event noun → resume continuation
- event loop in `rune` CLI: mock event source (stdin JSON lines) for development
- `rune-interp`: implement host bridge
  - `wasmi` crate for WASM modules
  - `host::wasm(module_particle, fn_name, args)` → Noun
  - host::gpu and host::infer stubs (return Noun::Atom(0) until wGPU/glia land)
- `rune-lower`: lower `Expr::Hint` and `Expr::Host` to Nox patterns 10 and runtime hooks
- `rune-lower`: lower `Expr::Eval` to Nox pattern 2 (already handled in lowering)
- tests: graph reactor loop with mock event source terminates after N events

Completion signal: graph reactor example from rune.md runs with mock events.

### M6 — compile back-end

Goal: pure rune routes through trident's TIR pipeline; compiled .nox artifacts produced; background compilation and transparent swap.

Work packages:
- add `nox = { path = "../nox/rs" }` and `trident` deps to workspace
- `rune-lower`: emit trident-compatible IR for pure (dynamic-free) functions
- background compilation thread: `compile_in_background(source_particle) -> compiled_artifact`
- swap mechanism: hot-function counter in interpreter; trigger compile above threshold
- compiled artifact cached as particle (hemera hash of source → compiled .nox)
- three new TIR opcodes for hint/host/eval (rune-only extension to trident's TIR)

Completion signal: `double(5)` eventually runs from compiled .nox after threshold; interpreter still handles first N calls.

### M7 — planetary cache

Goal: compiled artifacts published as particles in the cybergraph; cold start amortized across neurons.

Work packages:
- particle storage interface: `store_particle(noun) -> Hash`, `fetch_particle(hash) -> Option<Noun>`
- cache key: hemera(source_particle) → compiled artifact particle
- `rune run --cache-dir` option for local particle cache (no cybergraph dep in dev mode)
- invalidation: source particle change → cache miss → recompile

Completion signal: second `rune run` of the same file skips compilation, fetches cached artifact.

### M8 — profile-guided tiering

Goal: hot-path detection, background compilation, transparent swap — user never waits.

Work packages:
- call-site counter in interpreter (per arm/gate)
- background compile queue (tokio task)
- swap on next call after compile finishes
- `#![compile]` pragma for explicit AOT; `#![interpret-only]` to suppress

Completion signal: fibonacci(35) starts interpreted, silently upgrades to compiled mid-run.

### M9 — lazy proof generation

Goal: zheng proofs produced on demand for compiled pure regions; cached by trace particle.

Work packages:
- add `zheng` dep
- proof hook in compiled back-end: capture execution trace for pure regions
- `rune prove <file>` command: run, capture trace, produce proof particle
- proof cached: hemera(trace) → proof particle
- `rune verify <file> <proof>` command

Completion signal: `rune prove double.rune` produces a verifiable zheng proof for `double(5) = 10`.

### M10 — Kelvin freeze

Goal: declare `rune-core` stable; publish K-number; pure grammar matches trident's freeze.

Work packages:
- audit: every Expr variant has a test, every Nox pattern has a test
- version all digraphs and subject slots as frozen
- rune-core Kelvin inherits trident's K-number when trident freezes
- rune-dyn (hint/host/eval) carries its own Kelvin, evolves above frozen core

## open questions (from rune.md spec)

These must be resolved before the relevant milestone:

1. **Register interleaving** (M4): can a single function body mix classic and pure on a per-line basis?
   Leaning toward per-block via fenced markdown code block or file-level pragma.

2. **Mold inference across algebras** (M3): does the type system need explicit algebra parameters
   or can it always infer from sample? `@nebu` and `@kuro` are distinct molds.

3. **Subject capability model** (M2–M3): what restricts which code can read or write subject slots?
   Capability tokens in subject itself? Per-block declared imports?

4. **Markdown hosting** (M1): rune in fenced blocks (graph-native) vs `.rune` files (file-native)?
   Probably both; cybergraph canonical.

5. **Wet gate caching in compiled mode** (M6): wet gates re-typecheck at each call site.
   Need dedup by argument-type fingerprint to avoid compilation explosion.

6. **Hint event matching** (M5): by type, by selector pattern, by subject slot path?
   Leaning toward selector patterns over cybermark addresses.

7. **Scry pure-or-async** (M5): `.^` reads from cybergraph — pure when local, hint when remote.
   Unified primitive with mode inferred from address scope, or two distinct primitives?

8. **Host call typing** (M5): fully typed (optimizer reasons about return shape) or untyped?
   Leaning typed with a typed-void escape for genuinely opaque calls.

9. **Parallel doors** (M5–M6): doors are state-isolated by construction. Can a runtime run
   multiple doors in parallel without coordination? Likely yes; cyberlinks as only cross-door channel.

## current state

M1 complete. M2–M6 substantially implemented, M8 partially done. M7 skipped (not needed).

### completed

**M1** — all Nox patterns (0–17) covered; `rune eval '42'` and `rune run file.rune` work.
- workspace: rune-ast, rune-lex, rune-parse, rune-lower, rune-interp, rune-subject, rune-mold, rune CLI
- Nox patterns 0–17 in rune-interp (44 unit tests)
- classic register parser: atoms, let, fn, if, loop, match, impl/door, rebind/continue, cybermark addresses, `< > <= >= == != + - * /`
- lowering: all Expr variants including Trap/Loop (dynamic $ axis), Rebind (depth-d subject rebuild), Door/Arm, Match, Gate (call-site inlining for closures)
- integration tests: double(5)=10, fibonacci(10)=55, factorial(5)=120, closures

**M2** — done. Subject slots resolve and cybermark addresses scry the world.
- `~self … ~world` lower to axis reads (reserved names resolve via `slot_axis`, with or without `~`)
- `#path` / `@neuron` / `~name` addresses lower to `[17 [1 key] [0 255]]` — scry over `~world`
- `rune_subject::path_atom` is the one address→key hash (real hemera Poseidon2); lower and the
  `world` builder share it; `rune_subject::world(&[(path, val)])` builds a graph slice
- `Subject::standalone()` — resolvable `~self`/`~here`; the CLI injects a live `~now`
- fixed: the scry key was emitted unquoted (`[17 [k …]]`); `look` evaluates the path, so it is now
  `[1 k]`. Verified: `rune eval '~self'` → neuron atom; `#cyber/truth` resolves through a populated world

**M3** (partial) — base molds @ud, @da, #, @neuron; strata molds @nebu, @kuro, @jali, @trop, @genies; mold_and()

**M4** — pure register crate (rune-parse-pure) exists; 25 digraphs mapped to Expr variants

**M5** (partial) — hint passthrough (opcode 16 body evaluates correctly); host stub (identity); scry stub (returns 0)

**M6** (partial) — rune-compile TierRouter: hot-path counter, background compile stub, artifact cache

**M8** (partial) — ResultCache in TierRouter: (formula_hash, subject_hash) memoization for pure formulas after hot_threshold

### pending

- M2: `~world` is empty in standalone — populated by a real graph source (cyb/bbg) later
- M3: mold normalization at cast sites (`^-` / `^+`); remaining strata coverage
- M5: deep hint parking (CPS suspend/resume) — shallow top-level parking exists (`eval_step`/`resume`)
- M5: host WASM bridge (wasmi) — the generic `Host`/act seam is in; no wasmi yet
- M6: trident TIR emission for pure functions (no `trident` dep wired)
- M8: background compile + transparent swap (only result-memoization exists)
- M9: zheng proof generation
- M10: Kelvin freeze audit
