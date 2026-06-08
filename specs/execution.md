# three execution modes

[← specification index](README.md)

---

## pure — proven by construction

every reduction in the pure subset has a [[Nox]] trace, which has a [[zheng]] proof.

```
|=  [x=@nebu y=@nebu]
(add (mul x x) (mul y y))
```

## reactive — event-driven computation

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

## host — the only genuine trust boundary

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

# eval is intrinsic

[[Nox]] has `.*` — evaluate formula against subject. eval is one digraph, the same kind as `add` or `cons`.

```
=/  src   "(mul x 2)"
=/  ast   (parse src)
=/  form  (lower ast)
.*(.  form)
```

macros, interpreters, hot patching, runtime DSLs — all are special cases of `.*`.

---

# instant start

between source becoming available and execution starting, only parsing happens. parse → [[Nox]] noun → tree-walk reduction begins immediately. no compilation phase. latency bounded by parsing time — milliseconds for any human-scale program.

instant start enables:
- REPL-style interaction with the [[cybergraph]]
- agent kernels reacting to events with low latency
- `eval()` of dynamically-built formulas
- hot code reload by subject rewrite
- [[dialects]] evaluated on demand
- the [[cyb/robot]] starting up and being responsive immediately

instant start is non-negotiable. every architectural decision is checked against one question: does this preserve the ability to run the program the moment its source becomes available, with no phase between?
