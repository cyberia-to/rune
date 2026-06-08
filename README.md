# rune

[[Rs]] syntax + [[hoon]]'s subject model + [[cybermark]]'s sigil layer. one language, one AST, two syntactic registers — classic (familiar) and pure (alien) — lowering directly to [[Nox]].

provable at every tier: pure code unconditionally, event-reactive code relative to its event log, host calls relative to their results. instant start by design.

rune is the dynamic evaluation layer above [[trident]]'s 14 deterministic substrate languages. it adds three extensions: `hint` (event stream), `eval` (runtime formulas), `host` (external compute) — each with its own proof contract. `host` is the only genuine trust boundary.

full language specification: [specs/README.md](specs/README.md)

---

## two registers, one AST

every program expressible two ways. both parse to the same AST and lower to the same [[Nox]] noun.

```rust
// classic register — familiar to any Rust, Go, or TypeScript programmer
fn double(x: @nebu) -> @nebu { x * 2 }
```

```
:: pure register — digraph grammar, alien and precise
|=  x=@nebu  (mul x 2)
```

`rune fmt --to=rune` converts mechanically. nothing forces movement. both coexist forever.

---

## instant start

between source and execution, only parsing happens. parse → Nox noun → tree-walk reduction begins immediately. no compilation phase, no bytecode generation, no static analysis pass between source and run.

```
rune run program.rune
rune eval '6 * 7'          # classic register → 42
```

---

## stack position

```
cybermark  →  rune inline  →  rune  →  trident  →  Rs
```

| layer | role |
|-------|------|
| [[cybermark]] | address sigil set — `#` `@` `~` `/` `^` `!` `.` |
| rune inline | cybermark + `sigil[expr]` — reactive prose |
| rune | full language — gates, doors, agents, kernels |
| [[trident]] | proven subset — consensus-critical, frozen |
| [[Rs]] | native-speed jets |

---

## crate map

| crate | what |
|-------|------|
| `rune-ast` | shared AST (`Expr`) and `Noun` types |
| `rune-lex` | tokenizer — shared for both registers |
| `rune-parse` | parser — classic (rust) register |
| `rune-parse-pure` | parser — pure register (all 25 digraphs) |
| `rune-lower` | `Expr` → Nox noun |
| `rune-interp` | tree-walking Nox interpreter (instant start) |
| `rune-subject` | subject model (`~self` `~now` `~here` `~world` …) |
| `rune-mold` | mold system (types as normalizing functions) |
| `rune-prysm` | rune↔prysm bridge — chunk-noun → `tape::Chunk` |
| `rune-compile` | compile tier — `TierRouter`, `ArtifactCache` (M6) |
| `rune` | CLI binary (`run` / `eval` / `fmt` / `check`) |

---

## implementation status

| milestone | what | status |
|-----------|------|--------|
| M1 | classic parser + lowerer + 18-pattern Nox interpreter | done |
| M2 | subject model (`~self` … `~world`) + cybermark addresses | axis constants done; hemera hash stubbed |
| M3 | mold system (`@nebu` `@ud` `@p` `@tas` `@t` `@flag`) | basic molds done; `@kuro` `@jali` `@trop` `@genies` `@da` pending |
| M4 | pure register — all 25 digraphs | done |
| M5 | dynamic extensions — acts, host bridge, hint parking | acts live (`Host` trait, `eval_with_host`); hint parking via `eval_step`/`resume`; deep cell suspend/resume (CPS) pending |
| M6 | compile tier — `TierRouter`, trident integration stub | router done; trident call stubbed |
| M7 | planetary cache — compiled artifacts as particles | pending |
| M8 | profile-guided tiering — background compile, hot-swap | pending |
| M9 | lazy proof generation — zheng integration | pending |
| M10 | Kelvin freeze | pending |

items marked "stubbed" compile and run but return `Atom(0)` — the eval path skips them. items marked "pending" are not started.

---

## quick start

```sh
cargo build
cargo run --bin rune -- eval '6 * 7'              # → 42
cargo run --bin rune -- eval 'text("hi")'         # → (# t) hi   (a prysm chunk)
cargo run --bin rune -- eval 'emit(text("hi"))'   # → (# t) hi   (via an act)
cargo test
```

---

## open questions

see [specs/README.md#still-open](specs/README.md#still-open) for the full list. key unresolved items:

1. register interleaving — per-line mix or per-block commit?
2. mold inference across algebras — explicit algebra parameters or always inferred?
3. subject capability model — what restricts subject slot access?
4. hint event matching — by type, selector pattern, or subject slot path?
5. host call typing — fully typed return shapes or untyped?
