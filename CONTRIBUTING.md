# Contributing

Rules that hold from the first commit:

- Standard library only. No third-party crates.
- No `HashMap` or `HashSet`: their iteration order is random. Use `BTreeMap` and `BTreeSet`.
- No `f32` or `f64` method whose precision is not deterministic (`sin`, `powf`, `exp`, `ln` and the
  rest of the list in `clippy.toml`). Clippy rejects them.
- Overflow checks stay on in dev and release.
- `unsafe` is forbidden in the `paka` crate. Only one module of `paka-rt` may use it.
- Expected outputs are written by hand and committed before the code that produces them.

Before a pull request:

```
cargo build --release --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --check
```
