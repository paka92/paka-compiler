# Contributing

Rules that hold from the first commit:

- Standard library only. No third-party crates.
- No `HashMap` or `HashSet`: their iteration order is random. Use `BTreeMap` and `BTreeSet`.
- No `f32` or `f64` method whose precision is not deterministic (`sin`, `powf`, `exp`, `ln` and the
  rest of the list in `clippy.toml`). Clippy rejects them.
- `f64` maths lives in `paka-rt/src/ops` alone: clippy rejects float arithmetic and the `f64`
  methods elsewhere, and a test fails on any other file that names `f64` or `f32`.
- A Paka Int is `paka_rt::ops::Int`. Clippy rejects `Int::get` outside that module; allow it only
  where a value leaves Paka, never to do arithmetic on the raw `i64`.
- Overflow checks stay on in dev and release.
- `unsafe` is forbidden in the `paka` crate. Only one module of `paka-rt` and the `paka-installer`
  crate may use it, each block with a `SAFETY:` comment that states its invariant.
- Expected outputs are written by hand and committed before the code that produces them.

Before a pull request:

```
cargo build --release --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --check
```
