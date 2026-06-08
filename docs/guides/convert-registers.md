# convert between registers

rune has two syntactic registers — *classic* (Rust-shaped) and *pure*
(digraph). Both parse to the same AST, so conversion is mechanical and
lossless. `rune fmt` does it.

All commands run from `rs/`.

## classic → pure

```sh
echo '6 * 7' > prog.rune
cargo run --bin rune -- fmt --to=rune prog.rune
```

```
(mul 6 7)
```

## pure → classic

```sh
cargo run --bin rune -- fmt --to=rust prog.rune
```

```
6 * 7
```

With no `--to` flag, `fmt` defaults to the classic (`rust`) register.

## what is preserved

Conversion never changes the AST or the lowered Nox noun — only the spelling.
A program means exactly the same thing in either register; nothing forces you
to move between them. See [two registers, one AST](../explanation/two-registers.md)
for why both exist.

## note on `eval`

`rune eval` parses the **classic** register. To evaluate pure-register source,
convert it first or put it in a file for `rune run`.
