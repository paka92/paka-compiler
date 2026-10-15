# M3 experiment protocol

Frozen at the `protocol` tag. It collects what DECISIONS.md M3-01 to M3-04 already settled and
adds no decision. Reasons live there. Counted runs start only from the commit tagged `run`, with a
clean tree (`harness/src/commit.rs`). The last two sections settle the numbers the decisions left open.

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
counted: T4 T5 P4 P5 S3 S4 G2 G3 R4 R5 N1 N4 F3 F4
reserve: T6 S5 N5 F5
practice: X1 X2
```

Counted (14), two per class. Tag = expected to cost Paka (M3-04.4); 5 of 14.

| Task | Class | Tag | Outline |
|---|---|---|---|
| T4 | text processing | | Variable expansion with defaults expanded only when used; cycle, undefined and syntax errors in a set order |
| T5 | text processing | tagged | Multi-key line sort: field and column keys, numeric or case-blind, non-numbers last, stable reverse |
| P4 | parsing and interpreters | | Register machine: labels, call and return, truncating division, exact step limit |
| P5 | parsing and interpreters | tagged | Tiny Lisp: lexical scope, parallel `let`, `define`, six built-ins, call-depth limit |
| S3 | state machines | | Comment stripper for C-like text: strings, escapes, nested block comments, line joins |
| S4 | state machines | | Vending machine: slots, coins, credit cap, greedy change that can fail, cancel |
| G2 | grids and dynamic programming | tagged | Falling sand on a grid until nothing moves or a step limit |
| G3 | grids and dynamic programming | | Cheapest grid path with right, down and diagonal moves; three-way tie-break |
| R4 | graphs by list index | | Failure cascade in simultaneous rounds by share of failed dependencies |
| R5 | graphs by list index | tagged | Cheapest trips: least cost, then fewest roads, then earliest place by ASCII |
| N1 | integer and float reports | | Invoice in cents: discounts, tax in hundredths of a percent, round half up per line |
| N4 | integer and float reports | | Shared costs split by weight in whole cents; totals and shares per member |
| F3 | file tools | | Apply an edit script to a text file and write the result, possibly over the target |
| F4 | file tools | tagged | Rewrite links in a set of wiki pages after renames, following a rename map |

Reserve (4), in swap order; a swap brings the reserve task's own tag (T6 and F5 tagged, S5 and
N5 not):

| Order | Task | Class | Tag | Outline |
|---|---|---|---|---|
| 1 | T6 | text processing | tagged | Canvas: lines, boxes, text, fill and copy on a character grid |
| 2 | S5 | state machines | | Line editor with grouped undo, capped undo list, redo |
| 3 | N5 | integer and float reports | | Grade book: weighted scores, grades and ranks |
| 4 | F5 | file tools | tagged | Split a book into one file per chapter and write a table of contents |

Amendment after calibration (M3-03.4): the first calibration found 11 of the first 14 tasks too
easy (every run passed first try) against 4 reserves. S3, G2 and N1 stay; the other eleven
counted tasks and all four reserves were replaced by the harder tasks above. The replaced
tasks' folders stay in `experiment/tasks/` and their calibration logs in
`experiment/runs/calib-1/`; they are not part of the experiment.

Owner ruling on the second calibration (`experiment/runs/calib-2/`): first try was Rust 29 of 42
(69%) and TypeScript 33 of 42 (79%), inside the gate. T5, P4, S4, G3, R4, R5 and N4 passed first
try in all 6 runs, 7 swaps due against 4 reserves. The owner keeps the list as it stands: the
list-wide gate is met, and a task Rust always passes still shows any Paka shortfall in the
first-try gap. F4's error line was clarified (every run printed the placeholder `OLD` itself)
and F4 calibrated again in `experiment/runs/calib-3/`: 4 of 6 passed first try, so it stays.

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

## Settled here

Numbers the decisions left open, set by the owner with this file:

- Watchdog limit: one model call gets 10 minutes of wall-clock time (`Setup.call_limit`); a call
  past it is a harness fault.
- `claude` CLI: version 2.1.299, recorded, not pinned.
- Batch order: the 14 batches run in the order of the `counted:` line. Reps are numbered 1 to 4 per
  task and language; a rerun takes the next free number (M3-03).

## Measured in the dry run (M3-19)

- Model id the replies report: `claude-sonnet-5-5`.
- Fixed CLI text: 784 input tokens per first call. The first-call guard compares each run's first
  call with the first call of an earlier run of the same task and language, within 50 tokens.
- Paka reference: `spec/paka.md` whole, about 9,400 tokens (Paka first call minus Rust first call
  on X1). Amendment: it is sent whole, not trimmed to 8,000 tokens, because M3-01 sends the whole
  reference and input tokens are outside the verdict.
