# your first rune program

By the end you will have evaluated arithmetic, rendered a piece of UI, and
seen the same program written two ways. Every command here is real — run them
as you read.

## build the CLI

The Rust workspace lives in `rs/`. From the repo root:

```sh
cd rs
cargo build --bin rune
```

For the rest of this tutorial we invoke it with `cargo run --bin rune --`.
(If you prefer, the built binary is at `rs/target/debug/rune`.)

## step 1 — evaluate an expression

```sh
cargo run --bin rune -- eval '6 * 7'
```

```
42
```

`eval` parses one expression, lowers it to a [Nox](../../specs/README.md) noun,
and reduces it. There is no build phase between source and result — this is
rune's *instant start*.

## step 2 — render a piece of UI

rune programs can evaluate to a **chunk** — a unit of terminal UI. Try the
`text` constructor:

```sh
cargo run --bin rune -- eval 'text("hello")'
```

```
(# t) hello
```

The `(# t)` is the chunk's sigil and render kind — here, a text chunk. `error`
makes an error chunk, and `col` lays several out in a column:

```sh
cargo run --bin rune -- eval 'col(text("a"), error("b"))'
```

```
(# t) a
(! e) =s~tlevel#terror=s~tsource#t =s~tmessage#tb
```

Two chunks, one per element.

## step 3 — emit through an act

The chunks above were the program's *value*. A program can also **emit** UI as
it runs, through an act — the boundary where pure evaluation asks the host to
do something:

```sh
cargo run --bin rune -- eval 'emit(text("live"))'
```

```
(# t) live
```

`emit` is performed by the CLI's host as the program runs. In the
[cyb](../../../cyb) terminal the same act renders straight into the screen.
See [acts and the ward](../explanation/acts-and-the-ward.md) for the why.

## step 4 — the same program, two registers

The `6 * 7` you wrote is the *classic* register. rune has a second, *pure*
register. Write the classic form to a file and convert it:

```sh
echo '6 * 7' > prog.rune
cargo run --bin rune -- fmt --to=rune prog.rune
```

```
(mul 6 7)
```

Same AST, same Nox noun — only the spelling differs. `--to=rust` converts back.

## where to go next

- [render prysm chunks](../guides/render-prysm-chunks.md) — the full chunk vocabulary
- [two registers, one AST](../explanation/two-registers.md) — why both exist
- [the specification](../../specs/README.md) — the complete model
