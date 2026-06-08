# execution architecture

[← specification index](README.md)

---

## workload split

| workload | profile | right strategy |
|----------|---------|----------------|
| REPL, scripts, ad-hoc | run once, dies fast | interpret directly |
| event handlers, UI | run often per session, short bursts | interpret + jet substitution on hot ops |
| agent kernels ([[cyb/robot]]) | run for years | interpret first, compile in background, swap when ready |
| [[dialects]] (consensus-critical) | called by every [[neuron]], deterministic + fast | compile AOT through [[trident]], deploy as proven .nox |
| inner-loop primitives | hot, narrow, performance-critical | written in [[Rs]], AOT-compiled, called as jets |

the source language is the same in all five rows. choice is per-particle, opportunistic, and reversible.

---

## the pipeline

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
