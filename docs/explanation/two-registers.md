# two registers, one AST

rune can be written two ways. The same program, the same meaning, two
spellings. This is a deliberate design choice, not an accident of history.

## the problem

The model rune inherits from [[hoon]] is correct — subject-oriented
evaluation, fixed-arity keywords, cores. But the surface taxes adoption: a
digraph grammar is alien to anyone arriving from Rust, Go, or TypeScript.

Throwing the model away to get a familiar surface would be the wrong trade.
Keeping only the alien surface would be the wrong trade too.

## the resolution

Keep the model. Offer two surfaces over it:

- the **classic** register — Rust-shaped, familiar on day one
- the **pure** register — the digraph grammar, austere and precise

Both parse to one AST (`rune-ast`) and lower to one [Nox](../../specs/README.md)
noun. The conversion between them is mechanical — `rune fmt` does it without
loss, because there is nothing to lose: the registers differ only in spelling,
never in structure.

## why this works

Because the AST is the single source of truth, the two registers cannot drift.
A tool that understands the AST — the formatter, the checker, the
interpreter — understands both registers for free. There is no second parser
to keep in sync with the first; there is one AST with two front-ends.

The practical effect: a newcomer writes classic, a systems programmer or
[[dialects|dialect]] author writes pure, and the same file can hold both. No
migration, no deprecation. Both coexist forever.

## see also

- [convert between registers](../guides/convert-registers.md) — the `fmt` recipe
- [grammar.md](../../specs/grammar.md) — the pure-register digraphs
