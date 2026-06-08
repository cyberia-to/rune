# instant start

Between a rune program becoming available and it starting to run, only one
thing happens: parsing. There is no compilation phase.

## what it means

```
source ──parse──> AST ──lower──> Nox noun ──reduce──> result
```

Each arrow is milliseconds for any human-scale program. The interpreter walks
the Nox noun and reduces it directly; it does not first emit bytecode, build a
type-checked IR, or run a static analysis pass. The moment the source is in
hand, it runs.

## why it is non-negotiable

rune is the authoring layer for the [[cybergraph]] — agent kernels, terminal
programs, dialects evaluated on demand. These need to react to events with low
latency and reload code by rewriting their own subject. A build phase between
source and execution would poison all of it: every hot reload, every
dynamically-built formula, every REPL keystroke would pay a compile tax.

So every architectural decision is checked against one question: *does this
preserve the ability to run a program the moment its source becomes available,
with no phase between?* If a feature requires a build step, it does not belong
on the default path.

## the compile tier does not break it

rune has a compile back-end (`rune-compile`, milestone M6) that lowers hot
functions further for steady-state speed. It runs **in the background**, never
on the path to first execution. A function is interpreted instantly on its
first call; only if it runs hot does compilation happen, off to the side, and
the compiled form is swapped in on a later call. The interpreter path stays the
contract; compilation is an optimization layered on top, never a precondition.

## see also

- [architecture.md](../../specs/architecture.md) — the interpreter and compiler tiers
- [execution.md](../../specs/execution.md) — the three execution modes
