# subject — the noun that holds everything

[← specification index](README.md)

---

every rune evaluation runs against a subject — a [[Nox]] noun containing the entire visible environment.

```
subject = [
  ~self  : @neuron       :: who I am                axis 2
  ~now   : @da           :: current time             axis 6
  ~here  : #             :: where in the graph       axis 14
  ~caps  : capabilities  :: what I am allowed to do  axis 30
  ~code  : core          :: the program itself        axis 62
  ~libs  : core          :: imported libraries        axis 126
  ~mem   : ...           :: persistent state          axis 254
  ~world : graph-slice   :: the cybergraph I can see  axis 255
]
```

subject is a right-nested cons tree. axis navigation follows binary tree addressing: `axis(n) = (1 << (slot+2)) - 2` for slots 0–6; ~world is the rightmost tail at axis 255.

every name resolves through the subject. `~self` finds the neuron slot. `~mem.counter` reads from persistent state. `#cyber/truth` resolves through `~world` then path.

three consequences fall out of this design:

1. metaprogramming is trivial — the program lives in `~code`, manipulate it like any other noun
2. eval is trivial — evaluate any formula against any subject via `.*`
3. hot upgrade is trivial — receive an event whose payload includes new `~code`, install with `=.`

## cybermark unifies with subject traversal

`#cyber/truth` is a wing through the subject's `~world` slot. when you write `#cyber/truth.rank`:

1. `#cyber/truth` resolves through `~world` in the subject
2. `.rank` pipes through a computation

the dot is the same dot. addresses ARE computations against the subject.
