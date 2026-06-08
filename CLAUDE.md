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

## Layout
The Rust workspace lives at `rs/` — run all `cargo` commands from there.
Folders are bare; package names keep the `rune-` prefix (folder `rs/ast` →
package `rune-ast`). Docs live at the project root beside `rs/`:
`specs/`, `docs/`, `.claude/`.

```
rs/ast          rune-ast         shared AST (Expr) and Noun types
rs/lex          rune-lex         tokenizer — shared for both registers
rs/parse        rune-parse       parser — classic (rust) register
rs/parse-pure   rune-parse-pure  parser — pure register (all 25 digraphs)
rs/lower        rune-lower       Expr → Nox noun
rs/interp       rune-interp      tree-walking Nox interpreter (instant start) + Host/acts
rs/subject      rune-subject     subject model (~self ~now ~here ~world …)
rs/mold         rune-mold        mold system (types as normalizing functions)
rs/prysm        rune-prysm       rune↔prysm bridge (chunk-noun → tape::Chunk)
rs/compile      rune-compile     compile tier — TierRouter, ArtifactCache (M6)
rs/cli          rune-cli         CLI binary `rune` (run / eval / fmt / check)
```

## Invariant: Instant Start
Every architectural decision must preserve instant start:
parse → lower → interpret begins immediately with no compilation phase.
This is the contract. Break it only for the compile back-end (M6),
and only as a background path — the interpreter path stays the default.

## Shell: Nushell
Use `nu -c '...'` or `nu script.nu` for scripting. Reserve bash for git.

## Testing
- Unit tests live in the crate they test (`#[cfg(test)]` in `lib.rs`)
- Integration tests for the CLI go in `rs/cli/tests/`
- Snapshot tests (insta) for parser and lowering output
- Every Nox pattern (0–11) must have a unit test in rune-interp
