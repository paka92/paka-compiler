# Harness

Tools for the M3 experiment, std only. `src/task.rs` loads a task folder; `src/runner.rs` builds a
submission and judges it on the task's cases.

## Task folder

```
statement.md        the task statement, not empty
cases/<name>/       visible cases, at least one
hidden/<name>/      hidden cases, at least one
```

A case folder holds:

```
args      one argument per line; none when absent
in/       files put in the working directory before the run; optional
stdout    the exact standard output; required
status    the exit status as a number; 0 when absent
out/      files the program must write, with their exact content; optional
```

The experiment tasks live in `experiment/tasks/<id>/`. Each also holds `working.md`, the hand
working for every expected result; the loader ignores it, and like `hidden/` it is never shown to
the model.

`in/` and `out/` hold plain files only. Anything else in a case folder, or a missing `stdout`, is
malformed, and the whole task is refused. Standard error is not compared.

## Build and run

Every file is `main.<ext>`. Each case runs in a fresh directory holding its `in/` files, with the
environment cleared except `TZ=UTC` and `LC_ALL=C`. Limits: 120 seconds to build, 10 seconds
per case.

| Language | Build | Run |
|---|---|---|
| Paka | `paka check main.paka` | `paka run main.paka <args>` |
| Rust | `rustc --edition 2024 -C debug-assertions=on -C overflow-checks=on -o main main.rs` | `./main <args>` |
| TypeScript | `tsc --strict --target es2022 --module commonjs --types node --typeRoots ts/node_modules/@types --noEmitOnError --skipLibCheck --rootDir . --outDir out main.ts` | `node out/main.js <args>` |

The build lines follow M3-01 plus the flags for the local install (`--typeRoots`, `--skipLibCheck`, `--rootDir`, `--outDir`).

Install `tsc` once with `ts/check.sh`.

## Verdicts

A case passes when standard output and the exit status match and every file in `out/` was written
byte for byte. Otherwise its verdict is the first of: `Timeout`, `Stdout`, `Status`, `Files`.
