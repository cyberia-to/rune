# check molds

A **mold** is a type expressed as a normalizing function. `rune check` runs the
mold checker over a program without executing it, reporting type mismatches it
can catch statically.

All commands run from `rs/`.

## a passing check

```sh
echo '6 * 7' > prog.rune
cargo run --bin rune -- check prog.rune
```

```
ok
```

## a failing check

Bind a text literal to a numeric mold and the checker rejects it:

```sh
printf 'let x: @ud = "hi"\nx' > bad.rune
cargo run --bin rune -- check bad.rune
```

```
mold error: text literal bound to `@ud`, expected a text mold (@t)
```

`check` exits non-zero when it finds errors, so it composes in scripts and CI.

## what is checked today

The checker validates literal-to-mold compatibility at typed binding sites —
for example a text literal requires a text mold (`@t`), and a cell cannot be
bound to `@ud`. The base molds (`@ud @da @t @flag @p`) and the strata algebra
molds are recognised. See [model.md](../../specs/model.md) for the full mold
system.
