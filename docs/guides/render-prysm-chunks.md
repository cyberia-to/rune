# render prysm chunks

A rune program renders UI by evaluating to a **chunk-noun**, which
`rune-prysm` decodes into `tape::Chunk`s — the wire format prysm draws to wgpu,
ansi, or html. `rune eval` shows the decoded chunks.

All commands run from `rs/`.

## constructors

| call | makes | output sigil |
|------|-------|--------------|
| `text("…")` | a text chunk | `(# t)` |
| `error("…")` | an error chunk | `(! e)` |
| `anno("…")` | an annotation | |
| `log("…")` | a log line | |
| `button("label", "target")` | a button | |
| `col(a, b, …)` | a column — one chunk per element | |

## a single chunk

```sh
cargo run --bin rune -- eval 'text("hello")'
```

```
(# t) hello
```

## several chunks in a column

```sh
cargo run --bin rune -- eval 'col(text("first"), text("second"))'
```

```
(# t) first
(# t) second
```

`col` yields one chunk per element, in order.

## emitting versus returning

A chunk can be the program's **value** (as above) or it can be **emitted** as
the program runs, through the `emit` act:

```sh
cargo run --bin rune -- eval 'emit(text("a"))'
```

Emitted chunks appear first, then the final value's chunks. Both feed the same
`prysm::dispatch`. The difference matters once a program emits many chunks over
time — see [acts and the ward](../explanation/acts-and-the-ward.md).

## non-UI values

A program whose value is not a chunk-noun — plain arithmetic, say — prints as a
noun instead:

```sh
cargo run --bin rune -- eval '6 * 7'
```

```
42
```
