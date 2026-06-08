# relation to [[trident]] and [[Rs]]

[← specification index](README.md)

---

[[trident]] is the substrate. rune is [[trident]] extended. one grammar lineage, two tools.

| dimension | [[trident]] | rune | [[Rs]] |
|-----------|-------------|------|--------|
| primary purpose | provable consensus-critical code | dynamic personal/agent code | native-speed jets |
| type system | mold-based, mandatory static enforcement | mold-based, optional enforcement | Rust-subset static |
| dynamism | none — total determinism | eval, hint, host as first-class | none |
| provability | mandatory — every program produces a [[zheng]] proof | tiered — pure: unconditional; reactive: conditional on event log; host: conditional on host results | reference + jet equivalence verified at compile time |
| evaluation | AOT compiled to optimized .nox | interpret first, compile in background | AOT compiled to native, deployed as jet |
| launch latency | slow (compile + prove) | instant (parse + lower + walk) | n/a — runs as called |
| audience | protocol authors, dialect writers | agent developers, scripters | runtime engineers |

rune grammar = [[trident]] grammar + three dynamic forms (hint, host, eval). [[trident]] is self-contained; rune depends on [[trident]]'s grammar spec, not its toolchain.

## why [[trident]] stays dynamic-free

[[trident]]'s proof-mandatory guarantee requires total determinism. the three rune extensions break this:

- `hint`: proof becomes conditional on the event log — trident requires unconditional proofs
- `host`: result is an asserted witness — the host runtime carries no [[Nox]] trace
- `eval`: proof depends on runtime content — formula is unknown at compile time

## the trident subset — explicit scope

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
