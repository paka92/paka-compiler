# Paka

A programming language for code written by AI agents.

People describe what they want and decide what a program may do; an agent writes and reads the
code. The design and its reasons are in the [genesis paper](GENESIS.md).

**Status:** the language is defined, and a checker and an interpreter run it. Dev mode, the live
GUI and release mode are being built, in that order.

## What works

- [`spec/paka.md`](spec/paka.md): the whole language on one page.
- `paka check`: parses and type-checks the full language of the reference.
- `paka run`: runs headless programs in an interpreter.
- `paka build`: compiles the milestone 1 subset to a native binary through LLVM IR and `clang`.
- `paka examples`: runs a program's in-file examples.

```paka
enum Msg { Start }

fn update(m: Msg) -> List<Effect<Msg>> {
  [Print("let there be light"), Exit]
}

app light {
  start:  Start
  update: update
  grants: [Console]
}
```

A program touches only what its `grants` allow; this one may write to the console.

## Build

Needs Rust 1.96.1 (pinned in `rust-toolchain.toml`) and, for `paka build`, `clang` on the path.
Standard library only; no third-party crates.

```
cargo build --release --locked
./target/release/paka run examples/light.paka
```

```
usage: paka run <path> [argument...]
       paka build <path> -o <output>
       paka check <path> [--json]
       paka examples <path>
```

## Benchmark

How well does a model write a language it has never seen? One model (`claude-sonnet-5-5`) solved
14 tasks in Paka, Rust and TypeScript, 4 times each, with build and test feedback. The method is in
[`experiment/protocol.md`](experiment/protocol.md).

| | Paka | Rust | TypeScript |
|---|---|---|---|
| First try | 75.0% | 83.9% | 87.5% |
| Mean edits | 0.34 | 0.20 | 0.21 |
| Output tokens | 445,593 | 192,932 | 157,316 |

Every run was solved. With no Paka code in its training, the model writes Paka nearly as correctly
as Rust, at 2.3 times the output tokens. Dev mode's one-definition tools aim at that cost; the
benchmark stays to track it.

Rebuild the numbers from the raw logs:

```
cargo run --release -p harness --bin metrics -- experiment/runs/counted <rulings-file> 9400
```

## Layout

```
paka/          the command: lexer, parser, checker, interpreter, IR emitter
paka-rt/       runtime for built programs
harness/       experiment harness: sandboxed runner, trial loop, batch and metrics commands
spec/          language reference and the milestone 1 subset
docs/          core library, ABI and decision records
tests/         conformance, case and error suites
experiment/    protocol, tasks, reference solutions and raw run logs
TASKS.md       the plan, milestone by milestone
DECISIONS.md   every design decision and its reasons
```

## Contributing

Anyone may use the project and contribute to it. Changes reach `main` only through pull requests,
and only the maintainer merges them. Read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## Licence

[MIT](LICENSE)
