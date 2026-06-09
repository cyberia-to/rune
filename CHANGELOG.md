# Changelog

All notable changes to rune are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com); rune is pre-1.0 and tracks
milestones M1–M10 toward the Kelvin freeze (see `PLAN.md`).

## [Unreleased]

### Added

- **Two registers, one AST** — classic (Rust-like) and pure (Hoon-style digraph)
  parsers both lower to the same Nox noun. `rune fmt --to=rune` / `--to=rust`
  converts mechanically (M1, M4).
- **Instant-start interpreter** — tree-walking evaluator over all 18 Nox
  reduction patterns; parse → lower → reduce with no compilation phase (M1).
- **Subject model** — `~self ~now ~here ~caps ~code ~libs ~mem ~world` slots with
  reserved axes; cybermark address forms in the parser (M2).
- **Mold system** — types as normalizing functions; base molds (`@ud @da @t
  @flag @p`) plus the strata algebra tower; `rune check` validates molds without
  running (M3).
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
