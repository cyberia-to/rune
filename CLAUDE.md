# Claude Code Instructions

## Git Workflow
- **Commit by default.** After completing a change, commit it.
- **Atomic commits.** One logical change per commit.
- **Conventional commits.** Use prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`.

## Architecture
rune is the open-computation language for cyber's soft3 stack.
See `~/cyber/cyber/root/rune.md` for the full specification.

Stack position:
```
cybermark  →  rune inline  →  rune  →  trident  →  Rs
```

Sibling repos (all at `~/cyber/`):
- `nox/rs`     — Nox VM (18-pattern tree-reduction: 16 compute + call + look); added as dep in M6
- `trident`    — compile path substrate; added as dep in M6
- `strata`     — algebra molds (@nebu, @kuro…); added in M3
- `hemera/rs`  — hash primitive; added in M2
- `zheng`      — prover; added in M9

## Crate Map
```
rune-ast      shared AST (Expr) and Noun types
rune-lex      tokenizer — shared for both registers
rune-parse    parser — classic (rust) register [pure register: M4]
rune-lower    Expr → Nox noun
rune-interp   tree-walking Nox interpreter (instant start)
rune-subject  subject model (~self ~now ~here ~world …)
rune-mold     mold system (types as normalizing functions)
rune          CLI binary (run / eval / fmt / check)
```

## Invariant: Instant Start
Every architectural decision must preserve instant start:
parse → lower → interpret begins immediately with no compilation phase.
This is the contract. Break it only for the compile back-end (M6),
and only as a background path — the interpreter path stays the default.

## Shell: Nushell
Use `nu -c '...'` or `nu script.nu` for scripting. Reserve bash for git.

## Testing
- Unit tests live in the crate they test (`#[cfg(test)]` in `src/lib.rs`)
- Integration tests for the CLI go in `crates/rune/tests/`
- Snapshot tests (insta) for parser and lowering output
- Every Nox pattern (0–11) must have a unit test in rune-interp
