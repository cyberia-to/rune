# examples

[← specification index](README.md)

---

## double a number

```rust
// classic
fn double(x: @nebu) -> @nebu { x * 2 }
```

```
:: pure
|=  x=@nebu  (mul x 2)
```

## graph reactor — watch, rank, link

```rust
// classic
async fn reactor(p: #) {
    loop {
        let _ = hint::particle_changed(p);
        let r = !rank(p);
        if r > 0.5 {
            !cyberlink(p, ^truth, +1);
        }
    }
}
```

```
:: pure
|=  p=#
|-
=/  _   ~hint:%particle.p
=/  r   !rank.p
?:  (gth r 0.5)
  !cyberlink.p.^truth.+1
$
```

## trading agent — hint + host + pure

```rust
// classic
fn trading_agent(pair: #, model: #) {
    let threshold = 0.7;
    loop {
        let price  = hint::particle_changed(pair);
        let signal = host::infer(model, price.history);
        let conf   = Tri::normalize(signal);
        let sent   = Arc::rank(pair, 100);
        if conf > threshold {
            !cyberlink(pair, ^truth, +1);
        }
        host::gpu(chart_shader, [price, signal, sent]);
    }
}
```

```
:: pure
|=  [pair=# model=#]
=/  threshold  .7
|-
=/  price   ~hint:%particle.pair
=/  signal  ~host:%infer.model.price
=/  conf    (normalize:Tri signal)
=/  sent    (rank:Arc pair 100)
?:  (gth conf threshold)
  !cyberlink.pair.^truth.+1
~host:%gpu.[chart-shader [price signal sent]]
$
```

## a [[neuron]] as a door

```
|%
++  identity     ~self
++  follow       |=  who=@   !cyberlink:~self:who:+1
++  unfollow     |=  who=@   !cyberlink:~self:who:-1
++  pulse        |-          =.  ~now  ~hint:%tick   $
--
```

## dynamic — load and run code from the graph

```
=/  code   #~/agents/trader
=/  ast    (parse code)
=/  form   (lower ast)
=/  sub    [~self ~now ~here ~caps ~mem ~world]
.*(sub form)
```
