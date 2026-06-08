# rune language specification

rune K140 — working spec, mutations expected

see [../README.md](../README.md) for project overview and implementation status.

---

## specification

| file | what it covers |
|------|----------------|
| [subject.md](subject.md) | the subject noun — the environment every evaluation runs against |
| [grammar.md](grammar.md) | computation digraphs — the variant axis and the 25 digraphs |
| [model.md](model.md) | the core model, molds as functions, data structures as nouns |
| [execution.md](execution.md) | three execution modes, eval as intrinsic, instant start |
| [architecture.md](architecture.md) | workload split, tier mechanics, reference + jets, the kernel |
| [examples.md](examples.md) | worked programs in both registers |
| [trident.md](trident.md) | relation to trident and Rs, the proven subset, Kelvin discipline |
| [decisions.md](decisions.md) | rejected alternatives, resolved decisions, still-open questions |

---

## three parents

### from [[hoon]] — the model

the model is correct. the surface taxes adoption. take the model, keep the original surface as the pure register alongside a familiar classic register.

| inherit | what it gives |
|---------|--------------|
| subject-oriented evaluation | environment IS a noun — modules, state, code, all in one tree |
| fixed-arity keywords | precedence disappears |
| symbol-only keywords | alphabetic names are always free, no reserved words |
| tall/flat duality | one grammar at any scale |
| cores — gates, doors, traps | functions, objects, thunks unified as `[code data]` |
| molds as functions | types are runtime values that also drive static checking |
| wet and dry gates | generics via inlined re-typecheck, no higher kinds |
| reference + jets | spec stays small and correct; jets are fast and verified equivalent |
| Kelvin versioning | the spec freezes by design |
| solid-state interpreter | event log + subject = replayable, hot-upgradable kernel |
| eval as base operation | `.*` is one digraph, no special `eval()` form |

### from the current rune — the surface

| inherit | what it gives |
|---------|--------------|
| Rust-shape syntax option | minutes to onboard anyone with ALGOL or Rust background |
| `hint` for async | explicit yield/resume tied to [[cybergraph]] events |
| `host()` for FFI | clear boundary to WASM, wGPU, [[glia]] |
| `eval()` ergonomic form | syntax sugar over `.*` for everyday code |
| instant start | tree construction stands in for compilation — no build phase |
| three jet categories | pure (proven), host (practical), hint (async) |

### from [[cybermark]] — the address layer

| inherit | what it gives |
|---------|--------------|
| `#` particle, `@` neuron, `~` name | address sigils ARE values in every expression context |
| `/` scope, `^` root, `*` wildcard | path traversal is computation traversal |
| `!` action, `.` pipeline | verbs are first-class |
| sigil-based, language-neutral | works equally in Russian, Mandarin, English |
| frozen sigil set | the address language commits to a fixed alphabet |

---

## the central idea

one language with one AST, parsed from two syntactic registers. both lower to the same [[Nox]] noun. files can mix registers freely.

| register | tag | who writes it |
|----------|-----|--------------|
| `rust` | `rust` | every Rust, Go, or TypeScript programmer day one |
| `rune` | `rune` | systems programmers, [[dialects|dialect]] authors, agent kernels |

`rune fmt --to=rune` converts mechanically. nothing forces movement. both coexist in the codebase forever.

---

## relation to other layers

| layer | role |
|-------|------|
| [[cybermark]] | the address sigil set — addresses everywhere in rune |
| [[Nox]] | the target — every rune AST lowers to a Nox noun |
| [[hoon]] | the inspiration for the subject model and pure register |
| [[zheng]] | the prover — every pure Nox trace becomes a proof |
| [[cyb/robot]] | the canonical consumer — kernel functions in rune |
| [[dialects]] | first-class rune constructs |
| [[bbg]] | event log storage substrate |
| [[radio]] | source of async hints from the network |
