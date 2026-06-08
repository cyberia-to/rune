# decisions

[← specification index](README.md)

---

## rejected alternatives

### separate conventional stack VM for rune

rejected. reasons:
- introduces a third execution model — pure complexity tax
- breaks noun representation that makes [[cybergraph]] integration trivial
- loses provability for everything that goes through it
- fragments tooling — debugger, profiler, formatter, type checker handle two VMs

the better answer for faster interpretation: optimize the [[Nox]] interpreter (inline caches, jet substitution, slot caching).

### merging rune and trident into one language

not a merger — a superset relationship. [[trident]] is the substrate grammar. rune = [[trident]] grammar + three dynamic forms. separate tools, one grammar lineage.

---

## resolved decisions

| question | decision |
|----------|----------|
| tier granularity | per-particle — a particle is a coherent unit, easy to cache, address, and invalidate |
| jet invalidation | jet identity is a [[particle]]. jet upgrade produces a new [[particle]]. compiled artifacts stay pointing at the old jet; new compilations reference the new one |
| proof generation timing | lazy. proofs produced on demand for compiled pure regions, cached by trace particle, reusable across [[neurons]] |
| trident/rune relationship | superset relationship. trident is the grammar substrate; rune extends it with three dynamic forms |
| separate stack VM | rejected. direct [[Nox]] interpretation preserves instant start, noun representation, provability, tooling unity |
| eval at compiled tier | runtime interpreter callback from compiled code. compiled functions containing eval retain an interp escape at the eval point |

---

## still open

1. register interleaving — can a single function body mix classic and pure on a per-line basis, or must each block commit? leaning toward per-block via fenced markdown or file-level pragma
2. mold inference across algebras — `@nebu` and `@kuro` are distinct molds; wet gates inline at call site, but does the type system need explicit algebra parameters or can it always infer from sample?
3. subject capability model — what restricts which code can read or write subject slots? capability tokens in subject itself? per-block declared imports?
4. markdown hosting — rune lives primarily inside markdown fenced blocks (graph-native) or as `.rune` files (file-native)? probably both, [[cybergraph]] canonical
5. wet gate caching in compiled mode — wet gates re-typecheck at every call site; need dedup by argument-type fingerprint to avoid compilation explosion
6. hint event matching — by type, by selector pattern, by subject slot path? leans toward selector patterns over [[cybermark]] addresses
7. scry pure-or-async — `.^` reads from cybergraph; pure when graph slice is local, hint when remote. unified primitive with mode inferred from address scope, or two distinct primitives?
8. host call typing — fully typed (optimizer can reason about return shape) or untyped (worst-case assumptions). leans typed with a typed-void escape for genuinely opaque calls
9. parallel rune at the door level — doors are state-isolated by construction; can a runtime run multiple doors in parallel without coordination? likely yes, with cyberlinks as the only cross-door communication channel
