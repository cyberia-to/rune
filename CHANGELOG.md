# Changelog

All notable changes to rune are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com); rune is pre-1.0 and tracks
milestones M1–M10 toward the Kelvin freeze (see `PLAN.md`).

## [Unreleased]

### Added

- **Two registers, one AST (M4)** — classic (Rust-like) and pure (Hoon-style
  digraph) both parse to the same AST and lower to the same Nox noun (verified:
  `fn double(x: @ud) { x * 2 }` ≡ `(|= x=@ud (mul x 2))`). `fmt` is
  bi-directional (`--to=rune`/`--to=rust`, either input register). Pure register
  executes: `rune eval --pure …`, or a `:: register: rune` file pragma.
- **Instant-start interpreter** — tree-walking evaluator over all 18 Nox
  reduction patterns; parse → lower → reduce with no compilation phase (M1).
- **Subject model + cybermark scry (M2)** — `~self … ~world` resolve to their
  axis (reserved names work with or without `~`); `#path`/`@neuron`/`~name`
  addresses scry the `~world` slot via `look`. `rune_subject::path_atom` is the
  one address→key hash (real hemera), shared by the lowerer and the
  `rune_subject::world(&[(path, val)])` slice builder. `Subject::standalone()`
  gives a resolvable `~self`/`~here`; the CLI injects a live `~now`.
- **Mold system (M3)** — types as normalizing functions; base molds (`@ud @da @t
  @flag @p @tas @neuron #`) plus the strata algebra tower (`@nebu @kuro @jali
  @trop @genies`); `rune check` validates without running. The checker runs the
  real molds on literals, so each enforces its own constraint (`@flag = 2`,
  `@ud = [1 2]` are caught), not just hard-coded heuristics.
- **Acts** — the pure/world boundary: opcode-16 act tags (`emit query link seal
  subscribe host`) are performed through a `Host` trait (`eval_with_host`); pure
  `eval` no-ops them via `DenyHost`. The seam the cyb ward plugs into (M5).
- **Cell entrypoint** — `event::run_cell` reactive loop: a gate invoked per
  event, emitting as it runs, threading evolving state (M5).
- **rune↔prysm bridge** (`rune-prysm`) — decodes a chunk-noun into `tape::Chunk`s,
  the exact wire format prysm renders. `rune eval` shows emitted/returned chunks.
- **CLI** (`rune`) — a **REPL** (no-arg) built on reedline (the nushell line
  editor): persistent history, Ctrl-R search, tab-completion (builtins, acts,
  bound names), syntax highlighting, multi-line input, and a plain fallback for
  piped input. Persistent `let` bindings; ANSI chunk rendering; meta-commands
  `:nox` (show the lowered Nox formula), `:type`, `:ast`, `:load`, `:env`,
  `:reset`, `:help` (categorized). Plus `run`, `eval`, `fmt`, `check`,
  `--version`. End-to-end tests pin the command + REPL contract.

### Notes

- Hint parking exists via `eval_step`/`resume`; deep cell suspend/resume (CPS) is
  still pending (M5b).
- The compile tier (`rune-compile`, M6) is a stub — `TierRouter` routes and caches
  but does not yet call trident. The interpreter path is the contract.
