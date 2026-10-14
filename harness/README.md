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

## Sandbox

Builds and cases run under `bwrap` and `prlimit` from util-linux; both must be in `/usr/bin`.

- No network: the sandbox has its own network namespace with only loopback.
- Reads: `/usr`, the toolchains and, for a case, its build, all read-only. The task folder,
  hidden cases, the harness and home directories are absent.
- Writes: only the build or case directory.
- Memory: 1 GB per process for a case, 4 GB for a build (the Rust linker needs more than 1 GB).
- Files: 64 MB per file. Processes: 256 per sandbox, so a fork bomb stops.
- Output: standard output and standard error each stop at 1 MB, and the program is killed.
- Time: at the limit the sandbox is killed, and with it every process the program started.

A program killed by a signal has status 128 plus the signal number: 134 when it aborts on
running out of memory, 153 when it writes a file past the cap. Memory is capped per process,
not per sandbox.

## Verdicts

A case passes when standard output and the exit status match and every file in `out/` was written
byte for byte. Otherwise its verdict is the first of: `Timeout`, `Stdout`, `Status`, `Files`.
