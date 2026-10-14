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

## Sandbox

Builds and cases run under `bwrap` (bubblewrap) and `prlimit` (util-linux); both must be in
`/usr/bin`.

- No network: the sandbox has its own network namespace with only loopback.
- Reads: `/usr` and the toolchains under `/tools`, read-only. The build directory shows as
  `/build`, the case directory as `/work`. The task folder, hidden cases, the harness and home
  directories are absent.
- Writes: only `/build` while building, only `/work` while a case runs.
- Environment: none is inherited. bwrap reads its options from standard input, so the
  sandbox's `/proc/1` shows no host path.
- Memory: 1 GB per process for a case, 4 GB for a build (the Rust linker needs more than 1 GB).
- Files: 64 MB per file. Processes: 256 per sandbox, so a fork bomb stops.
- Output: standard output and standard error each keep 1 MB; a program writing more is killed.
- Time: at the limit the sandbox is killed, and with it every process the program started.

A program killed by a signal has status 128 plus the signal number: 134 when it aborts on
running out of memory, 153 when it writes a file past the cap.

Known gaps: memory is capped per process, not per sandbox, and disk use per file, not in total.

## Verdicts

A case passes when standard output and the exit status match and every file in `out/` was written
byte for byte. Otherwise its verdict is the first of: `Timeout`, `Stdout`, `Status`, `Files`.

## Trial loop

`src/trial.rs` runs a batch: per run one `claude -p` process (`src/client.rs`), the statement
with the visible cases as the first message (`src/prompt.rs`), then up to eight submissions. Each
reply is a submission; one without exactly one code block, or cut off (stop reason `max_tokens`
or more than 32,000 output tokens), fails and counts. Code blocks are fenced with backticks only
(no `~~~`); an opening fence may name a language, and a line of backticks only, at least as many
as opened, closes the block. Each program is judged on every case; the feedback gives the build
text, cut after 8,000 bytes, or each failing visible case in full and the hidden cases as counts.
Arguments are shown JSON-quoted. Hidden cases and `working.md` never reach the model.

Every run and attempt has a folder, plain files only:

```
<root>/<task>-<language>-<rep>/<attempt>/log       raw log, one JSON object per line
<root>/<task>-<language>-<rep>/<attempt>/claude/   the process's working directory:
                                                   stream.log (every line sent and read),
                                                   stderr.log and claude.pid
```

`log` lines, each written and synced before the next step: `start` (system text, model, effort),
then per submission `call` (message sent, time), `reply` (text, tokens, cut off, time, duration),
`submission` (program or why there is none) and `report` (build and its full text, diagnostic
codes, every case), and last `end` (passed, submissions, edits, tokens) or `void` (reason, fault).
Times are milliseconds since the Unix epoch.

The process runs in its own process group. A watchdog kills the group when a model call, writing
the message included, passes its wall-clock limit; the run is void. Any error during a run voids
it: process death, compaction, provider error, refusal, empty reply, the usage cap, a failed
first-call guard. A void run is run again; a third void attempt stops the batch with `Paused`.
Voids from the usage cap do not count towards the three, and reaching a cap stops the batch.

A run's `end` line counts only its last attempt's tokens; a void attempt's tokens live in the
usage ledger and its `stream.log`. Tokens of a call the watchdog killed, or of a process left
running by a killed harness, never reach the ledger, so the caps can undercount.

Resume: run the same batch again. A run whose last attempt logged `end` is skipped. An attempt
whose log has neither `end` nor `void`, or ends in a torn line, was cut short by a kill: its
process group is killed if its process still runs in the attempt's folder, the log is closed as
void, and the run goes on with its next attempt.

## Metrics

`cargo run --release -p harness --bin metrics -- <runs-root> <rulings-file>` turns the logs under
a runs root (the folders of "Trial loop") and a rulings file into the three metrics and the
verdict of M3-02. A run counts through the `end` line of its last attempt that ended; void
attempts and runs with no ended attempt are ignored, and the latter are listed at the end. Per
language, over all counted runs not ruled out and not in a dropped task:

```
first-try  share of runs whose submission 1 passed
edits      mean edits per run (8 for a run that never passed, as the log says)
output     sum of output tokens
```

It prints these per language, per task (every task, dropped ones included), the verdict, and
two tables marked "not the verdict": nothing dropped, and tagged runs removed. The verdict
compares Paka with Rust: first-try gap held up to 10 points, close up to 20, lost above; edits
gap 0.5 and 1.0; token ratio 1.7 and 2.0. More than 2 dropped tasks gives `inconclusive`; else
any metric lost gives `stop`, all three held `go`, anything else `inconclusive`. Comparisons
use whole numbers, so a gap exactly on a line is held or close, never decided by rounding.

The rulings file has one ruling per line, blank lines allowed; the words after the third are a
free reason:

```
run G2-Rust-1 void harness: sandbox failure
run N2-Paka-3 tagged reference gap: silent on the empty list
task F1 dropped tools and reference disagree
```

- `void`: the run is left out of every table.
- `tagged`: the run counts in the verdict and is left out of the tagged table.
- `dropped`: every run of the task, in all languages, is left out of the verdict tables.

A run is named by its folder, `<task>-<language>-<rep>`. A ruling naming no run or task, or any
other line, is an error. The fixture is in `tests/metrics`.
