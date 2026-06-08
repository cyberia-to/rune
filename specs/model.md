# the core model

[← specification index](README.md)

---

a `core` is `[battery payload]` — code paired with data. all higher constructs are cores.

| construct | shape | what |
|-----------|-------|------|
| gate | `[code [sample context]]` | function |
| door | `[code [sample state context]]` | object with methods |
| trap | `[code context]` | thunk |
| library | `[code context]` | namespace |
| book | `[code [slot-1 slot-2 ... context]]` | record |

calling a gate: replace `sample` slot, evaluate `code` against the resulting core. tree substitution — no stack, no calling convention.

## gates are particles

a gate is a noun. a noun has a [[Hemera|hemera hash]]. a hemera hash is a [[particle]]. every gate is a [[particle]] in the [[cybergraph]] by construction. self-reference becomes structural rather than syntactic.

---

# molds — types as functions

a mold is a gate `noun -> noun` that idempotently normalizes a value to its canonical icon. molds are values — composable, storable, passable — and the compiler evaluates them statically for type checking.

```
@nebu       :: Goldilocks scalar mold
@kuro       :: F₂ tower element mold
@jali       :: ring element mold
@trop       :: tropical semiring element mold
@genies     :: CSIDH-512 element mold
@ud         :: unsigned decimal mold
@p          :: phonetic name mold
@da         :: absolute date mold
#           :: any particle mold
@neuron     :: neuron mold
```

molds extend cleanly across the five [[strata]] algebras. arithmetic is mold-polymorphic via wet gates — `add` works on any algebra, the compiler picks the right [[lens]] backend per call site.

---

# data structures as nouns

[[Nox]] nouns ARE the data structures. no heap, no garbage collector — allocation is `cons`, freeing is not referencing.

| classic type | [[Nox]] noun | how |
|---|---|---|
| `Vec<T>` | cons-list | `cons(head, cons(head, cons(head, nil)))` |
| `HashMap<K,V>` | Merkle tree | balanced binary tree of key-value pairs, authenticated |
| `String` | [[Hemera]] hash | content-addressed in the [[cybergraph]] — a string IS a [[particle]] |
| `Box<T>` | subtree | `cons(value, nil)` |
| dynamic growth | `cons` | every `cons` creates new structure, immutable |

classic register syntax sugar:
- `[1, 2, 3]` → `cons(1, cons(2, cons(3, nil)))`
- `{ key: value }` → balanced Merkle tree
- `"text"` → `Hemera(bytes)` — a [[particle]] hash

pure register: noun literals written directly — `[1 2 3]`, `[%key value]`, `~[bytes]`
