# Finish M2 — subject model + cybermark addresses

## state (verified)

The whole mechanism already exists:
- `~self`/`~now`/…/`~world` lower to axis reads (rs/lower Sym match, hardcoded 2/6/14/…/255)
- `#path` addresses lower to `[17 [path-atom [0 255]]]` — scry over `~world`
- `scry_world` walks `[[key val] … 0]`, matches key == path
- hemera hash is real (opcode 15); `string_to_atom = hemera::hash(s)[..8]` (private in lower)

## gaps

1. `Subject::minimal()` is all `Atom(0)` → `~self` → 0, `~world` empty
2. the world-key hash (`string_to_atom`) is private to the lowerer → nothing else can
   build matching `~world` keys
3. no `~world` builder, no populated subject

## work

1. **rune-subject** (+hemera dep):
   - `pub fn path_atom(s) -> u64` — the canonical address→world-key hash (hemera)
   - `pub fn world(entries: &[(&str, Noun)]) -> Noun` — build `[[key val] … 0]`, key = path_atom
   - `Subject::standalone()` — real-ish: self = neuron atom, here = home, now/world host-filled
   - keep `minimal()` (zeros) for pure tests
2. **rune-lower** (+rune-subject dep):
   - `string_to_atom` → `rune_subject::path_atom` (one source of truth for the key)
   - hardcoded slot axes → `rune_subject::axis::{SELF,NOW,…}` (kills magic numbers)
3. **cli**: a `subject()` helper — `standalone()` with `~now` = real unix time; used by
   eval / run / repl instead of `minimal()`
4. **tests**:
   - rune-interp integration: build subject with `world([("cyber/truth", 42)])`, parse+lower+eval
     `#cyber/truth` → 42 (scry threading, the completion signal)
   - cli integration: `rune eval '~self'` → non-zero neuron
   - rune-subject unit: path_atom stable, world layout

## completion signal

`rune eval '~self'` returns a neuron atom; `#cyber/truth` scry-resolves through a
populated `~world`. Then update PLAN.md M2 + CHANGELOG.
