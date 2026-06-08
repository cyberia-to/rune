# acts and the ward

rune evaluation is pure. The 18 Nox patterns cannot touch the world — no file
written, no graph read, no pixel drawn. So how does a rune program *do*
anything? Through an **act**.

## an act is a request, not a deed

When the interpreter reaches an act — opcode 16 with an act tag (`emit`,
`query`, `link`, `seal`, `subscribe`, `host`) — it does not perform it. It hands
the act to a **host**: `(act, args, caps) → result`. The host performs the
effect and returns a noun, which the interpreter splices back in and continues.

```
rune program ──evaluates──> act request ──> host performs ──> result ──> continue
```

This is the whole trick. The evaluator stays pure and provable; the host is the
single, explicit place where consequence happens. A pure `eval` (no host) simply
no-ops every act, so the same program is inert or active depending only on which
host runs it.

The CLI demonstrates this: `rune eval 'emit(text("hi"))'` runs with a host that
collects emitted chunks and prints them. Swap the host and the same `emit` draws
to a terminal, a log, or nothing.

## the ward performs the acts

rune defines acts but does not decide whether they are allowed. That belongs to
cyb, in a boundary called the **ward** (see `cyb/root/ward.md`). The ward is to
acts what prysm is to rendering: one runtime-blind boundary that many runtimes
feed. It is not a rune feature — Nox, rune, Inf, glia, and wysm all route their
acts through the same ward.

The program carries its granted capabilities in the subject's `~caps` slot
(axis 30). The interpreter reads `~caps` before calling the host, so the host —
the ward — can check the act against the rights the program actually holds. A
program can only narrow its own caps, never widen them.

```
program ──act + ~caps──> ward.perform(act, args, caps) ──> result | denied
```

## what is built today

The act mechanism is real: the `Host` trait, `eval_with_host`, and the
`emit`/`query`/`link`/`seal`/`host` act tags are implemented, and the cyb
terminal performs `emit` live. The ward as an enforcement boundary — gating acts
on `~caps`, the deny-by-default policy, the prompt UX — is designed and
documented but not yet wired; today the terminal host performs `emit` ungated.
The seam exists, so adding the gate is a contained change.

## see also

- [execution.md](../../specs/execution.md) — pure, reactive, and host modes
- `cyb/root/ward.md` — the permission boundary in full
