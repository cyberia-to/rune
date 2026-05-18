# rune language specification

rune K140 — working spec, mutations expected

see [../README.md](../README.md) for project overview and implementation status.

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
| `rune` | `rune` | systems programmers, [[semcons|semcon]] authors, agent kernels |

`rune fmt --to=rune` converts mechanically. nothing forces movement. both coexist in the codebase forever.

---

## subject — the noun that holds everything

every rune evaluation runs against a subject — a [[Nox]] noun containing the entire visible environment.

```
subject = [
  ~self  : @neuron       :: who I am                axis 2
  ~now   : @da           :: current time             axis 6
  ~here  : #             :: where in the graph       axis 14
  ~caps  : capabilities  :: what I am allowed to do  axis 30
  ~code  : core          :: the program itself        axis 62
  ~libs  : core          :: imported libraries        axis 126
  ~mem   : ...           :: persistent state          axis 254
  ~world : graph-slice   :: the cybergraph I can see  axis 255
]
```

subject is a right-nested cons tree. axis navigation follows binary tree addressing: `axis(n) = (1 << (slot+2)) - 2` for slots 0–6; ~world is the rightmost tail at axis 255.

every name resolves through the subject. `~self` finds the neuron slot. `~mem.counter` reads from persistent state. `#cyber/truth` resolves through `~world` then path.

three consequences fall out of this design:

1. metaprogramming is trivial — the program lives in `~code`, manipulate it like any other noun
2. eval is trivial — evaluate any formula against any subject via `.*`
3. hot upgrade is trivial — receive an event whose payload includes new `~code`, install with `=.`

### cybermark unifies with subject traversal

`#cyber/truth` is a wing through the subject's `~world` slot. when you write `#cyber/truth.rank`:

1. `#cyber/truth` resolves through `~world` in the subject
2. `.rank` pipes through a computation

the dot is the same dot. addresses ARE computations against the subject.

---

## computation digraphs

digraphs compose two characters: the first (family) carries the atomic semantic; the second (variant) determines the shape within that family.

### variant axis (second character)

| char | name | shape within family |
|------|------|---------------------|
| `-` | hep | minimal — the bare, degenerate form |
| `=` | tis | explicit — with binding or comparison |
| `*` | tar | generic — wildcard, broadcast, polymorphic |
| `+` | lus | augmented — more arguments, larger arity |
| `/` | fas | structural — scoped, path-aware |
| `.` | dot | composed — transform, pipeline |
| `:` | col | paired — two-branch, two-way |
| `>` | gar | flowing — sequential composition |
| `~` | sig | null — absent, optional, empty |
| `^` | ket | meta — abstract, lifted |
| `&` | pam | combined — trace, accumulate |
| `_` | cab | reversed — alternative, mirror |
| `%` | cen | rich — multi-part, structured |
| (doubled) | — | canonical — the prototype of the family |

### the 25 digraphs

| digraph | family × variant | what it makes | arity |
|---------|------------------|---------------|-------|
| `\|=` bartis | core × explicit | gate (with sample) | 2 |
| `\|*` bartar | core × generic | wet gate | 2 |
| `\|-` barhep | core × minimal | trap (no sample) | 1 |
| `\|%` barcen | core × rich | door (multi-arm) | 1 |
| `=/` tisfas | bind × structural | let binding | 3 |
| `=.` tisdot | bind × composed | rebind subject slot | 3 |
| `=>` tisgar | bind × flowing | compose right against left | 2 |
| `?:` wutcol | test × paired | if-then-else | 3 |
| `?-` wuthep | test × minimal | switch by tag | 2 |
| `?=` wuttis | test × explicit | pattern match with binding | 2 |
| `?~` wutsig | test × null | null test | 3 |
| `:-` colhep | cell × minimal | pair | 2 |
| `:+` collus | cell × augmented | triple | 3 |
| `:_` colcab | cell × reversed | flipped pair | 2 |
| `^-` kethep | cast × minimal | cast to explicit mold | 2 |
| `^+` ketlus | cast × augmented | cast to value's mold | 2 |
| `.*` dottar | eval × generic | evaluate formula against subject | 2 |
| `.+` dotlus | eval × augmented | increment | 1 |
| `.^` dotket | eval × meta | scry — read from graph | 2 |
| `~&` sigpam | hint × combined | trace | 2 |
| `~_` sigcab | hint × reversed | hint pair | 2 |
| `/+` faslus | build × augmented | import library | 1 |
| `/-` fashep | build × minimal | import types only | 1 |
| `++` luslus | arm × canonical | arm definition | 2 |
| `!!` zapzap | crash × canonical | crash | 0 |

every digraph decomposes: `?-` = test-minimal = simplest test (switch). `\|*` = core-generic = wet gate. `.^` = eval-meta = scry over the graph layer.

fixed arity per digraph. precedence disappears. tall and flat forms parse identically.

digraphs do not collide with cybermark sigils: sigils stand alone (`#name`, `@alice`, `~truth`); digraphs are exactly two symbols with no whitespace (`\|=`, `?:`, `=/`).

---

## the core model

a `core` is `[battery payload]` — code paired with data. all higher constructs are cores.

| construct | shape | what |
|-----------|-------|------|
| gate | `[code [sample context]]` | function |
| door | `[code [sample state context]]` | object with methods |
| trap | `[code context]` | thunk |
| library | `[code context]` | namespace |
| book | `[code [slot-1 slot-2 ... context]]` | record |

calling a gate: replace `sample` slot, evaluate `code` against the resulting core. tree substitution — no stack, no calling convention.

### gates are particles

a gate is a noun. a noun has a [[Hemera|hemera hash]]. a hemera hash is a [[particle]]. every gate is a [[particle]] in the [[cybergraph]] by construction. self-reference becomes structural rather than syntactic.

---

## molds — types as functions

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

## data structures as nouns

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

---

## three execution modes

### pure — proven by construction

every reduction in the pure subset has a [[Nox]] trace, which has a [[zheng]] proof.

```
|=  [x=@nebu y=@nebu]
(add (mul x x) (mul y y))
```

### reactive — event-driven computation

a `hint` yields execution until matching data arrives. the kernel is a pure function `(subject, event) -> (new-subject, effects)` — the event is the witness. [[Nox]] is confluent: reduction order does not affect the result. the proof is conditional on the event log — given the sequence of events as witnesses, every step is fully provable.

```
|=  p=#
|-
=/  e   ~hint:%particle.p
=/  r   !rank.p
?:  (gth r 0.5)
  !cyberlink.p.^truth.+1
$
```

the runtime parks the trap. when [[radio]] delivers a matching event or the [[cybergraph]] observes the addressed [[particle]] change, the runtime resumes with the event noun in the appropriate subject slot.

### host — the only genuine trust boundary

a host call runs native code outside [[Nox]] — WASM module, wGPU shader, [[glia]] inference. the call returns a noun. the proof system records "host result was N" as a witness; the surrounding pure computation is provable conditional on that witness. the host computation itself carries no [[Nox]] trace.

```
=/  features  ~host:%infer.model.input
=/  ranked    (rank-by-features features)
```

the proof spectrum is three tiers:

```
unconditional proof:    any pure digraph reduction, mold normalization, arm lookup
conditional proof       any computation containing hint — proven given the event log
  (event log witnesses):  as witnesses. full provability, external inputs as given
conditional proof       any computation containing host — proven given the host
  (host witnesses):       results as witnesses. trust boundary is the host itself
```

events are [[cybergraph]]-native and verifiable by anyone. host results require trusting the host runtime. the event log is the difference between the two.

---

## eval is intrinsic

[[Nox]] has `.*` — evaluate formula against subject. eval is one digraph, the same kind as `add` or `cons`.

```
=/  src   "(mul x 2)"
=/  ast   (parse src)
=/  form  (lower ast)
.*(.  form)
```

macros, interpreters, hot patching, runtime DSLs — all are special cases of `.*`.

---

## instant start

between source becoming available and execution starting, only parsing happens. parse → [[Nox]] noun → tree-walk reduction begins immediately. no compilation phase. latency bounded by parsing time — milliseconds for any human-scale program.

instant start enables:
- REPL-style interaction with the [[cybergraph]]
- agent kernels reacting to events with low latency
- `eval()` of dynamically-built formulas
- hot code reload by subject rewrite
- [[semcons]] evaluated on demand
- the [[cyb/robot]] starting up and being responsive immediately

instant start is non-negotiable. every architectural decision is checked against one question: does this preserve the ability to run the program the moment its source becomes available, with no phase between?

---

## examples

### double a number

```rust
// classic
fn double(x: @nebu) -> @nebu { x * 2 }
```

```
:: pure
|=  x=@nebu  (mul x 2)
```

### graph reactor — watch, rank, link

```rust
// classic
async fn reactor(p: #) {
    loop {
        let _ = hint::particle_changed(p);
        let r = !rank(p);
        if r > 0.5 {
            !cyberlink(p, ^truth, +1);
        }
    }
}
```

```
:: pure
|=  p=#
|-
=/  _   ~hint:%particle.p
=/  r   !rank.p
?:  (gth r 0.5)
  !cyberlink.p.^truth.+1
$
```

### trading agent — hint + host + pure

```rust
// classic
fn trading_agent(pair: #, model: #) {
    let threshold = 0.7;
    loop {
        let price  = hint::particle_changed(pair);
        let signal = host::infer(model, price.history);
        let conf   = Tri::normalize(signal);
        let sent   = Arc::rank(pair, 100);
        if conf > threshold {
            !cyberlink(pair, ^truth, +1);
        }
        host::gpu(chart_shader, [price, signal, sent]);
    }
}
```

```
:: pure
|=  [pair=# model=#]
=/  threshold  .7
|-
=/  price   ~hint:%particle.pair
=/  signal  ~host:%infer.model.price
=/  conf    (normalize:Tri signal)
=/  sent    (rank:Arc pair 100)
?:  (gth conf threshold)
  !cyberlink.pair.^truth.+1
~host:%gpu.[chart-shader [price signal sent]]
$
```

### a [[neuron]] as a door

```
|%
++  identity     ~self
++  follow       |=  who=@   !cyberlink:~self:who:+1
++  unfollow     |=  who=@   !cyberlink:~self:who:-1
++  pulse        |-          =.  ~now  ~hint:%tick   $
--
```

### dynamic — load and run code from the graph

```
=/  code   #~/agents/trader
=/  ast    (parse code)
=/  form   (lower ast)
=/  sub    [~self ~now ~here ~caps ~mem ~world]
.*(sub form)
```

---

## workload split

| workload | profile | right strategy |
|----------|---------|----------------|
| REPL, scripts, ad-hoc | run once, dies fast | interpret directly |
| event handlers, UI | run often per session, short bursts | interpret + jet substitution on hot ops |
| agent kernels ([[cyb/robot]]) | run for years | interpret first, compile in background, swap when ready |
| [[semcons]] (consensus-critical) | called by every [[neuron]], deterministic + fast | compile AOT through [[trident]], deploy as proven .nox |
| inner-loop primitives | hot, narrow, performance-critical | written in [[Rs]], AOT-compiled, called as jets |

the source language is the same in all five rows. choice is per-particle, opportunistic, and reversible.

---

## execution architecture

```
rune source (classic or pure)
     │
     ▼  parse (ms)
shared AST (rune-ast)
     │
     ▼  lower (ms)
Nox noun (18 patterns)
     │
     ├──────────────────────────┐
     ▼                          ▼
tree-walking interpreter    compile pipeline
(default, instant start)    (Nox → TIR → optimized TASM)
     │                          │
     │                     neural optimizer
     │                     (extended for hint/host/eval)
     │                          │
     │                     proven .nox + jet hints
     │                          │
     │                     zheng proof (lazy)
     ├──────────────────────────┘
     ▼
Nox runtime
+ Rs jets (compiled hot paths)
+ host bridge (WASM, wGPU, [[glia]])
+ cybergraph cache (compiled artifacts as particles)
```

### interpreter back-end (instant start path)

rune evaluates directly into [[Nox]] tree rewriting. there is no separate rune VM. [[Nox]] is the VM, rune is one of its surfaces.

optimizations that keep the interpreter fast without losing instant start:
- inline caches: arm lookups, slot accesses, mold dispatches cached per call site after first execution
- jet substitution: matching [[Nox]] subtrees swap to native [[Rs]] jets at runtime
- subject pinning: the subject does not change during a call; cache slot offsets
- pre-flattening: cons-list `[1 2 3 nil]` exposes as an array view for sequential access
- escape analysis: short-lived intermediate nouns can live on a stack rather than the heap

none requires a compilation phase. instant start preserved at every step.

### compiler back-end (steady-state path)

same [[Nox]] noun, lowered further to [[trident]]'s TIR. same TIR passes (DCE, inlining, constant folding, algebra-specific lowering). output: optimized TASM → final .nox bytecode with annotations identifying jettable subtrees.

three new TIR opcodes for rune's dynamism:
- `hint` — yields execution, resumes on matching event. opaque to optimizer
- `host` — escapes to WASM/wGPU/[[glia]], returns a noun. typed return shape lets optimizer reason about callers
- `eval` — runs a dynamically constructed formula. requires runtime interpreter callback from compiled code

compilation result is itself a [[particle]]. indexed in the [[cybergraph]] by source particle. reusable across all [[neurons]] — the planetary compilation cache.

### tier mechanics

| trigger | from | to | latency to caller |
|---------|------|----|-------------------|
| first call | source | parse + lower + interpret | ms |
| second call, cold | interp | interp + inline-cache | none added |
| function hot (>N calls/sec) | interp | submit for compile in background | none — user never waits |
| compile finishes | interp | compiled .nox | swap on next call |
| source particle changes | any | reset to interp | no compile-time wait |
| explicit `#![compile]` pragma | first call | compile then run | one-time compile cost |
| explicit `#![interpret-only]` | always | interp | never compiles |
| jet upgrade (new particle) | compiled | mark stale | next call recompiles in background |

proof generation is lazy. compiled pure regions produce [[zheng]] proofs only when requested, cached by trace particle, reusable across [[neurons]].

---

## relation to [[trident]] and [[Rs]]

[[trident]] is the substrate. rune is [[trident]] extended. one grammar lineage, two tools.

| dimension | [[trident]] | rune | [[Rs]] |
|-----------|-------------|------|--------|
| primary purpose | provable consensus-critical code | dynamic personal/agent code | native-speed jets |
| type system | mold-based, mandatory static enforcement | mold-based, optional enforcement | Rust-subset static |
| dynamism | none — total determinism | eval, hint, host as first-class | none |
| provability | mandatory — every program produces a [[zheng]] proof | tiered — pure: unconditional; reactive: conditional on event log; host: conditional on host results | reference + jet equivalence verified at compile time |
| evaluation | AOT compiled to optimized .nox | interpret first, compile in background | AOT compiled to native, deployed as jet |
| launch latency | slow (compile + prove) | instant (parse + lower + walk) | n/a — runs as called |
| audience | protocol authors, semcon writers | agent developers, scripters | runtime engineers |

rune grammar = [[trident]] grammar + three dynamic forms (hint, host, eval). [[trident]] is self-contained; rune depends on [[trident]]'s grammar spec, not its toolchain.

### why [[trident]] stays dynamic-free

[[trident]]'s proof-mandatory guarantee requires total determinism. the three rune extensions break this:

- `hint`: proof becomes conditional on the event log — trident requires unconditional proofs
- `host`: result is an asserted witness — the host runtime carries no [[Nox]] trace
- `eval`: proof depends on runtime content — formula is unknown at compile time

### the trident subset — explicit scope

every digraph form in rune is [[trident]]-compatible. the pure register IS the trident grammar.

trident-native forms:

| component | what it includes |
|-----------|-----------------|
| all 25 base digraphs | `\|=` `\|*` `\|-` `\|%` `=/` `=.` `=>` `?:` `?-` `?=` `?~` `:-` `:+` `:_` `^-` `^+` `.*` `.+` `.^` `~&` `~_` `/+` `/-` `++` `!!` |
| full subject model | `~self` `~now` `~here` `~caps` `~code` `~libs` `~mem` `~world` |
| full core model | gate, door, trap, library, book |
| full mold system | `@nebu` `@kuro` `@jali` `@trop` `@genies` `@ud` `@p` `@da` `#` `@neuron` and any user-defined mold |
| cybermark address sigils | `#` `@` `~` `/` `$` `^` `!` `.` in noun-position addresses |

rune-only extensions — what the trident checker explicitly rejects:

| form | what it does | why excluded |
|------|-------------|--------------|
| `hint expr` | parks execution until external event | proofs become conditional on event log |
| `host(target, args)` | escapes to WASM/wGPU/[[glia]] | result is an asserted witness |
| `eval(formula)` | executes a runtime-constructed formula | proof depends on runtime content |

a rune file containing none of the three dynamic forms is [[trident]]-compatible as-is — no translation, same grammar.

---

## Kelvin discipline

[[trident]]'s grammar is the Kelvin-versioned substrate. starts at K140. rune inherits this Kelvin for its pure subset. rune's dynamic extension layer carries its own Kelvin number and can evolve independently.

```
trident  K140    :: working spec, mutations expected
trident  K50     :: production spec, narrow corrections only
trident  K0      :: frozen forever — the planetary commitment

rune-core K0     :: inherits trident freeze
rune-dyn  K140   :: dynamic extension layer — evolves above the frozen core
```

surface registers (classic vs pure spelling) can evolve at higher kelvins — adding a syntactic spelling changes neither the AST nor the [[Nox]] noun.

---

## solid-state interpreter for [[cyb/robot]]

the [[cyb/robot]] is a kernel function:

```
kernel : (subject, event) -> (new-subject, effects)
```

state is the subject. inputs are events from the [[cybergraph]], [[radio]], the user. outputs are effects — [[cyberlinks]], signals, messages, UI updates. pure. deterministic. replayable.

events persist as a log. state at time `t` is the fold of `kernel` over events up to `t`. crash recovery is replay. upgrades are events whose payload includes new kernel code installed via `=.` subject mutation.

---

## reference + jets

every primitive has a reference Nox formula and an optional jet. the runtime recognizes the formula by structure or hint and substitutes the jet. reference is slow but always correct. jet is fast and verified equivalent.

```
Nox reduction (tree rewriting)
  │
  ├── pure jets → proven computation
  │     fma, ntt, p2r, lut, normalize, rank, link...
  │
  ├── host jets → practical computing
  │     ├── wasm(module, fn, args)   → wasmi execution
  │     ├── gpu(shader, data)        → wgpu compute dispatch
  │     └── infer(model, input)      → [[glia]] inference
  │
  └── hint → async input from the world
        ├── network event ([[radio]])
        ├── user input ([[cyb]] UI)
        ├── timer (epoch tick)
        └── [[cybergraph]] change ([[particle]]/[[cyberlink]] event)
```

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

---

## relation to other layers

| layer | role |
|-------|------|
| [[cybermark]] | the address sigil set — addresses everywhere in rune |
| [[Nox]] | the target — every rune AST lowers to a Nox noun |
| [[hoon]] | the inspiration for the subject model and pure register |
| [[zheng]] | the prover — every pure Nox trace becomes a proof |
| [[cyb/robot]] | the canonical consumer — kernel functions in rune |
| [[semcons]] | first-class rune constructs |
| [[bbg]] | event log storage substrate |
| [[radio]] | source of async hints from the network |
