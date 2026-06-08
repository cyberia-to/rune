# rune 0.1.0 release prep

Tracking the structural + quality work to bring rune to the cyber project
convention (reference: `~/cyber/inf` — the multi-crate language analog —
and `~/cyber/hemera`).

## convention (confirmed from inf + hemera)

- workspace root lives at `rs/Cargo.toml`; the project root has **no** Cargo.toml
- workspace members are bare-named folders under `rs/`: `rs/ast`, `rs/lex`, …
- **package names keep the project prefix** (`inf-ast`, so `rune-ast`) — only the
  folder loses it. This keeps every `use rune_ast::…` and `{ workspace = true }`
  dep unchanged; only paths move.
- the CLI is a member: `rs/cli`, package `rune-cli`, `[[bin]] name = "rune"`
- `specs/`, `docs/`, `.claude/`, CHANGELOG.md, LICENSE, README.md, CLAUDE.md live
  at the **project root**, beside `rs/`
- license: `license = "Cyber"` via `[workspace.package]`, plus a LICENSE file

## target layout

```
rune/
├── rs/                  workspace root (Cargo.toml + Cargo.lock)
│   ├── ast/  lex/  parse/  parse-pure/  lower/  prysm/
│   ├── interp/  subject/  mold/  compile/
│   └── cli/             (was crates/rune; package rune-cli, bin rune)
├── specs/               multiple files (split the 650-line README)
├── docs/                tutorials/ guides/ explanation/  (Diataxis)
├── .claude/plans/
├── CHANGELOG.md  LICENSE  CLAUDE.md  PLAN.md  README.md
```

## phases

1. **Restructure** (this commit)
   - `git mv crates/rune-X rs/X` (strip prefix), `crates/rune rs/cli`
   - move root Cargo.toml → rs/Cargo.toml; members bare; workspace.deps paths
     bare; `cyber-hemera` path `../hemera/rs` → `../../hemera/rs`; add
     `[workspace.package] license = "Cyber"`
   - rs/cli/Cargo.toml: package `rune` → `rune-cli`
   - cyb/shell/Cargo.toml: 6 paths `../../rune/crates/rune-X` → `../../rune/rs/X`
   - regenerate Cargo.lock; `cargo test` from rs/; verify cyb-shell `cargo check`
   - update CLAUDE.md build paths

2. **LICENSE + CHANGELOG** — cyber license file; CHANGELOG with 0.1.0 entry

3. **Split specs/** — break specs/README.md (650 lines) into language.md,
   grammar.md, ir.md, subject.md, molds.md, acts.md, … + README index

4. **docs/ Diataxis** — tutorials/ (first program), guides/ (task how-tos),
   explanation/ (architecture, registers, acts/ward)

## deferred quality-audit findings (post-restructure, separate commits)

- pass 5: 4× `.unwrap()` in rune-lower library paths
- pass 6: error types are `{message:String}` → enum variants
- pass 2/7: rune-compile `contains_dynamic`/`formula_hash` unbounded recursion
- pass 8/10: rune-compile TierRouter mixes concerns + redundant clones
- file size: lower(1138) interp(900) parse-pure(778) parse(766) mold(556) > 500
- pass 12: no property tests for field arithmetic invariants
- pass 4: inv_field non-constant-time (document)
- pass 9: precedence magic numbers; parse_fn → parse_gate
