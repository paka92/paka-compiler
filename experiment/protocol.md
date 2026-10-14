# M3 experiment protocol

Frozen at the `protocol` tag. It collects what DECISIONS.md M3-01 to M3-04 already settled and
adds no decision. Reasons live there. Counted runs start only from the commit tagged `run`, with a
clean tree (`harness/src/commit.rs`). The numbers in the `open` list at the end are not settled.

## What a run is (M3-01)

One run is one task in one language, one `claude -p` process. The model answers with a whole
program in one reply and gets build and test feedback until it passes or has made 8 submissions.
One edit replaces the whole file in all three languages. Paka's per-definition Find, View and Edit
tools are not measured; nothing about GUI, build speed or run speed is measured.

Model and settings (M3-02.1, M3-01.7):

```
model:    claude-sonnet-5-5   (checked in every reply; any other model voids the run)
effort:   medium, `--effort medium` on every call
thinking: provider default, counted as output tokens
sampling: provider default
flags:    claude -p --model claude-sonnet-5-5 --effort medium --system-prompt "<run rules>"
          --tools "" --setting-sources "" --strict-mcp-config --no-session-persistence
          --disable-slash-commands --input-format stream-json --output-format stream-json --verbose
never:    --bare --fallback-model --max-budget-usd --resume --continue
```

## Toolchains (M3-01 build lines; flags for the local install in `harness/README.md`)

| Language | Version | Build | Run |
|---|---|---|---|
| Paka | `paka` 0.0.0, built from the `run` commit | `paka check main.paka` | `paka run main.paka <args>` |
| Rust | rustc 1.96.1 (31fca3adb 2026-06-26) | `rustc --edition 2024 -C debug-assertions=on -C overflow-checks=on -o main main.rs` | `./main <args>` |
| TypeScript | tsc 7.0.2, `@types/node` 22.20.5 (`harness/ts/package-lock.json`), node v22.22.1 | `tsc --strict --target es2022 --module commonjs --types node --typeRoots ts/node_modules/@types --noEmitOnError --skipLibCheck --rootDir . --outDir out main.ts` | `node out/main.js <args>` |

A build passes when its line exits 0. Every file is `main.paka`, `main.rs` or `main.ts`. Rust uses
the standard library only; TypeScript uses Node's built-in modules only.

## Prompt and feedback (M3-01)

- One system text, the same for all languages except language name, file name and build line.
  Paka runs add the whole Paka reference and one line saying the app may list any grant the
  reference names. The first message is the task statement with its 2 visible cases.
- A reply holds exactly one backtick-fenced code block. None, several, or a reply cut off (stop
  reason `max_tokens`, or over 32,000 output tokens) is a failed submission and counts.
- Arguments are shown JSON-quoted.
- A failed build returns the compiler text unchanged, cut after 8,000 bytes with a note; warnings
  only when the build fails. A working build returns each failing visible case in full (stdout
  and stderr each cut after 2,000 bytes) and the hidden cases as counts by first cause: over time
  or memory, wrong exit status, wrong output, wrong written file. No hidden text is shown.
- Every case runs in a fresh directory, environment empty except `TZ=UTC` and `LC_ALL=C`.
- A submission passes when it builds and every visible and hidden case matches: stdout byte for
  byte, the exit status, and every file the statement says is written. Stderr is not compared.
- Holes, examples and in-file tests are allowed and add no step.
- Limits: 10 s and 1 GB per case, 120 s per build.
- Error status: a defined error prints its message on stdout and exits with status 20, in every
  task.

## Attempts, voids, reruns (M3-01.2, M3-03)

- 8 submissions per run: submission 1 is the first try, 2 to 8 are edits 1 to 7.
- A run that never passes counts 8 edits, a failed first try and its real output tokens.
- A harness fault voids the attempt and the run goes again from the same commit. A run has at
  most 3 void attempts; a third pauses the experiment for the owner. A void from the usage cap
  does not count towards the 3, and reaching a cap stops the batch.
- Harness faults: provider error, usage limit reached mid-run, refused or empty reply, lost
  network, sandbox failure, harness crash, full disk, failed first-call token guard, compaction,
  a model call past its time limit. A wrong, slow or cut-off program is not one.
- A run lost to a fault is ruled `void` and rerun in a new rep folder (B-Rust-2 void, rerun as
  B-Rust-3), so the counts meet again.
- After the `run` tag nothing changes. A new `run` tag and a full rerun need the owner's yes.
- Calibration swaps (M3-03.4): a task is swapped when its 6 calibration runs (3 Rust, 3
  TypeScript) all pass first try, or none solves. The replacement is the unused reserve task of
  the same class, else the next in reserve order. The final list may be tagged `run` only if
  Rust's first-try rate on it is 30% to 85% and TypeScript's at most 85%; otherwise the owner
  decides. Swaps amend the lists below before the `run` tag.

## Tasks (M3-04)

Written fresh; 2 visible and 8 hidden cases each; folders in `experiment/tasks/<id>/`.
ASCII only, files under 200 lines, stated values within plus or minus 2^53 - 1, no clock,
randomness, network or standard input.

Machine-read lines (the metrics tool checks run folders against `counted:` and `reserve:`):

```
counted: T1 T2 P1 P3 S2 S3 G1 G2 R1 R3 N1 N2 F1 F2
reserve: T3 V2 N3 V6
practice: X1 X2
```

Counted (14), two per class. Tag = expected to cost Paka (M3-04.4); 5 of 14.

| Task | Class | Tag | Outline |
|---|---|---|---|
| T1 | text processing | | Wrap and justify paragraphs to a width, spread spaces left first, split long words |
| T2 | text processing | | Column table from quoted comma-separated rows, numeric columns right-aligned, total line |
| P1 | parsing and interpreters | | Integer calculator: precedence, brackets, variables across lines, truncating division, error column |
| P3 | parsing and interpreters | tagged | Nested brace config with strings, escapes, lists; sorted flat key paths; exact first error |
| S2 | state machines | | Ledger with holds in whole cents: open, deposit, hold, capture, release, transfer |
| S3 | state machines | | Comment stripper for C-like text: strings, escapes, nested block comments, line joins |
| G1 | grids and dynamic programming | | LCS line diff with a fixed tie-break and hunk format |
| G2 | grids and dynamic programming | tagged | Falling sand on a grid until nothing moves or a step limit |
| R1 | graphs by list index | | Build order with ASCII tie-break; cycle printed in a fixed form, exit 20 |
| R3 | graphs by list index | | Islands and bridges by removing each link and recounting |
| N1 | integer and float reports | | Invoice in cents: discounts, tax in hundredths of a percent, round half up per line |
| N2 | integer and float reports | tagged | Per-group count, mean, median, standard deviation to two decimals |
| F1 | file tools | tagged | Include resolver: nested includes, each file read once, cycle chain named, merged file written |
| F2 | file tools | tagged | Split one input file into one output file per key plus an index file |

Reserve (4), in swap order; a swap brings the reserve task's own tag (T3 and V6 tagged, V2 and
N3 not):

| Order | Task | Class | Tag | Outline |
|---|---|---|---|---|
| 1 | T3 | text processing | tagged | Natural sort: digit runs as numbers, case-blind, stable |
| 2 | V2 | state machines | | Car-park billing: tariff bands, daily cap, minute arithmetic |
| 3 | N3 | integer and float reports | | Calendar 1900 to 2100: days between, weekday, working days with holidays |
| 4 | V6 | file tools | tagged | Pack files into one text archive and unpack it |

Practice (2), harness shake-down only, never in the verdict: X1 word counts (top N words, tie
rules; text); X2 numbered join (two files joined with line numbers, missing file is an error; file
tools).

## Metrics and verdict (M3-02)

Per language, over all counted runs not ruled out and not in a dropped task:

```
F = runs whose submission 1 passed / runs
E = mean over runs of edits(run)     edits = submissions - 1 if passed, 8 if never passed
T = sum over runs of output tokens (thinking included)

first-try gap = F_rust - F_paka   (points)
edits gap     = E_paka - E_rust
token ratio   = T_paka / T_rust
```

Rust is the deciding baseline; TypeScript is run and reported the same way. Bands, decided on
exact whole numbers; an edge belongs to the better band:

```
first-try gap:  held <= 10 points < close <= 20 points < lost
edits gap:      held <= 0.5        < close <= 1.0       < lost
token ratio:    held <= 1.7        < close <= 2.0       < lost
```

Verdict:

```
inconclusive  if more than 2 of the 14 tasks are dropped
stop          else if any metric is lost
go            else if all three are held
inconclusive  otherwise
```

The verdict needs a complete run set: every kept task has the same number of ended runs in all
three languages and no kept run is unfinished. Otherwise there is no verdict and the metrics tool
exits with status 20. Go means only that the rule did not stop the project; the owner records the
decision.

Not the verdict, printed as such: each metric per task; nothing dropped; tagged runs removed;
solve rate within 8 submissions; cut-off and refused replies per language; input tokens (sum of
plain, cache write and cache read) with and without the Paka reference; Paka against TypeScript.

## Caps and budget (M3-02.4, M3-02.8)

- Token caps over dry run, calibration and counted runs together: 16,000,000 input and 9,500,000
  output. The harness stops when either is reached; going on needs the owner's yes. Tokens of a
  call the watchdog killed can be missing, so the caps can undercount.
- Reply limit used: 32,000 output tokens per reply, thinking included.
- Run plan: 14 counted tasks x 3 languages x 4 reps = 168 runs, 56 per language, in batches of
  one task.
- Dry run: 2 practice tasks x 3 languages x 2 runs = 12 per pass, at most 3 passes.
  Calibration: 14 tasks x 2 languages (Rust, TypeScript) x 3 runs, plus up to 24 for swaps.

## Rulings file (M3-03, `harness/README.md`)

One ruling per line, blank lines allowed, the reason required:

```
run G2-Rust-1 void harness: sandbox failure
run N2-Paka-3 tagged reference gap: silent on the empty list
task F1 dropped tools and reference disagree
```

- `void`: the run leaves every table (a harness fault; rerun in a new rep folder).
- `tagged`: the run counts and leaves the tagged table (reference silent or unclear).
- `dropped`: every run of the task, in all languages, leaves the verdict. For a Paka mismatch
  between tools and reference, a toolchain defect, or an expected result the statement proves
  wrong; each needs the owner's approval. A reference gap is not a mismatch.

## Open

Numbers or names the decisions leave open; this file does not settle them.

- The model id as the dry run's replies report it (M3-02 names `claude-sonnet-5-5`; the dry run
  confirms it).
- The fixed CLI text, in tokens, for the first-call guard (within 50 tokens); measured in the dry
  run (M3-01.7).
- The wall-clock limit of one model call for the watchdog; M3-09 says only that there is one.
- The `claude` CLI version; not pinned by any decision.
- The Paka reference text and its token count (M3-19 trims it to 8,000 tokens if over).
- The order in which the 14 batches run and the numbering of reps.
