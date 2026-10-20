# Paka build tasks

Every task needed to build Paka, cut by milestone and graded by how capable an agent it needs.
A caller reads a task, starts an agent of that level, and checks the result against `done when`.

Status: nothing is built yet. This list was cut by six planning agents and corrected after three rounds
of independent checks (one checker did a real trial build of milestone 1).

## Levels

| Level | Name | What the task is | Agent that writes it | How it is checked |
|---|---|---|---|---|
| 1 | Mechanical | Fully specified. Nothing to decide. One command proves it. | Haiku 5.5, effort low | CI, or the owner seeing it where CI itself is being built |
| 2 | Routine code | Clear spec, one module, a well-known technique, tests given or obvious. | Sonnet 5.5, effort medium | CI and the task's own tests |
| 3 | Solid engineering | Several modules, local design choices, writes its own tests, details that are easy to miss. | Sonnet 5.5, effort high | CI and one reviewer a level up (Opus 5.5, high) |
| 4 | Hard engineering | Subtle correctness, unsafe code or speed-critical. A mistake is silent. | Opus 5.5, effort high | CI and a reviewer told to break it (Opus 5.5, xhigh) |
| 5 | Design | Shapes the language or cannot be undone. The output is a written proposal, not code. | Fable 5.1, effort high: three independent proposals and a judge | The owner approves |

Relative weight per token: Haiku 1, Sonnet 20, Opus 40, Fable 100 (from list prices looked up on
2026-10-13). It only compares how fast the levels use up a usage limit; nothing is billed.
Effort `max` is never used.

## Rules for the caller

1. One task is one pull request into `development`, or one written decision the owner approves.
   Agents open pull requests; the caller merges one when its check has passed, unless the task
   says the owner merges. Only the owner merges into `main`: one pull request from `development`
   per milestone, opened by the caller when the exit check passes.
2. A task is graded by its hardest part. A task that mixes levels is split.
3. A task that fails its check twice is run again one level up. Exceptions:
   - Level 4 does not move to level 5. Run it once more with Opus 5.5 at xhigh, then split it or ask the owner.
   - `model-calls` tasks and checks the owner watches by hand are never rerun automatically.
   - A decision the owner rejects is rerun at the same level with the owner's reasons attached.
   - An `open-ended` task with work left is split by cause; that is not a failure.
4. A `decision` task ends with the owner's approval at any level. At level 4: one proposal with
   alternatives, a reviewer told to break it, then the owner. At level 5: three proposals, a judge,
   then the owner. A task's note may set another
   process (M6-23). The owner gets one plain page: the choice, what each option costs, what it locks.
5. Expected outputs are written by hand and committed before the code that produces them. The caller
   checks that order before the merge (in M1 with the command in the M1 notes). No tool has a
   mode that rewrites expectations. A level 3 reviewer of conformance or benchmark work re-derives a
   sample by hand.
6. Unsafe code that touches pointers or memory is always level 4, and its reviewer writes hostile
   inputs and runs them (the build machine has no Miri and no valgrind). Unsafe that is only an export
   attribute or one call into compiled code keeps its level; the task's note says so.
7. Level 3 to level 4 only doubles the cost. When a mistake would be silent, grade up.
8. A `conditional` task that is dropped counts as done for the tasks that need it.
9. A milestone starts when the one before it has passed its exit check on `development`. Its tasks
   are checked again against the decisions made so far before the first one is handed out.
   `needs` lists only the specific tasks a task waits for.
10. An agent is given its task, its milestone's exit check and notes, a checkout of `development`
    with its needs merged, GENESIS.md and, from M2-18 on, the language reference. Nothing else is
    assumed known.
11. Whoever runs the agents sets a usage limit. The caller checks it before each batch of agents;
    past the limit it starts no agents and stops the running ones until told to go on.
12. The plan needs no paid service: no API key and no metered usage. A task that cannot be done
    without one stops and asks the owner.

## Flags

| Flag | Meaning |
|---|---|
| `decision` | The output is a ruling the owner approves. |
| `owner` | The owner must do, see or approve something. |
| `download` | Something is fetched from the network. Needs the owner's permission first. |
| `unsafe` | Contains Rust `unsafe` code. |
| `native-only` | Exists only because the program is compiled to machine code. |
| `model-calls` | Makes live model calls. Rules 11 and 12 apply. |
| `repeat` | Run once per batch; the note says how many. |
| `conditional` | Exists only if an earlier decision or measurement says so; the note names the task. |
| `open-ended` | Size depends on what is found; split by cause when it grows. |

Task ids are stable. Gaps (M1-04, M1-05, M1-11, M2-24, M3-12, M3-13, M4-02, M4-20) are tasks removed
or merged during review. Ids with a letter were added during review.

## Milestones

| Milestone | Tasks | L1 | L2 | L3 | L4 | L5 | Owner decisions |
|---|---|---|---|---|---|---|---|
| M1 Let there be light | 17 | 1 | 9 | 4 | 3 | 0 | 2 |
| M2 The headless language, complete, in the interpreter | 46 | 0 | 7 | 21 | 8 | 10 | 10 |
| M3 The first measurement | 22 | 0 | 6 | 5 | 6 | 5 | 6 |
| M4 Dev mode for headless programs | 51 | 0 | 8 | 18 | 25 | 0 | 7 |
| M5 Live GUI on Linux | 43 | 0 | 3 | 20 | 13 | 7 | 9 |
| M6 Release mode | 34 | 1 | 7 | 10 | 14 | 2 | 3 |
| M7 Evidence and gates | 16 | 1 | 4 | 3 | 6 | 2 | 3 |
| **Total** | **229** | 3 | 44 | 81 | 75 | 26 | 40 |

## M1 — Let there be light

**Exit check.** On a clean checkout: `cargo build --release --locked --offline` succeeds; `target/release/paka build examples/light.paka -o light` writes a native executable; `./light` prints exactly `let there be light` and a newline and exits with status 0; `target/release/paka run examples/light.paka` (the interpreter) gives the same output and status; `cargo test --release --locked --offline` passes and includes the case `tests/cases/run/light_no_console` (the same program without the `Console` grant, with the output and status approved in M1-06) on both back ends; `tests/cases-pending` does not exist; CI is green on the last pull request merged to main.

- The native route is proven on the build machine: hand-written LLVM IR plus a small Rust static library, linked by clang 21, prints the line. Link 0.12 s; the whole exit check 0.71 s.
- The paper (section 9) puts the experiment before any code generation. This milestone pulls a thin native slice forward because the first milestone must be a compiled program. Tasks flagged `native-only` are that extra cost.
- The interpreter is built here too (M1-13): it is the reference for meaning from day one.
- Not in M1, and rejected rather than ignored: comments, formatter, holes, examples, match, integers, labelled arguments, more than one file, the derived database.
- `gh` is available to the caller for pull requests.
- A stage moves `tests/cases-pending/<stage>/` to `tests/cases/<stage>/` with git mv; `git diff -M --name-status origin/development...HEAD -- tests/cases tests/cases-pending` must show only R100 lines.
- Milestone 1 is complete on `development`: all its tasks are merged and the exit check passes.
- Can start first: M1-01, M1-06.
- The owner is needed in: M1-02, M1-06, M1-14, M1-15.
- Downloads: M1-02, M1-15.

### M1-01 · L2 · Create the Rust workspace with pinned toolchain, checked profiles and lint bans

- status: done
- needs: nothing
- flags: none
- done when: On a clean checkout, `cargo build --release --locked --offline`, `cargo clippy --locked --offline --all-targets -- -D warnings` and `cargo fmt --check` pass (fmt takes no such flags); Cargo.lock is committed; the pull request log shows a planted HashMap, HashSet, f64::sin and f64::powf each failing clippy; Cargo.toml shows `overflow-checks = true` for release.
- why this level: The ban lists for f64 methods and clippy settings are not given in full; a gap silently lets non-determinism in.
- note: Unsafe is forbidden inside the `paka` crate, not in workspace lints (paka-rt needs one unsafe module). Records the from-first-commit rules in CONTRIBUTING.md. rust-toolchain.toml pins channel 1.96.1 with components clippy and rustfmt. The root manifest is `[workspace]` only, resolver 3, with crate `paka/` now and `paka-rt/` in M1-16. Profiles: `overflow-checks = true` in dev and release; no `panic = "abort"`, no lto. clippy.toml: disallowed-types HashMap and HashSet; disallowed-methods every f32 and f64 method whose std documentation says its precision is not deterministic (at least sin, cos, tan, asin, acos, atan, atan2, sin_cos, sinh, cosh, tanh, asinh, acosh, atanh, exp, exp2, exp_m1, ln, ln_1p, log, log2, log10, powf, powi, cbrt, hypot; confirm against the std docs). The written rules: std only, the two bans, overflow checks on, unsafe only in paka-rt's one module, expectations hand-written first.

### M1-02 · L1 · Add CI that builds, tests and lints every pull request

- status: done
- needs: M1-01
- flags: download, owner
- done when: The owner sees the `ci` job green on this task's pull request and its log shows the four commands; the owner sees a planted failing pull request go red.
- why this level: A workflow file that runs four given commands; no decisions.
- note: Uses only `actions/checkout`; the runner's rustup reads rust-toolchain.toml. Enabling Actions and required checks is the owner's repository setting. One job named `ci` runs `cargo build --release --locked --offline`, `cargo test --release --locked --offline`, `cargo clippy --locked --offline --all-targets -- -D warnings` and `cargo fmt --check`. The owner allowed GitHub Actions on 2026-10-13. `runs-on:` is the Ubuntu release the build machine runs (26.04) if GitHub offers it, else the newest LTS it offers; pinned by version, never `-latest`. The job runs on pull requests into `development` and into `main`.

### M1-03 · L2 · Add the paka command skeleton and the diagnostic format

- status: done
- needs: M1-01, M1-06
- flags: none
- done when: cargo test passes hand-written expected lines; an integration test shows bad arguments exit 2 with usage and a missing file exits 1 with one line.
- why this level: Routine argument handling and one error type; the line format and exit statuses are given.
- note: Diagnostic line: `path:line:col: error[code]: message`; with no position, `path: error[code]: message`. Exit 1 = rejected program, 2 = bad usage. Commands: `paka run <file>` and `paka build <file> -o <output>`; in this task both only read the file. The case runner calls `paka run`. Diagnostics and usage go to standard error. Codes come from the spec. Exit 1 also covers tool failures such as a missing clang.

### M1-06 · L4 · Write the M1 subset spec: grammar, start, end and grant rules, hand-written cases

- status: done
- needs: nothing
- flags: decision, owner
- done when: Owner approves the spec: tokens, grammar, name and type rules; Print and Exit meanings, including standard output, standard error and status when a write fails or the pipe is closed; provisional start (initial state or none, first message), end and missing-grant rules; the list of diagnostic codes, unreadable file and missing clang included; and case files with expected results under `tests/cases-pending/<stage>/<name>/` for the stages lex, parse, check and run. A case folder holds the source, the expected stdout and the expected status, or for a reject its path, line, column and code; the path is the file name inside the case folder. `run/light_no_console` fixes the exact output and status of light.paka without Console. There is a reject case for each of: comment, hole, example, match, integer literal, labelled argument. The pull request adds `examples/light.paka`. The spec fixes the status when standard output cannot be written: /dev/full, and a pipe whose read end is closed. An accept case may also hold the expected standard error; without that file standard error must be empty.
- why this level: Must be exact enough to build from while settling nothing the paper leaves open beyond this program.
- note: light.paka is a headless app that answers its first message with Print of `let there be light` and then Exit, with grants [Console]. On both back ends it writes that line and a newline to standard output and exits 0. The subset is exactly what this program needs. The start, end and grant rules are provisional and follow GENESIS 3.5 (the runtime performs only the effects the grants allow) and section 4 (the runtime sends the first message), narrowed for M1: the first message has no payload, command-line arguments are not delivered, enum variants carry no payload. M2-15 makes the lasting ruling. A reject case fixes path, line, column and code, not the message wording. Exit statuses 1 and 2 are taken by the tool.

### M1-07 · L2 · Write the lexer for the spec's token list

- status: done
- needs: M1-03, M1-06
- flags: none
- done when: Unit tests compare examples/light.paka and each lexical reject in the spec against hand-written token lists and diagnostics; `paka run` on a lexical reject prints the spec's diagnostic and exits with the spec's status.
- why this level: One module, a given token list; spans and string-literal rejects are the only fiddly parts.

### M1-08 · L2 · Add the case runner that checks programs against hand-written expectations

- status: done
- needs: M1-07
- flags: none
- done when: cargo test runs every case folder under tests/cases/<stage>/; the spec's lexical reject cases pass; the lex group is moved with git mv, R100 lines only, and `tests/cases-pending/lex` no longer exists; runner self-tests report failure for each of: a wrong stdout byte, a missing final newline, a wrong status, a wrong line, column or code, a case folder with no expectation file. The runner also compares standard error of an accept case byte for byte, and its self-tests report failure for a wrong standard-error byte and for a case folder without a source file.
- why this level: A golden-file runner: walk case folders in sorted order, run paka, compare; no update mode allowed.
- note: The runner starts `paka run <file>` with the case folder as working directory. No mode that rewrites expectations.

### M1-09 · L3 · Write the parser for the spec's grammar

- status: done
- needs: M1-08
- flags: none
- done when: examples/light.paka parses to a hand-built tree in a unit test; every syntax reject case in the spec passes in the case runner. Its group is moved with git mv, R100 lines only, and `tests/cases-pending/parse` no longer exists.
- why this level: Syntax tree design that later milestones extend, plus precise error positions; nested generic brackets are easy to get wrong.

### M1-10 · L3 · Resolve names and type-check the subset, with the built-in names table

- status: done
- needs: M1-09
- flags: none
- done when: light.paka resolves and checks in unit tests; every name, type and app reject case in the spec passes in the case runner. Its group is moved with git mv, R100 lines only, and `tests/cases-pending/check` no longer exists.
- why this level: Checking Effect<Msg> against an expected type without inventing general generics; sets the definitions table every later stage uses.

### M1-12 · L2 · Define the core form for the subset and lower into it

- status: done
- needs: M1-10
- flags: none
- done when: A unit test lowers light.paka and compares the result with a core program built by hand.
- why this level: Three expression kinds with tests given; M2-23 redefines the core form, so nothing here is locked.

### M1-13 · L2 · Write the reference interpreter and paka run for the subset

- status: done
- needs: M1-12
- flags: none
- done when: `paka run` passes every runnable case in the spec and both light.paka grant variants, against the spec's hand-written expectations. Its group is moved with git mv, R100 lines only, and `tests/cases-pending/run` no longer exists. A run with standard output on /dev/full, and a run whose standard output is a pipe with its read end already closed, each give the spec's status.
- why this level: For this subset it evaluates one function that returns a list of constants; the rules come from the spec.
- note: The interpreter flushes after every Print and prints the spec's standard-error lines exactly; M1-20 compares standard error.

### M1-14 · L4 · Decide how compiled code and the runtime share data and calls

- status: done
- needs: M1-06
- flags: decision, owner, native-only
- done when: Owner approves the document; its IR is committed as `docs/abi/light.ll`; `clang -c docs/abi/light.ll -o /dev/null` gives empty stderr; the pull request log shows the stand-in runtime's source, the link command and the printed line; the IR contains none of nsw, nuw, inbounds, noalias, fast, nnan, ninf, nsz, arcp, contract, afn, reassoc.
- why this level: Layouts and the trap status return must match on both sides; a mismatch is silent memory corruption.
- note: The document must settle seven points found by a trial build: the IR owns `main`; triple `x86_64-pc-linux-gnu`; a panic in the runtime is caught and returned as a status; the runtime never uses `println!`; output is flushed before exit; how the runtime gives M1-06's result for a failed write and a closed pipe without dying by signal; the runtime is never built with linker-plugin LTO. No heap ownership rules: all M1 data is constant.

### M1-15 · L2 · Install clang 21 on the CI runner

- status: done
- needs: M1-02
- flags: download, native-only, owner
- done when: The CI log shows clang --version reporting 21.x and the workflow still passes.
- why this level: The install step is not given: the package source for clang 21 depends on the image.
- note: Uses the image fixed in M1-02. The owner allowed the install on 2026-10-13. Source: the image's own Ubuntu packages when they carry clang 21, else apt.llvm.org.

### M1-16 · L4 · Create paka-rt with the ABI types and effect-list decoding

- status: done
- needs: M1-01, M1-14
- flags: unsafe, native-only
- done when: Tests decode each ABI value built by hand as raw bytes; Cargo.lock is updated; the crate denies unsafe outside one module; paka-rt's lib.rs denies `clippy::undocumented_unsafe_blocks`, so the M1-01 clippy command fails on an unsafe block without a safety comment.
- why this level: Unsafe reads of memory laid out by compiled code; no Miri here, so mistakes stay silent.
- note: No allocator unless the approved ABI needs heap memory. Edition 2024 needs `#[unsafe(no_mangle)]`, so every exported function lives in the one unsafe module. Per the M1-14 decision: it decodes three shapes made only of 8-byte words (string, effect, list header), refuses the cases the ABI document lists, takes no allocator through the interface, and holds the exported entry and the one safe wrapper that sets the broken-pipe signal to ignored.

### M1-17 · L3 · Add the paka-rt run loop: first message, grants, Print, Exit

- status: done
- needs: M1-16, M1-06, M1-13
- flags: native-only, unsafe
- done when: Unit tests drive the loop with a fake update function and a captured output, matching the expected stdout and status of every case under tests/cases/run; the entry that the ABI document's IR calls is exported from M1-16's module under the document's name.
- why this level: Calls compiled code across the ABI and enforces grants; a missed grant check is silent.
- note: Stays level 3 only while its unsafe is the export attribute on that entry and the one call into compiled code; all decoding of the returned data stays in M1-16's module. The loop follows the runtime rules of the ABI document approved in M1-14. Per the M1-14 decision: the entry `paka_rt1_run` takes five plain arguments; the loop catches panics and returns 4, treats a non-zero status from update as an internal error and prints the spec's lines; the fake update is an ordinary `extern "C"` function.

### M1-18 · L2 · Write the LLVM IR text emitter for the core form

- status: done
- needs: M1-12, M1-14, M1-15, M1-13
- flags: native-only
- done when: `clang -c -x ir - -o /dev/null` accepts every runnable case's IR with empty stderr; a test finds none of the words nsw, nuw, inbounds, noalias, fast, nnan, ninf, nsz, arcp, contract, afn, reassoc; string bytes are hex-escaped. The test fails if it finds no runnable case.
- why this level: For this subset the emitter is a text template fixed by the ABI document's own IR.
- note: Per the M1-14 decision: internal functions are named `p_` plus the Paka name, the result is stored as one whole record with no getelementptr, and no datalayout line is written.

### M1-19 · L3 · Add paka build: run clang on the IR and link paka-rt

- status: done
- needs: M1-17, M1-18
- flags: native-only
- done when: From a clean target, `cargo test --locked --offline` and `cargo test --release --locked --offline` each alone build paka-rt, then build the ABI document's IR and light.paka into executables; both print the line and exit 0; a missing clang gives one diagnostic and status 1.
- why this level: First contact between emitted IR, clang and the runtime; layout mismatches show up here and need cross-module debugging.
- note: The test helper runs `cargo build -p paka-rt --locked --offline` in the same profile before any native test. `paka build` finds libpaka_rt.a next to its own executable and clang on PATH; clang runs at -O2. Command: `paka build <file> -o <output>`. Per the M1-14 decision: uses the ABI document's link command with the system libraries it lists and never passes an LTO flag; a stale runtime library shows up as the linker's undefined-reference error and is reported as one diagnostic under the spec's code for a failed clang run, status 1.

### M1-20 · L2 · Run every runnable case natively and compare with the interpreter

- status: done
- needs: M1-13, M1-19
- flags: native-only
- done when: cargo test runs each runnable case both ways; stdout and status match the hand-written expectation, and stdout, stderr and status match each other; a comparer self-test rejects a planted mismatch; death by signal counts as a mismatch; the /dev/full run and the closed-pipe run each give the same status both ways.
- why this level: The M1-08 runner called twice plus a comparison.
- note: Status 4 never appears in an expectation, so an internal error is always a mismatch.

## M2 — The headless language, complete, in the interpreter

**Exit check.** On `development`, `cargo test --workspace --locked --offline` passes and includes all of: (1) the reference size test under the one counting rule; (2) every sample in the reference checks and runs in the interpreter; (3) every syntax node kind and every built-in appears in a passing conformance case; (4) the hand-written conformance suite passes with no skipped case, and covers example statuses including "unfinished" and effects refused without their grant; (5) every error kind has a rejected program whose `paka check` output carries the offending text, and the types and a fix where the error kind declares them; (6) the paper's wc sample and the counter's update and example run, each change from the paper's text tied to a decision; (7) the M1 light test still passes on both back ends; (8) `tests/cases-pending` does not exist. The ten decisions (M2-01, 02, 03, 05, 06, 09, 10, 12, 14, 15) are merged with the owner's approval.

- Ten owner decisions, five of them in a chain (M2-02, 05, 09, 12, then 14 and 15). The owner's reading time is the limit here, not model cost.
- Tasks that build undecided designs (M2-17, 22, 29, 30, 31, 34, 35, 36, 37) are graded for the likely outcome. Re-grade them once M2-09, M2-10, M2-12 and M2-15 are approved.
- No native work: the M1 emitter is only kept compiling and refuses every other core node. One exception: paka-rt's run loop follows M2-15 in M2-36 and M2-37.
- Packages and dependencies between programs are not decided in any milestone.
- M1 cases that stop holding: the 27 cases listed in section 4 of docs/decisions/M2-02.md, and the cases it lists there under M2-03 and M2-15. The pull request whose code first makes such a case fail deletes it in a commit before the code, cites the decision, and changes nothing else under tests/cases. M2-07 and M2-11 hit this first. M2-38 to M2-42 cover the construct from then on.
- Can start first: M2-01, M2-02, M2-04.
- The owner is needed in: M2-01, M2-02, M2-03, M2-05, M2-06, M2-09, M2-10, M2-12, M2-14, M2-15, M2-41b.

### M2-01 · L5 · Decide which kinds of experiment tasks the headless language must cover

- status: done
- needs: nothing
- flags: decision, owner
- done when: Owner approves a written list of task classes, each naming the types, effects and data shapes it needs, including whether graphs are in.
- why this level: Sets the language's scope and the fairness of the stop-or-go experiment; cannot be redone after measuring.
- note: Decided: the full record is docs/decisions/M2-01.md.

### M2-02 · L5 · Decide bindings, control flow, return, argument labels, discarded results and names

- status: done
- needs: M1-06
- flags: decision, owner
- done when: Owner approves a proposal with alternatives and a grammar for every statement and definition form, with accepted and rejected samples. It also fixes comment syntax and which definition a comment belongs to, and lists each M1-06 token, grammar, name and type rule as kept or changed; the start, end and missing-grant rules are left to M2-15. It fixes the name rules: shadowing, unqualified variant names, two enums sharing a variant name, and which files make one program. For each changed rule it lists the M1 cases under tests/cases that stop holding.
- why this level: Shapes every program a model writes; one spelling per construct cannot change after the experiment.
- note: Decided: the full record, with the grammar, the exact newline rule and the new diagnostic codes, is docs/decisions/M2-02.md. Nesting is limited to 64 open brackets; the 65th opener is the error `nesting-too-deep`.

### M2-03 · L5 · Decide the primitive types and the exact meaning of every operation

- status: done
- needs: M2-01
- flags: decision, owner
- done when: Owner approves a table: Int width, wrapping spelling, Float and the canonical NaN bits, float-to-text, String, conversions, evaluation order, and whether sin, exp and pow exist and which maths library gives them; every row gives a result or a named trap.
- why this level: This table is the 'one meaning' promise; every back end is tested against it.
- note: Decided: the full record, with the operation table, the exact algorithms, the escapes and the two new diagnostic codes, is docs/decisions/M2-03.md.

### M2-04 · L2 · Give every error one shape, printed as text or JSON

- status: done
- needs: nothing
- flags: none
- done when: paka check on a file with a seeded error prints the hand-written expected text and JSON, with offending text and fix fields. Each error kind declares whether it carries expected and found types and whether it carries a suggested fix.
- why this level: The fields are given by paper section 6; one module, routine formatting.
- note: Fields: stable kind id, definition name, message, file and offsets, offending text, expected type, found type, optional fix (replacement text for a span). The first line of the text form is M1's `path:line:col: error[code]: message`. The pull request gives one worked sample as text and the same error as JSON with exact key names; positions are byte offsets plus 1-based line and column. Merged: `paka check <file> [--json]` prints the one-line form plus one line per field, or one JSON object; `paka run` and `paka build` still print the one line. M2-18 writes the command and the field names into the reference.

### M2-05 · L5 · Decide ownership: copies, inout exclusivity, borrows, recursive types, changing items in place

- status: done
- needs: M2-02
- flags: decision, owner
- done when: Owner approves a proposal with alternatives, the exact overlap rule, and accept and reject samples for every rule.
- why this level: Heart of pillar 2; the rule must be checkable at one call and is hard to loosen later.
- note: Decided: the full record, with the overlap test, the call rule, the grammar piece and the diagnostic codes, is docs/decisions/M2-05.md.

### M2-06 · L5 · Decide traps, holes and what an example means

- status: done
- needs: M2-02, M2-03
- flags: decision, owner
- done when: Owner approves trap kinds and reports, interpreter limits, hole typing, and example statuses including unfinished, with samples. It also fixes which parts of a trap report are part of the program's meaning (kind, source position) and which trap wins when two are possible.
- why this level: Defines failure and evidence for the whole language; every later back end must copy it.
- note: Decided: the full record, with the trap report, the limits, the hole rules and the `paka examples` output byte for byte, is docs/decisions/M2-06.md.

### M2-07 · L2 · Extend the lexer to every token in the approved grammar

- status: done
- needs: M2-02, M2-03, M2-04
- flags: none
- done when: Lexer tests with hand-written token lists pass for every token kind and every lexical error.
- why this level: Token list is given by the decisions; one module, well-known technique.
- note: Follows docs/decisions/M2-03.md; the M1 lex cases it lists as no longer holding are deleted in a commit before the code that breaks them. Merged: the lexer gives every M2 token and lexical error; the M1 parser skips what it does not use until M2-11. `literal-out-of-range` is left to the checker.

### M2-08 · L3 · Implement Int, Float, Bool and String operations from the approved table

- status: done
- needs: M2-03
- flags: none
- done when: Every row of the approved table has a hand-written vector (operation, inputs, expected result or named trap) in a data file read by a runner that does not depend on the back end; all pass in the interpreter; a test fails if a row has none. Paka Int and Float are their own Rust types exposing only the table's operations; a lint or test fails on raw arithmetic or f64 maths methods in the compiler crate; float rows are compared bit for bit.
- why this level: Many exact edge cases: overflow, NaN, float printing, string units; errors are easy to miss.
- note: Follows docs/decisions/M2-03.md; the M1 lex cases it lists as no longer holding are deleted in a commit before the code that breaks them. Merged: the operations live in the runtime crate (`paka-rt/src/ops`), which the compiler also links, so every back end runs the same routines. The vectors are tests/vectors/ops.vec; a back end runs them through one function from operation name and inputs to result or trap. M2-16 covers the checker-only rows.

### M2-08b · L3 · Add the approved maths functions without f64 methods

- status: done
- needs: M2-03, M2-08
- flags: conditional
- done when: Every maths row of the table has a hand-written bit-exact vector passing in the interpreter.
- why this level: Bit-exact results without the platform's maths library; rounding mistakes are easy to miss.
- note: Only if M2-03 approves functions such as sin, exp or pow. Gains the download and owner flags if a crate is vendored for it. Closed without work: M2-03 approves no maths functions beyond the table rows M2-08 implements.

### M2-09 · L5 · Decide generics, interfaces and function values together

- status: done
- needs: M2-02, M2-05
- flags: decision, owner
- done when: Owner approves a proposal with alternatives and samples showing every generic definition is checked from signatures alone. It also fixes how a generic definition is compiled in dev mode (shared code or one copy per type) and what a user's interface hash covers. It also fixes equality, ordering and to-text for user types, whether function values can be compared, and how max, ==, +, += and length work on several types without overloading.
- why this level: Open research point; pillars 1 and 4 hold only if this design works.
- note: Decided: the full record, with the built-in table, the matching rule, the interface hash and the dev-mode compilation rule, is docs/decisions/M2-09.md.

### M2-10 · L5 · Decide shared immutable data and pools with handles

- status: done
- needs: M2-01, M2-05, M2-09
- flags: decision, owner
- done when: Owner approves the types, their operations, the stale-handle rule and iteration order, with samples for a tree and a graph.
- why this level: Fixes the memory model for graph data; programs will depend on it everywhere.
- note: M2-01 put graphs in by list index and pools are not built: the pool part is dropped, shared data is covered with a tree sample only, and pools are recorded as not decided. Decided: nothing is built; the record with the tree and graph samples is docs/decisions/M2-10.md.

### M2-11 · L3 · Parse all definitions, statements, expressions, patterns and holes

- status: done
- needs: M2-02, M2-03, M2-04, M2-05, M2-06, M2-07
- flags: none
- done when: Parser tests with hand-written trees pass; one broken definition gives an error with its offending text and leaves the others parsed.
- why this level: Recursive descent with recovery per definition and helpful messages; details are easy to get wrong.
- note: Follows docs/decisions/M2-02.md: the newline, comment and nesting rules of its sections 1.5 to 1.7, the grammar of section 2 and the codes of section 5. Follows docs/decisions/M2-03.md; the M1 lex cases it lists as no longer holding are deleted in a commit before the code that breaks them. Follows docs/decisions/M2-06.md for traps, holes and examples. Done. Nesting is counted with and without brackets and every chain stops at 1,000 links. The check code `unsupported-form` marks forms the parser accepts and the checker does not handle yet; each checker task removes its forms from it, and the code goes when none is left.

### M2-12 · L5 · Decide the one error-handling mechanism

- status: done
- needs: M2-09, M2-06
- flags: decision, owner
- done when: Owner approves one mechanism with alternatives, the line between trap and error, and samples for failure and absence.
- why this level: Language-shaping and hard to undo; it touches every signature that can fail.
- note: Decided: the full record, with the typing rules, the trap-or-return table and the samples, is docs/decisions/M2-12.md.

### M2-13 · L3 · Resolve names across a program

- status: done
- needs: M2-02, M2-11
- flags: none
- done when: Tests pass for every name kind, duplicates, and unknown names with a suggested fix.
- why this level: Name resolver over several definition kinds.
- note: Done: the pass is paka/src/resolve.rs and no command calls it yet; M2-16 wires it in. It gives declaration errors first, then one result per definition, and app entry errors per entry, so the checker keeps the order of spec/m1.md section 8. The nearest-name rule is in docs/decisions/M2-02.md.

### M2-14 · L5 · Decide the built-in types and functions a headless program may call

- status: done
- needs: M2-01, M2-03, M2-09, M2-12, M2-10
- flags: decision, owner
- done when: Owner approves a full list with signatures and exact results that covers every approved experiment task class. Pool and shared-data types and operations are taken from M2-10 unchanged.
- why this level: Sets the vocabulary models must learn; names and ordering rules cannot change after the experiment.
- note: Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Decided: the full table of 9 types, 4 constructors and 63 functions, with the exact sort procedure, is docs/decisions/M2-14.md.

### M2-15 · L5 · Decide headless apps: first message, effects, grants, ending, effect failure, standard input and handler traps

- status: done
- needs: M2-01, M2-06, M2-09, M2-12, M1-06
- flags: decision, owner
- done when: Owner approves the app form, the effect list, refusal and failure behaviour, standard input, rollback visibility, sequencing and concurrency rulings, and wc rewritten under the rules; each provisional M1-06 start, end and missing-grant rule is named as kept or changed, with the new expected output and status of each affected M1 case stated in the decision.
- why this level: Fixes the program model and permission boundary; settles or defers sequential notation and concurrency.
- note: Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Decided: the full record, with the run loop, the effect and reason tables, wc rewritten and the nine re-expected M1 cases, is docs/decisions/M2-15.md.

### M2-16 · L3 · Check signatures and type definitions into one signature table

- status: done
- needs: M2-03, M2-05, M2-13, M2-09
- flags: none
- done when: Tests pass: each definition kind's signature is accepted or rejected with expected and found types; built-ins come from one table.
- why this level: Base for per-definition checking; recursive types and default values hide easy-to-miss cases.
- note: Follows docs/decisions/M2-05.md, including its two tightened call rules. Follows docs/decisions/M2-09.md: the interface hash covers every named type a signature reaches, and no app state may hold a function type. Follows docs/decisions/M2-14.md for the built-in table. Calls the M2-13 pass and gives the built-in table of paka/src/resolve.rs its signatures. Done: the table is paka/src/sig.rs with the built-in rows in paka/src/builtins.rs; no command calls it yet; M2-25 wires it in with the body check. It runs the finite-value test, so M2-21 does not. It holds no interface hash (M4-11) and no app rule (M2-32). The parser reads a field default as a constant.

### M2-17 · L4 · Parse type arguments, function values and error-handling forms

- status: done
- needs: M2-09, M2-11, M2-12
- flags: none
- done when: Parser tests with hand-written trees pass for every new form in the two decisions, including their error cases.
- why this level: Telling generic brackets from comparison operators is a known parser trap; a mistake mis-parses silently.
- note: Follows docs/decisions/M2-02.md: the newline, comment and nesting rules of its sections 1.5 to 1.7, the grammar of section 2 and the codes of section 5. Follows docs/decisions/M2-09.md: the interface hash covers every named type a signature reaches, and no app state may hold a function type. Follows docs/decisions/M2-06.md for traps, holes and examples. Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Done. Keeping comments in the tree (section 1.6 of M2-02) is not part of this task; it belongs to M4-15, and until then the lexer skips comments. M2-09 section 2 (amended by M2-30): the spellings (a, b) =>, || 3, fn(...) {...} in an expression and a nested fn get the section 5 lambda text; today they give the generic parser texts.

### M2-18 · L4 · Write the language reference from the approved decisions, within 8,000 tokens

- status: done
- needs: M2-02, M2-03, M2-05, M2-06, M2-09, M2-10, M2-12, M2-14, M2-15
- flags: none
- done when: Reference merged; the size test passes under the one counting rule (bytes divided by 3, at most 8,000, until M3 records real tokens); a checklist maps every decision point to a line; samples are marked for later testing; the reference replaces the M1-06 spec, and each M1 case affected by M2-15 gets a re-expected copy under tests/cases-pending with the decision cited; tests/cases is not touched.
- why this level: Must be exact, complete and tiny; one wrong sentence silently misleads every model reading it.
- note: The same counting rule is reused by M4-18, M5-41 and M7-13. Follows docs/decisions/M2-15.md for the app form, effects, grants and ending. Follows docs/decisions/M2-14.md for the built-in table. Done: spec/paka.md with its checklist and size test. The copies under tests/cases-pending wait for the run loop task, which moves them into tests/cases; two `paka check` status changes and one run case cannot be held there and are described in its NOTE.md.

### M2-19 · L3 · Check statements and expressions against signatures only

- status: done
- needs: M2-16
- flags: none
- done when: Accept and reject tests pass with expected and found types; an error in one body never changes another definition's result.
- why this level: Local inference, labels, mutability and discards across several modules; own tests needed.
- note: Done: the check is paka/src/body.rs and paka/src/body/; no command calls it yet, and M2-25 wires it in. It gives one result per definition, the earliest error in source order among name, signature and body errors, and a type per expression. Operators and `?` carry their position in the tree. Left to later tasks: match arms, patterns, holes, example and app bodies (M2-20, entry points in paka/src/body/hook.rs), ownership (M2-21), `cannot-infer` at an argument (M2-22), full function values (M2-30) and error forms (M2-31).

### M2-20 · L3 · Check match, patterns, holes and example bodies

- status: done
- needs: M2-06, M2-19
- flags: none
- done when: Tests pass: missing variants are listed with a fix, a hole takes its expected type, example bodies follow the decision.
- why this level: Exhaustiveness and expected-type holes are known techniques with easy-to-miss cases.
- note: Follows docs/decisions/M2-02.md: the newline, comment and nesting rules of its sections 1.5 to 1.7, the grammar of section 2 and the codes of section 5. Follows docs/decisions/M2-06.md for traps, holes and examples. Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Names the code for a `_` arm on an enum subject, which M2-02 leaves unnamed. Done in paka/src/body/ (arms.rs, pattern.rs, hole.rs, branch.rs). The `_` arm on an enum is `wildcard-on-enum`. Branches of `if`, `match` and list or map items take their type from the first branch that carries one, in any order. Example bodies are checked; apps are left to M2-32. Pattern binding types and resolved variants are not in the type map; M2-25 adds them if lowering needs them.

### M2-21 · L4 · Enforce copies, inout exclusivity and borrow rules in the checker

- status: done
- needs: M2-05, M2-20
- flags: none
- done when: Every accept and reject sample from the ownership decision passes, plus own edge tests; rejections carry a fix where the error kind declares one (M2-04).
- why this level: Overlap analysis on paths; a missed case is a silent hole release mode will trust.
- note: Follows docs/decisions/M2-05.md, including its two tightened call rules. Done: the checks are paka/src/body/own.rs, run after the type checks; the earliest error in a definition wins and a type error wins a tie. Places part ways only at fields before the first index step. A String key inside an & place counts as a read-only operand.

### M2-22 · L4 · Check each built-in use from its table row and argument types

- status: done
- needs: M2-09, M2-17, M2-20
- flags: none
- done when: Decision samples pass; the type variables of a built-in row are taken from the arguments and the expected type, and `cannot-infer` is given when neither fixes one. A test shows a function body's result never depends on any caller.
- why this level: The language has no user generics (M2-09), so only built-in rows carry type variables; binding them wrongly accepts or refuses programs silently.
- note: Follows docs/decisions/M2-09.md: the interface hash covers every named type a signature reaches, and no app state may hold a function type. Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Settle here whether the `cannot-infer` message names the local or the expression when both exist: docs/decisions/M2-02.md and M2-12.md give both forms. Owns a hole as a later argument of a built-in or inside `Some(..)` with a known result type. Done: built-in row letters bind from arguments and the expected type; open values are checked against their position with fits. Ok in one branch and Err in another still do not complete each other (cannot-infer).

### M2-23 · L4 · Define the core form for the whole headless language

- status: done
- needs: M2-05, M2-06, M2-09, M2-12, M2-20, M2-15
- flags: none
- done when: Core types and a written rule per node are merged, including the app, message and effect-list forms; examples/light.paka lowers to it and still passes on both back ends.
- why this level: Contract for every future back end; evaluation order and trap points must be explicit.
- note: M1's native comparison keeps running only the M1 run group; conformance cases are interpreter-only until M4-40. Follows docs/decisions/M2-06.md for traps, holes and examples. Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Done: docs/core.md is the contract for the core form. lower.rs still bridges to the M1 back end through m1(); M2-26 removes the bridge. Callee::Builtin indexes builtins::ROWS; locals are numbered with parameters first.

### M2-25 · L3 · Lower checked programs to the core form

- status: done
- needs: M2-23
- flags: none
- done when: Tests compare hand-written core for each statement, expression and pattern kind; all pass.
- why this level: Desugars match, interpolation and inout places while keeping the fixed evaluation order.
- note: Wires the name pass, the signature table and the body check into `paka check` and `paka run` in place of the milestone 1 checker. Done: paka check and paka run use the name pass, signature table and body check; check.rs is gone. Lowering is paka/src/lower.rs with lower/; the body check records the chosen built-in row in Bodies.rows. run and build still go through the m1() bridge and answer unsupported-form outside the milestone 1 subset.

### M2-26 · L3 · Evaluate the core form in the interpreter

- status: done
- needs: M2-08, M2-23
- flags: none
- done when: Hand-built core programs give hand-written results for every node, each trap kind and the call-depth limit.
- why this level: Tree-walking interpreter with inout places and traps as status returns; fiddly details.
- note: Removes the m1() bridge in paka/src/lower.rs and the unsupported-form tool error (spec/m1.md marks it temporary). Done: paka run evaluates the core form in paka/src/interp.rs with interp/; the m1() bridge is gone. paka build still emits only the milestone 1 subset (llvm::view) and answers unsupported-form for anything else, holes included. Expression nesting never traps; deep evaluation continues on a fresh stack segment. Built-ins other than assert, unreachable, abs, to_int and from_code end the run with an internal error until M2-33 and M2-34.

### M2-27 · L2 · Run examples and compute each one's status

- status: done
- needs: M2-06, M2-25, M2-26
- flags: none
- done when: paka examples prints hand-written expected statuses: pass, fail, trapped, and unfinished when a hole is reached.
- why this level: Clear rules from the decision; one module mapping results to statuses.
- note: Follows docs/decisions/M2-06.md for traps, holes and examples. Done: paka examples FILE in paka/src/interp.rs (examples, entry) with interp/show.rs for the value text form; tests in paka/tests/examples.rs. A failed example is reported at a claim site recorded in Fn.claims (docs/core.md section 9 item 18). An example that returns false early is reported at its first claim with no left and right lines.

### M2-28 · L2 · Extend the golden-file runner to check and example tests

- status: done
- needs: M2-04, M2-27
- flags: none
- done when: The runner passes one hand-written case of each kind and has no mode that rewrites expected files.
- why this level: Known pattern; output formats are fixed by earlier tasks.
- note: Done: the stage folder names the command: check-text, check-json and examples hold accept cases (spec/m1.md section 11); every other stage runs paka run. paka/tests/native.rs builds only the lex, parse, check and run stages.

### M2-29 · L3 · Run generic functions and interface calls in the interpreter

- status: done
- needs: M2-22, M2-25, M2-26, M2-28
- flags: none
- done when: Golden tests built from the generics decision samples print the hand-written results.
- why this level: Type arguments and interface lookup must reach the core without changing meaning.
- note: Done: M2-09 has no user generics or interfaces, so this runs the generic List, Map and Option rows and sort (paka/src/interp/generic.rs) and pins structural equality. Interpreter-only cases go in tests/cases/run-interp (spec/m1.md section 11). The not-a-function text is M2-09s for every local (M2-02 amended). String forms of the shared rows wait for M2-33; sort_by for M2-30 and M2-34.

### M2-30 · L3 · Check, lower and run function values

- status: done
- needs: M2-09, M2-29
- flags: none
- done when: Golden tests from the decision samples pass, including variant constructors as functions and every rejected capture.
- why this level: Spans checker, core and interpreter; capture rules need care.
- note: Done: function values check, lower and run on the interpreter, with sort_by (stable merge sort, before(right, left)). Mismatches name function types unquoted in the canonical spelling; an unbound M shows as Msg. A state type that holds a function type is type-mismatch, checked before bad-state. M2-09 sections 1.8, 1.9 and 2 amended.

### M2-31 · L3 · Check, lower and run the error-handling forms

- status: done
- needs: M2-12, M2-29
- flags: none
- done when: Golden tests from the decision samples pass; an unhandled failure is rejected with a suggested fix.
- why this level: Spans checker, lowering and interpreter; known desugaring with easy-to-miss paths.
- note: Follows docs/decisions/M2-12.md for Option, Result, `?` and unreachable(). Done: the ? forms, the habit spellings and unreachable() check, lower and run on the interpreter, with full-text cases for every M2-12 section 1.9 and section 5 text. An argument that leaves control keeps its parameter type. Ok and Err in different branches do not join (M2-02 section 5 amended). Runs that need parse_int or ReadFile are only checked here; M2-33 runs them.

### M2-32 · L2 · Parse and check app definitions

- status: done
- needs: M2-15, M2-20
- flags: none
- done when: Tests accept the approved app forms and reject a wrong state, update signature, first message or unknown grant.
- why this level: Clear rules from the app decision; one module.
- note: Follows docs/decisions/M2-06.md for traps, holes and examples. Done: paka/src/body/app.rs, not called by any command yet. New code `bad-state`. The update-signature text follows M2-15, not spec/m1.md section 8; M2-36 brings the spec in line.

### M2-33 · L2 · Implement the built-in String functions and every other built-in that is not a collection or pool function

- status: done
- needs: M2-14, M2-31, M2-34
- flags: none
- done when: Every function in the approved list outside M2-34 has hand-written golden tests (String functions for normal, empty and non-ASCII input); all pass; a test fails if an entry of the built-in table has no golden test.
- why this level: Each function is small with a clear spec; routine code.
- note: Follows docs/decisions/M2-14.md for the built-in table. M2-31 checks the parse_int samples and the M2-12 section 1.7 app but cannot run them; add run cases for them once parse_int and ReadFile run. When min runs, make tests/cases/run-interp/sample_grid_path_sum use min as in M2-14 section 4 (it uses a local lesser today); add M2-14 section 4 samples 1 and 4 word for word; make the read_own_source effect case count characters with len. Done: paka/src/interp/text.rs and number.rs run every String and number row of M2-14 in the interpreter; golden cases in tests/cases/run-interp; every_function_has_a_golden_case in paka/src/builtins.rs reads ROWS and fails when no run case calls a name. The text routines are not yet in paka-rt (M2-14 section 3): native code still refuses them.

### M2-34 · L3 · Implement the built-in collection types and their functions

- status: done
- needs: M2-14, M2-21, M2-30, M2-31
- flags: none
- done when: Every collection function in the approved list has hand-written golden tests, including bounds traps and iteration order; all pass.
- why this level: Generic signatures, function arguments and in-place change meet here; several modules.
- note: Follows docs/decisions/M2-14.md for the built-in table. Done: the List, Map and Option rows and sort/sort_by were in place; added the Map key message, the for fix texts, the += count idiom on a missing key (Int += only, M2-14 sections 1.11 and 10, M2-06 section 1.2), cap checks taking the cap as a parameter, and golden cases in tests/cases/run-interp and check-text.

### M2-36 · L3 · Run a headless app: first message, message loop, exit

- status: done
- needs: M2-15, M2-28, M2-32, M2-18
- flags: none
- done when: paka run tests pass for start, ending and exit codes. Where M2-15 changed the provisional start or end rule, the interpreter and paka-rt's run loop change in this pull request, and M2-18's re-expected copy is moved from tests/cases-pending over the old case with git mv and passes on both back ends.
- why this level: The care is in message and effect ordering.
- note: State rollback after a trapping handler is tested here only if M2-15 lets a headless app survive a trap; otherwise it arrives with M5-25b. The paka-rt change adds no unsafe beyond the one call into compiled code. Follows docs/decisions/M2-15.md for the app form, effects, grants and ending. Print moves to the raw standard-output descriptor in the interpreter and paka-rt, and docs/abi/m1.md sections 5 and 6 are corrected to match. Wires the app check of paka/src/body/app.rs, adds `bad-state` to the code table of spec/m1.md section 11 and updates the update-signature text there to M2-15. M2-25 already moved seven of the nine pending cases; check/unknown_grant and parse/variant_payload remain in tests/cases-pending. Done: paka run FILE [argument...] runs the M2-15 loop on one interpreter thread; Print writes descriptor 1 directly in both back ends; an ExitWith literal outside 0 or 5 to 255 is literal-out-of-range. Cases may carry an args file (spec/m1.md section 11). variant_payload passes on the interpreter only: native refuses start fields with unsupported-form until M4-05. paka run cannot see a descriptor 1 closed at start (M2-15 amended); a built program reports it.

### M2-37 · L3 · Perform console, file, argument and exit effects only when granted

- status: done
- needs: M2-30, M2-31, M2-36
- flags: none
- done when: Golden tests over an in-memory world, plus one on real files, pass for each effect, failure, and effect refused without its grant. Where M2-15 changed the provisional missing-grant rule, the interpreter and paka-rt's run loop change in this pull request, and M2-18's re-expected copy is moved from tests/cases-pending over the old case with git mv and passes on both back ends.
- why this level: Several effects, fixed failure values, and a grant check with no way around it.
- note: The paka-rt change adds no unsafe beyond the one call into compiled code. Follows docs/decisions/M2-15.md for the app form, effects, grants and ending. Done: paka/src/interp/effects.rs performs Print, ReadFile, WriteFile, Exit and ExitWith behind a small World trait; the host reads a file by its metadata first, so a non-regular file is refused unopened and one over the String limit unread. Cases in tests/cases/run-interp, a real-file test in paka/tests/effects.rs. paka-rt is unchanged: native file effects stay unsupported-form until its ABI carries them.

### M2-38 · L3 · Write conformance tests for values, expressions, control flow and functions

- status: done
- needs: M2-18, M2-28
- flags: none
- done when: Expected results merged before any run, written without access to a built paka; every decision point in scope has a case.
- why this level: Hand-computing exact results from the reference.
- note: The caller runs this agent with no paka binary and no way to build one (no cargo, or a checkout holding only the reference, the decision records and the tests folder); the agent must not build or run paka; the cases land in a not-yet-required list. M2-41b runs them. Follows docs/decisions/M2-15.md for the app form, effects, grants and ending. Done: 157 cases in tests/conformance/core, written from the records alone, with decision tables in tests/conformance/README.md. The open points they raised are settled in M2-02, M2-03, M2-06 and spec/m1.md. Reject cases follow spec/m1.md section 11 (paka run, one standard-error line).

### M2-39 · L3 · Write conformance tests for types, match, generics, function values and errors

- status: done
- needs: M2-18, M2-30, M2-31
- flags: none
- done when: Expected results merged before any run, written without access to a built paka; every decision point in scope has a case.
- why this level: Hand-computing exact results across interacting features.
- note: The caller runs this agent with no paka binary and no way to build one (no cargo, or a checkout holding only the reference, the decision records and the tests folder); the agent must not build or run paka; the cases land in a not-yet-required list. M2-41b runs them. Done: 208 cases in tests/conformance/types (47 run, 161 reject) for user types, match, generics, function values, habit spellings, sort_by and errors. The README Case reading table lists the readings chosen where a record leaves a point open.

### M2-39b · L3 · Write conformance tests for the built-in functions and collection types

- status: done
- needs: M2-18, M2-28
- flags: none
- done when: Expected results merged before any run, written without access to a built paka; every decision point in scope has a case.
- why this level: Hand-computing exact results for every built-in, including iteration order and bounds traps.
- note: The caller runs this agent with no paka binary and no way to build one (no cargo, or a checkout holding only the reference, the decision records and the tests folder); the agent must not build or run paka; the cases land in a not-yet-required list. M2-41b runs them. Done: 163 cases in tests/conformance/builtins, one or more for each of the 63 rows, written from the records alone. A run case for a stack or memory trap holds a trap file (kind only, any position); memory cases use only sizes no cap allows. M2-14 settled: the += report on a new key, the huge Float text, len of a bare empty literal.

### M2-40 · L4 · Write conformance tests for ownership, shared data and pools

- status: done
- needs: M2-18, M2-21
- flags: none
- done when: Expected results merged before any run, written without access to a built paka; every decision point in scope has a case.
- why this level: The accept and reject boundary is subtle; a wrong expectation silently blesses a wrong rule.
- note: The caller runs this agent with no paka binary and no way to build one (no cargo, or a checkout holding only the reference, the decision records and the tests folder); the agent must not build or run paka; the cases land in a not-yet-required list. M2-41b runs them. M2-01 put graphs in by list index and pools are not built: the pool part is dropped, shared data is covered with a tree sample only, and pools are recorded as not decided. Follows docs/decisions/M2-05.md, including its two tightened call rules. Follows docs/decisions/M2-10.md: runs its tree and graph samples and one case showing a value used twice in a tree is two independent copies. M2-35 is dropped. Follows docs/decisions/M2-14.md for the built-in table. Done: 94 cases in tests/conformance/ownership, written from the reference and records only; tests/conformance/README.md gives the pass rules, the decision-point table and the settled readings. No runner reads them yet; M2-41b runs them. A read of a function type now counts as fresh (M2-05 section 1.2), matching M2-09.

### M2-41 · L3 · Write conformance tests for apps, effects, grants, traps, examples and holes

- status: done
- needs: M2-18, M2-37
- flags: none
- done when: Expected results merged before any run, written without access to a built paka; every decision point in scope has a case.
- why this level: Many runtime paths: refusal, failure, unfinished; results worked out by hand.
- note: The caller runs this agent with no paka binary and no way to build one (no cargo, or a checkout holding only the reference, the decision records and the tests folder); the agent must not build or run paka; the cases land in a not-yet-required list. M2-41b runs them. paka build refuses a program with a hole as unsupported-form; paka run reports the hole. The cases decide which form the build side takes. M2-38 left call depth to this task (kind and status only); order_call_arguments and order_assignment_right_first in tests/conformance/core are the M2-06 section 1.3 first-trap samples and are reused here. Stack and memory cases use the trap file of tests/conformance/README.md (kind and status, any position). Done: 231 cases in tests/conformance/apps; the README defines the extra case files (command, files/, after/, absent, present, raw args, a reject line with no position). Settled with them: a function-typed state field is type-mismatch before the default check (M2-09 section 1.11); the fixed counter sample also fixes first_of_none; the 256-byte cut, hole note order, a hole in an example, a stack or memory example entry, a trap straight in an example (M2-06); start: naming a type (M2-15); spec/m1.md lists the file grants and the state entry.

### M2-41b · L4 · Run the conformance suite and rule on every mismatch

- status: done
- needs: M2-38, M2-39, M2-40, M2-41, M2-33, M2-34, M2-39b, M2-42, M2-08b
- flags: open-ended, owner
- done when: All cases pass with none skipped under `cargo test --workspace --locked --offline`; the not-yet-required list is empty; each change to code or expectation cites the decision line that settles it; a mismatch no line settles goes to the owner as an amendment to that decision before either side changes. A test proves every error kind has a case, and a test applies each expected fix and the result passes paka check. A kind found without a case gets one from an agent without the paka binary, as in M2-42.
- why this level: Choosing which side is wrong silently sets the language's meaning.
- note: Settle while running tests/conformance/errors: the span length where no record states it (bad-escape, changed-while-walked, inout-overlap, bad-assignment-target, marked * in the README table); spec/m1.md section 10 still puts type-mismatch at the first token and gives no-app the stage check, while later records put a mixed operand at the operator and only run and build raise no-app; spec/m1.md section 10 lists 31 of the 54 kinds; the lex cases M2-03 cites (string_close_brace, escape_before_line_end) do not exist. Rule on the types/ Case reading table: a wrong field count in a pattern is wrong-argument-count at the variant name; max(1, 2.0) is at the second argument; a record literal in an if header fails at the colon; a field default fails at the first token not part of a constant. Two types/ gaps the README calls unpinned are now settled by later records: missing-value at the arm first token (M2-02 section 5) and a state with a function field giving type-mismatch before bad-state (M2-09); add their cases. M2-34 amended the missing-key detail for Int += (M2-14 sections 1.11 and 10): check tests/conformance/builtins plus_equals_on_new_key against it. The apps/ README notes that state_function_field was written under the opposite reading of the state check order; the case itself holds under the settled one. Not covered by apps/ and left to the host tests: no permission, no space, a file over the String limit, Print on a closed or full stdout.

### M2-42 · L3 · Write a rejected program for every error kind

- status: done
- needs: M2-18, M2-21, M2-30, M2-31, M2-32
- flags: none
- done when: Every error kind has a rejected program with its kind id, position, offending text, and the types and fix where the kind declares them, all written by hand and merged before any run, by an agent without the paka binary; message wording is not compared.
- why this level: Needs a minimal trigger for each error across all checks, with exact expected output.
- note: The caller runs this agent with no paka binary and no way to build one (no cargo); the agent may read the compiler source for the list of error kinds and what each declares, and must not build or run paka; the cases land in a not-yet-required list. M2-41b runs them. Done: 53 cases in tests/conformance/errors cover 50 of the 54 kinds; file-unreadable, clang-missing, clang-failed and runtime-missing need a broken environment and stay with the spec/m1.md section 13 tests. Format: expect.json holds a subset of the paka check --json object; via run or build for no-app and unsupported-form.

### M2-43 · L2 · Add the paper's wc and counter samples as tests

- status: done
- needs: M2-33, M2-37
- flags: none
- done when: Both run with hand-written expected output; every difference from the paper's text is listed beside the decision that required it.
- why this level: Programs are given; only count_words and the change list are new.
- note: The counter is tested as its type, enum, update and example only, through paka examples. The view and app definitions are left out as a scope cut (M5) and listed as such, not tied to a decision. Done: tests/cases/run-interp/sample_wc_* (count, empty file, missing file, no argument) and tests/cases/examples/sample_counter; the difference lists are the README.md files in sample_wc_count and sample_counter.

### M2-44 · L3 · Add the exit gates: reference samples, size, construct and built-in coverage

- status: done
- needs: M2-33, M2-34, M2-38, M2-39, M2-40, M2-41, M2-42, M2-43, M2-41b, M2-39b
- flags: none
- done when: One command runs every reference sample, checks the size budget, and lists each syntax node kind and built-in missing from passing conformance cases. Node kinds and built-ins are read from the compiler's own definitions, not a separate list; a self-test shows a planted unused node kind, a planted unused built-in and a planted failing reference sample each make the command fail.
- why this level: Extracting samples and enumerating node kinds across stages; known technique, several modules.
- note: Done: the gate is the test exit_gate in paka/tests/conformance/gate.rs, run with `cargo test -p paka --test conformance --locked --offline gate` (gate plus self-tests); its readings are in the Exit gate section of tests/conformance/README.md. At merge it reports nothing missing.

### M2-45 · L4 · Close every gap the gate reports, in the reference and the conformance cases

- status: done
- needs: M2-44
- flags: open-ended
- done when: The gate command passes with nothing missing; every reference or case change cites its decision; size stays within budget. A new case is written by an agent without the paka binary, as in M2-38. The gate runs as a required test under `cargo test --workspace --locked --offline`.
- why this level: Edits must stay exact and in budget; one wrong sentence silently misleads every model.
- note: The unused-name warning (M2-02 section 2 and section 5) is decided but no task implements it; close it here if no earlier task does. Left open after M2-41b: no conformance case for hole notes under --json; no host test for a closed or full standard output or a file over the String limit; no case for an open joined list type such as [None, None]; text routines are not native in paka-rt yet. The gate already runs under cargo test --workspace and passes with nothing missing at M2-44; what remains is the list above. Done: the unused-name warning (M2-02 sections 1.3 and 5), host tests for a full or closed output and a file over the limit (M2-15 section 1.7), open list and map literals reported at their [ (M2-02 section 5), and the --json stream rule (M2-06 section 1.5). Native text routines in paka-rt are left to the compiled back ends. Untested: a FIFO swapped in between the metadata look and the open.

## M3 — The first measurement

**Exit check.** All five hold on `development`: (1) a protocol naming the tasks, the model, the three metrics (first-try pass rate, edits to a passing program, tokens to a passing program) and the losing margin was merged and tagged `protocol` by the owner at a commit older than the first model call in the raw logs; (2) each task has one set of hand-written tests that runs unchanged against Paka, Rust and TypeScript; (3) the raw logs hold a finished trial for every counted task, language and repetition, all from the commit the owner tagged `run` after the dry-run and calibration amendments, and the report lists every difference between the two tags; (4) one command rebuilds the report's three metrics from those raw logs; (5) the result is published in the README and the genesis paper.

- The owner continues to M4. The tasks and harness stay as a benchmark for the token cost.
- Live model calls happen only in M3-19, M3-19b and M3-20. They go through the Claude Code command line (`claude -p`) with no API key and no paid service. M3-19 measures what one run uses before M3-20 is planned.
- One edit = replace the whole file, in all three languages. The experiment does not measure Paka's per-definition tools; M3-01 must state that limit.
- Tasks flagged `repeat` are run once per batch; with 20 experiment tasks this milestone has 6 more tasks than listed.
- Tried on 2026-10-13 with no key set: `claude -p PROMPT --model ID --system-prompt TEXT --tools "" --setting-sources "" --strict-mcp-config --no-session-persistence --disable-slash-commands --output-format json < /dev/null` answers and reports input, cache and output tokens. About 800 tokens of harness text ride along on every call, the same for all three languages. `--bare` must not be used: it accepts only an API key.
- Can start first: M3-01.
- The owner is needed in: M3-01, M3-02, M3-03, M3-04, M3-05, M3-18, M3-19, M3-19b, M3-20, M3-21, M3-23.
- Downloads: M3-05.

### M3-01 · L5 · Decide what one run is: prompt, tools, feedback and counting rules

- status: done
- needs: nothing
- flags: decision, owner
- done when: Owner approves a written proposal with alternatives fixing prompt, tools, test feedback, edit, token and pass definitions, budgets, program shape and per-language build flags. It states that one edit replaces the whole file in all three languages and that Paka's per-definition tools are not measured, and rules whether the Paka reference's prompt tokens count toward the token metric.
- why this level: Each counting rule can tilt the result toward one language, and none can change once runs start.
- note: Done: approved by the owner on 2026-10-14; see docs/decisions/M3-01.md.

### M3-02 · L5 · Decide metric formulas, losing margin, model, repetitions and usage cap

- status: done
- needs: M3-01
- flags: decision, owner
- done when: Owner approves exact formulas for the three metrics, treatment of never-passing runs, repetitions, model id, usage cap, and the go, stop or inconclusive rule.
- why this level: The margin and never-passing-run handling decide whether the project stops; weak statistics make the verdict noise.
- note: Done: approved by the owner on 2026-10-14; see docs/decisions/M3-02.md.

### M3-03 · L5 · Decide rules for reference defects, task swaps and late changes

- status: done
- needs: M3-01, M3-02
- flags: decision, owner
- done when: Owner approves written rules for reference defects, toolchain defects, harness faults, tasks Paka cannot express, saturated tasks and amendments, naming who rules on each. The saturation rule that the calibration pass (M3-19b) applies is written here.
- why this level: Must foresee ways results get bent after the fact and close each without knowing the outcome.
- note: Done: approved by the owner on 2026-10-14; see docs/decisions/M3-03.md.

### M3-04 · L5 · Choose counted, reserve and practice tasks hard enough to separate languages

- status: done
- needs: M3-01, M3-02, M3-03
- flags: decision, owner
- done when: Owner approves a list of counted, reserve and practice tasks, each with an outline, what it stresses, and why Rust or TypeScript may fail. Each task names the M2-01 task class it falls in; a task outside every class is listed as such with the owner's reason.
- why this level: Easy tasks separate nothing; tasks suited to Paka's strengths bias the verdict; difficulty must be predicted unseen.
- note: Done: approved by the owner on 2026-10-14; see docs/decisions/M3-04.md.

### M3-05 · L2 · Pin and install the TypeScript compiler after the owner allows it

- status: done
- needs: M3-01
- flags: download, owner
- done when: With the network off, one command prints the pinned tsc version, then type-checks and runs a hello-world file; lockfile committed.
- why this level: No single given command: needs the exact version, the package, and a script that installs offline.
- note: The owner allowed this download on 2026-10-13. Version: the newest stable TypeScript on the day of the task, pinned exactly; the task names the package, source and size before it fetches. Node 22.22.1 and npm are installed; tsc is not. Pinned typescript 7.0.2 with @types/node 22.20.5 in harness/ts; check.sh is the one command. Programs are compiled with tsc, then run with node.

### M3-06 · L2 · Build the case runner: build, run and compare in three languages

- status: done
- needs: M3-01, M3-05
- flags: none
- done when: Tests show a known-good and a known-bad program in each language get the right verdict on hand-written cases, including a timeout. A task loader accepts a task folder in one written format (statement, cases, hidden cases) and rejects a malformed one.
- why this level: Golden-file runner with three fixed command pairs; one module, obvious tests.
- note: The task folder format is in harness/README.md: statement.md, cases/ and hidden/, each case with args, in/, stdout, status and out/. Cases run with only TZ and the C locale.

### M3-07 · L4 · Run submitted programs in a sandbox without network or hidden cases

- status: done
- needs: M3-06
- flags: none
- done when: Tests show a program in each language cannot reach the network, read hidden cases, write outside its directory or exceed limits, at build or run.
- why this level: A sandbox gap is silent and runs model-written code on the owner's machine; needs a reviewer told to break it.
- note: bwrap 0.11.1 is installed; no download. Builds and cases run under bubblewrap and prlimit with no network, an empty environment and only fixed paths (/tools, /build, /work). Known gaps: memory is capped per process and disk per file.

### M3-08 · L3 · Build the model client: send request, read reply, count tokens

- status: done
- needs: M3-01, M3-02
- flags: none
- done when: Against a fake `claude` command: the reply and its token counts are read correctly from the JSON output, retries are not double counted, and the usage cap stops calls.
- why this level: Own JSON reader, retry accounting and the usage cap; each is easy to get wrong.
- note: Runs the installed `claude -p` as a process with a pinned model, a fixed system prompt, no tools and no user settings; M3-01 fixes the exact flags and M3-18 freezes them. No key is involved. Lives in the harness crate. A failed, refused, compacted or wrong-model turn voids the run and its spent tokens still count toward the caps. The stream-json shapes and whether usage is per turn are confirmed in the M3-19 dry run; a hung process is left to M3-09. M3-01 sets one open `claude -p` process per run with stream-json in and out, and a check that the first call carries no outside text. Token caps: 16M input and 9.5M output, which the owner may lower; per-batch use is reported.

### M3-09 · L4 · Build the trial loop: prompt, edits, checks, raw logs and resume

- status: done
- needs: M3-06, M3-07, M3-08
- flags: none
- done when: With a scripted fake model, tests show correct edit and token counts, budget stops, full raw logs with the time of each model call, and resume after a kill without rerunning trials.
- why this level: Several modules meet; a miscount of edits or tokens silently skews two of the three metrics.
- note: Readings for the protocol (M3-18): a reply holds exactly one backtick-fenced code block, and none or several is a failed submission; args are shown JSON-quoted; there is no whole-run time limit, only the per-call one; a usage-cap void does not use up an attempt, other voids do; void attempts keep their tokens out of the run end line; tokens of a killed call can be missing from the ledger, so the cap may undercount; compiler text is logged in full and cut at 8,000 bytes only in feedback; a memory kill shows as a wrong exit status, under the label over time or memory. Writes each turn to the open process. A run whose process dies or compacts is void and run again under M3-03.

### M3-10 · L3 · Build the metrics tool: raw logs to three metrics and verdict

- status: done
- needs: M3-02, M3-03, M3-09
- flags: none
- done when: On a small hand-made log set with a hand-made rulings file (runs excluded or re-classed under M3-03), one command prints the three metrics per language and per task and the verdict, equal to values worked out by hand; the set includes a never-passing run and a ruled-out run.
- why this level: Never-passing runs and per-task grouping are easy to get subtly wrong, and this number decides the project.
- note: The metrics tool takes a runs root, a rulings file and an optional reference token count. It gives no verdict, and exits 20, unless every kept task has equal run counts in all three languages and no unfinished run; a void-ruled run is rerun in a new rep folder. Practice tasks and stray entries are errors; checking tasks against the frozen counted list waits for M3-18. Bands are decided on exact integers, and edges count as the better band. Formulas are pooled over runs (M3-02). More than 2 dropped tasks of 14 makes the verdict inconclusive: a third dropped task is enough.

### M3-11 · L4 · Write statements and hand-made tests for one batch of at most five tasks

- status: done
- needs: M3-04, M3-06
- flags: repeat
- done when: Task loader accepts the batch; each expected result has written hand working; a second-agent review finds no behaviour a statement leaves to the language. Each statement defines at least one error condition with an exact message and the one error status.
- why this level: A vague statement or one wrong hand-derived answer silently corrupts every run of that task.
- note: One task per batch; M3-04's list fixes how many (four for 20 tasks). Tasks live in experiment/tasks/<ID>/, with the hand working in working.md. Batch 1 (X1, X2, T1, T2, P1) is done; the harness appends the visible cases to the prompt. Batch 2 (P3, S2, S3, G1, G2) written; every case recomputed by an independent solver. Batch 3 (R1, R3, N1, N2, F1) written; every case recomputed by an independent solver. Batch 4 (F2 and reserves T3, V2, N3, V6) written; every case recomputed by an independent solver. All 20 tasks are in experiment/tasks.

### M3-14 · L3 · Write Rust reference solutions for ten tasks and list every test they dispute

- status: done
- needs: M3-11
- flags: repeat
- done when: Every task has a Rust solution built with the agreed flags; each case it fails is listed with a reason; the diff touches no test.
- why this level: Many small programs against given tests; hardest part is not bending a test to Rust's habits.
- note: One task per ten experiment tasks (two for 20 tasks). All 20 tasks pass every visible and hidden case with no dispute. Points a writer had to settle by reading, all agreeing with the cases: V6 entry lines of the wrong form and partly right headers, S3 a last comment-only line with no line feed, P1 the failing token as the one where recursive descent stops. Kept rustfmt-clean by hand, since cargo fmt does not cover experiment/.

### M3-15 · L3 · Write TypeScript reference solutions for ten tasks and list every test they dispute

- status: done
- needs: M3-11
- flags: repeat
- done when: Every task has a TypeScript solution that passes tsc with agreed flags; each failed case is listed with a reason; the diff touches no test.
- why this level: Many small programs against given tests; hardest part is exact integers beyond what a TypeScript number holds.
- note: One task per ten experiment tasks (two for 20 tasks). All 20 tasks pass every visible and hidden case with no dispute; BigInt is used in N1 and P1. Coverage gaps worth a case in M3-17: V6 has no case for an unreadable LIST file, and none for a WHICH that is neither `*` nor a valid name.

### M3-16 · L4 · Write Paka reference solutions for ten tasks and report every language or toolchain defect

- status: done
- needs: M3-11
- flags: repeat
- done when: Every task has a Paka solution the interpreter runs, or a filed defect report; each failed case has a reason; the diff touches no test.
- why this level: No Paka code exists to copy; author must tell own error from interpreter or reference defect.
- note: One task per ten experiment tasks (two for 20 tasks). For each filed defect the caller adds one level 4 task: done when a regression case written before the fix passes, the M2 exit check still passes, any reference change follows M3-03's rule, and the blocked Paka solution is added. All merge before M3-17. First ten tasks (F1 to R1) pass every case with no dispute. Tag check: all five counted tags and both reserve tags stand; F2 writes one file per answer rather than all at once, so a failed write can be named. P1 at the stated nesting of 100 reaches about 400 call frames, not under 200. Open parser gap: the match arm `Text(t) => _ = f(t)?` is refused although the spec allows an assignment as an arm. All 20 tasks pass every visible and hidden case with no dispute and no toolchain defect. Friction seen while writing them, as input for later language work: no nested patterns; no `_` arm on an enum; a bare-expression arm cannot return; no unit type; braces escaped in strings; no `"` inside interpolation; file steps need a hand-built state machine; no int-to-text function; `for _ in` refused; `find` cannot start at a position; `parse_int` too lenient for digit-only fields and no digit or letter test; no closures, so a run-time sort direction needs wrappers; no tuples, so matching on a pair needs nesting or a text key; `lines` hides a final line feed; Print and Exit boilerplate in every program. Confirm or correct the five counted tags and the two reserve tags (T3, V6) against the approved M2 rules.

### M3-17 · L4 · Settle every disputed test from the statement text alone

- status: done
- needs: M3-14, M3-15, M3-16
- flags: none
- done when: All reference solutions pass all cases in all three languages; each changed statement or expected result has a logged reason drawn from the statement. No defect report is open.
- why this level: Each ruling must come from the statement, not from whichever language's run looks right; errors are silent.
- note: No reference solution disputed any case, so no statement or expected result changed. The one defect report, a match arm `V => _ = f()?` refused by the parser, is fixed: the discard now goes through the line parser that statements and arms share, with conformance cases for it. This closes the parser gap in the M3-16 note.

### M3-18 · L2 · Freeze the protocol and make the harness refuse counted runs from other commits

- needs: M3-02, M3-03, M3-04, M3-09, M3-10, M3-17
- flags: owner
- done when: Before any model call the owner merges and tags (`protocol`) a protocol naming tasks, model, metrics, margin and toolchain versions; the harness refuses a counted run from any commit but the one tagged `run`.
- why this level: Assembles approved decisions into one file and adds one commit check; little to decide.
- note: Protocol file experiment/protocol.md and the commit check are in; open: model id, CLI fixed-text tokens, call time limit, CLI version, Paka reference size, batch order. Waits for the owner's protocol tag. The protocol carries the 20 task outlines, their classes, the tags, the reserve order, the band lines, the two token caps and error status 20.

### M3-19 · L3 · Dry-run the harness on practice tasks and fix its faults

- status: done
- needs: M3-18
- flags: owner, open-ended, model-calls
- done when: Practice tasks finish in all three languages on live model calls with zero harness faults in the last pass; fixes and amendments merged by owner. The run records the language reference's real token count from the model's usage report; over 8,000 is a protocol amendment. Raw logs merged.
- why this level: Faults can sit in any harness module; fixes must leave tasks, tests and counting rules untouched.
- note: A fault in the sandbox (M3-07) or in edit and token counting (M3-09) is fixed as its own level 4 pull request. One pass of 12 practice runs: all passed first try, no harness fault, 58.7k input and 12.1k output tokens. Measured: model claude-sonnet-5-5, 784 tokens of fixed CLI text, reference about 9,400 tokens, sent whole by amendment. Logs in experiment/runs/dry-1. Each pass is 12 runs (2 practice tasks x 3 languages x 2 runs), at most three passes; a fourth needs the owner's yes. It reports measured tokens per run and the projection for the rest.

### M3-19b · L2 · Run the Rust and TypeScript calibration pass and apply the swap rule

- needs: M3-19
- flags: owner, model-calls
- done when: Logs show every counted task run in Rust and TypeScript only, never Paka; each swap follows M3-03's rule and comes from the reserve list; the owner merges the amended list and tags the `run` commit. Raw logs merged.
- why this level: No design; must follow the pre-set rule exactly.
- note: First pass (logs in experiment/runs/calib-1): 108 runs, no harness fault; 15 of 18 tasks passed first try in all 6 runs, Rust first-try 88% and TypeScript 86% on the counted list, so 11 swaps were due and the reserve list could not supply them. Owner ruling: keep S3, G2 and N1, write 11 harder counted tasks (T4 T5 P4 P5 S4 G3 R4 R5 N4 F3 F4; T5 P5 R5 F4 tagged) and 4 reserves (T6 tagged, S5, N5, F5 tagged), then calibrate again. The harder tasks are written and merged; each has Rust, TypeScript and Paka references that pass every case. protocol.md now lists counted T4 T5 P4 P5 S3 S4 G2 G3 R4 R5 N1 N4 F3 F4 and reserves T6 S5 N5 F5, with the replaced tasks kept out of the lists. Next: calibrate the new list. Second calibration (experiment/runs/calib-2): 84 runs, all solved; first try Rust 29 of 42 (69%), TypeScript 33 of 42 (79%), inside the 30 to 85% gate. All 6 runs passed first try on T5, P4, S4, G3, R4, R5 and N4: 7 swaps due against 4 reserves, so the owner decides. Every F4 first try failed the same hidden case by printing the placeholder OLD instead of the name. F3-Rust-2 had two void attempts from a full disk and passed on its third. Owner ruling: keep the list as it stands, the list-wide gate being met; recorded in protocol.md. F4 error line clarified and F4 calibrated again (experiment/runs/calib-3): 4 of 6 first try. Next: tag run and start M3-20. 84 runs plus up to 24 for swapped-in reserve tasks. More than 4 swaps goes to the owner, not to a fifth swap.

### M3-20 · L2 · Run the counted experiment and publish the raw logs

- status: done
- needs: M3-19b
- flags: owner, model-calls
- done when: One command confirms every counted task, language and repetition has a finished trial log from the `run` commit; raw logs are merged.
- why this level: No design, but it spends the budget and must stop and report on any deviation, not improvise.
- note: Run from the run tag (02f35f5): all 168 runs ended, no void attempt, every run solved within 8 submissions. Raw logs in experiment/runs/counted, token ledger in experiment/runs/counted-usage.json. `metrics experiment/runs/counted <rulings> 9400` with no rulings yet: first-try Paka 75.0%, Rust 83.9%, TypeScript 87.5%; edits 0.34, 0.20, 0.21; output tokens 445,593, 192,932, 157,316. Verdict before M3-21 rulings: first-try gap 8.93 held, edits gap 0.143 held, token ratio 2.310 lost, so stop. The metrics tool refuses the batch ledger usage.json inside a runs root, so the ledger is kept beside it. 168 runs (14 tasks x 3 languages x 4 repetitions), in batches of one task; 126 if the fallback in M3-02 applies. Stops at the token caps.

### M3-21 · L4 · Rule on reviewed runs under the pre-set defect rules

- status: done
- needs: M3-03, M3-20
- flags: decision, owner
- done when: Every run the rules send to review has a recorded cause and ruling; a second agent re-rules a sample blind; the owner approves the list.
- why this level: Separating model error from reference or toolchain defect in transcripts is judgement that biases the result silently.
- note: Toolchain defects are ruled like mismatches and recorded as dropped tasks.

### M3-22 · L2 · Publish the report: three metrics per language with raw data

- status: done
- needs: M3-10, M3-21
- flags: none
- done when: One command rebuilds the report's three metrics per language, per task and per M2-01 class from the merged raw logs and rulings; every protocol deviation is listed. The report lists every difference between the `protocol` and `run` tags. The report command prints the `protocol` commit's date and the earliest model-call time across dry-run, calibration and counted logs, and fails if the tag is not older. The report lists what the verdict does not measure: pools, float ties, Unicode, user generics, closures, standard input, GUI, speed and overflow.
- why this level: Runs an existing tool on fixed data; only care needed is listing every deviation.

### M3-23 · L5 · Write the go / no-go analysis for the owner's decision

- status: done
- needs: M3-22
- flags: decision, owner
- done when: The analysis states the pre-set rule's verdict, every deviation and the limits of the evidence; the owner records go or stop in the repository.
- why this level: Can end the project; must weigh a noisy result against the pre-set rule without reading hope into it.
- note: Owner decision: continue to M4. The token cost is the target of dev mode's one-definition tools; the benchmark tracks it.

## M4 — Dev mode for headless programs

**Exit check.** One command on the build machine (built in M4-46) reports pass for every item: (1) on a generated headless program of the size fixed in M4-01 (5,000 definitions, worst of three runs), the 95th-percentile time from a body-edit request to the first handler of the rerun is under 100 ms for small, medium and large definitions; (2) a signature edit rechecks exactly the direct users of the changed interface, shown by a recheck counter against hand-written sets, and the edit reply lists exactly the definitions that broke; (3) after a breaking edit a run still executes the last version in which every definition checked, and fixing the users installs all changes together; (4) find, view, edit with a base version, and run pass hand-written transcripts over the versioned interface, each edit returning canonical text, typed errors and the broken list; (5) the wc sample runs under Console and ReadFiles grants and an ungranted effect is refused exactly as in the interpreter; (6) every M2 conformance test gives the same output, exit status and trap in dev mode as in the interpreter, with none unsupported, and the seeded generated-program and edit-sequence runs agree.

- M4-01b needs a download (Cranelift) and the owner's approval of every crate it brings.
- The database and tool tasks (M4-07 to M4-19) need no native code and are tested on the interpreter.
- No Miri, sanitizer or valgrind on the build machine. The allocation ledger (M4-25) stands in; it catches leaks and bad frees but not reads of freed memory.
- Can start first: M4-01, M4-07, M4-21, M4-47, M4-48.
- The owner is needed in: M4-01, M4-01b, M4-03, M4-04, M4-05, M4-06, M4-46, M4-47, M4-48.
- Downloads: M4-01b, M4-06.

### M4-01 · L4 · Fix the generator benchmark method, test definitions, program size and winning margin

- status: done
- needs: nothing
- flags: decision, owner, native-only
- done when: Before any measurement the owner approves a document fixing three Paka definitions (small, medium, large), timing start and end points, both generators' settings, the winning margin, and the benchmark program size and edit classes M4-44 will use.
- why this level: A method chosen after seeing numbers biases a choice that commits the dev-mode architecture.
- note: Done: approved by the owner on 2026-10-15; see docs/decisions/M4-01.md and bench/generators/defs.paka.

### M4-01b · L4 · Measure Cranelift and LLVM's JIT by the fixed method and record the choice

- status: done
- needs: M4-01
- flags: decision, owner, download, unsafe, native-only
- done when: One command reprints the timings of each generator measured, by the M4-01 method; Cranelift is built and timed first, and the LLVM side only if Cranelift fails the gate; its unsafe sits in one crate the owner allowed; the record is the M4-01 rule applied to the numbers.
- why this level: Hand-declared unsafe bindings to LLVM's C library, with no Miri to catch a wrong signature.
- note: The benchmark crate stays outside the workspace with its own lock file, so the --locked --offline build stays green; M4-06 brings the chosen crates in vendored. The owner allowed the Cranelift crates on 2026-10-13; the task names each crate, version and size before it fetches. LLVM 21's shared library is already on the build machine (`/usr/lib/llvm-21/lib/libLLVM-21.so`); nothing is installed. Cranelift 0.136.3 passed the rule (warm p95 of the large definition 2.1 ms, cold 3.6 ms); LLVM was not built. Result in docs/decisions/M4-01.md section 7.

### M4-03 · L4 · Propose the agent tool interface: transport, message shapes, versions, find queries

- status: done
- needs: M4-48
- flags: decision, owner
- done when: Owner approves a written interface with a schema for find, view, edit and run, and hand-written example exchanges for each. The base version of an edit is per definition.
- why this level: A compatibility promise to every agent; type-shape queries, token budgets and base versions have no settled meaning yet.
- note: Token budgets use M2-18's counting rule. Done: approved by the owner on 2026-10-15; see docs/decisions/M4-03.md.

### M4-04 · L4 · Propose where dev-mode code runs and how a run is stopped

- status: done
- needs: M4-01b
- flags: decision, owner
- done when: Owner approves a written choice among in-process, forked and separate-process runs, covering stuck handlers, memory reclaim and the 100 ms budget. It also covers how a visual app keeps its state in the same process while code is swapped (M5 needs it); stopping a run uses no signal handler for traps.
- why this level: Trades isolation against swap speed; a stuck or trapped run must never take the tool server down.
- note: Done: approved by the owner on 2026-10-15; see docs/decisions/M4-04.md. One runner process per server, started through std::process.

### M4-05 · L4 · Specify the dev-mode calling convention, trap returns and value layouts

- status: done
- needs: M4-01b, M4-04
- flags: decision, owner, native-only
- done when: Owner approves a specification covering context pointer, status returns, stack-limit check, slot calls, every type's layout and who frees each value. It names the code generator and the crate that turns generated code into runnable machine code, and says which runtime code the interpreter shares (recommended: the effect performer only). It names each M1-14 rule as kept or changed, says whether the dev runtime extends paka-rt (M1-16, M1-17) or replaces it, and names the crate that holds the unsafe runtime code; the M1 cases still pass on both back ends.
- why this level: Separately compiled definitions must agree exactly; a mismatch in layout, ownership or status handling is silent.
- note: Start fields, state and ExitWith reach the native back end here; tests/cases/parse/variant_payload then leaves the UNSUPPORTED list of paka/tests/native.rs. Per M4-04: an interrupt poll at every loop head and function entry, explicit checks for every trap, calls through a slot table, and a per-run memory cap checked by the allocator.
- note: Done: docs/decisions/M4-05.md. Every function is `(ctx, args..., out) -> status` (0 finished, 1 trapped, 2 stopped); trap kinds are numbered in the trap record (M2-06). The dev runtime extends paka-rt, whose `abi` modules keep the runtime's unsafe code; the new `paka-installer` crate holds the rest (a change to M4-04's one-crate wording).

### M4-06 · L2 · Vendor the chosen generator's crates and enforce locked offline builds

- status: done
- needs: M4-01b
- flags: owner, download, native-only, conditional
- done when: With the network off, cargo build --locked --offline passes from a clean clone, and an unlisted crate fails the check.
- why this level: Standard cargo vendoring; the only care is an approved-crates list that the build checks.
- note: Dropped if the generator chosen in M4-01b needs no outside crate. Runs if M4-01b records Cranelift; dropped if it records LLVM. M4-01b recorded Cranelift, so this task runs. Done: 38 crates in vendor/, read through .cargo/config.toml with the network off; .cargo/approved-crates.txt lists them and paka/tests/vendored.rs fails on any locked or vendored crate not listed. A clean clone with an empty cargo home builds bench/generators.

### M4-07 · L2 · Add a hand-written SHA-256 for content hashes

- status: done
- needs: nothing
- flags: none
- done when: The published SHA-256 test vectors, typed in by hand, pass under cargo test.
- why this level: Well-known algorithm with published test vectors, in one module.
- note: Done: paka/src/sha256.rs, one function over a byte slice; the FIPS 180-4 and NIST vectors and the padding edges pass.

### M4-08 · L2 · Give every definition a stable id and parse definitions singly

- status: done
- needs: M4-48
- flags: none
- done when: Tests show ids unchanged by body edits, reordering and rebuilds from text, and one definition reparses without touching others.
- why this level: Ids are bare names (M4-48); reparsing one definition reuses the existing parser.
- note: Done: parser::parse_def parses exactly one definition with positions from its own start, and parser::split gives each definition of a text as its id and its text. Comments are not carried yet (M4-15).

### M4-09 · L3 · Check and lower one definition from its body and used signatures

- status: done
- needs: M4-08
- flags: none
- done when: Per-definition results equal the whole-program checker on all conformance programs, and the read log lists exactly the hand-written expected names.
- why this level: Untangling whole-program passes so one definition reads only signatures, while logging every name it looked up.
- note: Done: paka::check_each gives each definition's first error and its reads: the definitions whose signatures it read (a variant's enum for a variant) and, as written, the names it looked up that name nothing. The resolver logs its lookups; the signature table logs every definition read through it, in checking and lowering. A test checks every definition of every conformance program in a copy where all it did not read, transitively, is blanked, and gets the same error and reads. Built-ins and locals are not logged. Two reads stay whole-program: the no-finite-value check follows held types transitively (M4-11's interface hash covers it), and core-form ids are positions in the program (M4-24's slots replace them).

### M4-10 · L3 · Build the exact who-uses-what index, including names that resolve to nothing

- status: done
- needs: M4-09
- flags: none
- done when: Fixture programs give the hand-written user lists, and adding a missing definition marks exactly its waiting users for recheck.
- why this level: Exactness under adds, deletes and renames; a missed dangling reference leaves a broken user unreported.
- note: The index is built from M4-09's reads. A name that names nothing is logged as written, so adding a variant must also wake the users waiting on the variant's name, not only on its enum's.
- note: Done: paka::index::Index maps each name read to the definitions that read it, kept up to date by `set` and `remove` as definitions are rechecked. `waiting` gives the users to recheck when a definition is added, changed or deleted: those of its name and its variants' names, and, because a suggestion is a read, those of a name that names nothing and that the new name could be suggested for. A test applies adds, a variant add, a delete and a rename, rechecks only what was woken, and gets the index of a clean rebuild.

### M4-11 · L4 · Compute the body and interface hash of every definition

- status: done
- needs: M4-07, M4-10, M2-09
- flags: none
- done when: For a hand-written list of edit kinds, tests show exactly the expected hashes change, including transitive by-value layout and recursion.
- why this level: Soundness of what each hash covers; one omitted input, such as a default value, silently keeps stale code.
- note: The closure hash arrives with its first reader, M7-09. Hashes come from the parsed definition, never from file bytes (M4-47), so the task does not need the printer.
- note: Done: paka/src/hash.rs; every DefCheck carries its body and interface hash. Both hash a position-free, length-prefixed text of the parsed definition. The body hash covers the whole definition and, for each name it read, the kind of definition that name names, or none. The interface hash follows docs/decisions/M2-09.md section 3: a user type inside a signature is written as its hash, and types that reach each other are hashed as one group. A test lists ten edit kinds with the hashes each must change (layout and comments none, a default value reaching through List, a recursive group, a label, a positional name, a variant, a missing name defined), and checks that order changes no hash.

### M4-12 · L4 · Recheck after an edit with early cutoff and list what broke

- status: done
- needs: M4-11
- flags: none
- done when: Scripted edits recheck exactly the hand-written sets: a body edit one definition, a signature edit its direct users; broken definitions are listed. A type-layout edit rechecks the direct users of every definition whose interface hash changed, against a hand-written set that reaches past the edited type's own users.
- why this level: Invalidation must be exact: too little leaves stale results silently, too much breaks the direct-users promise.
- note: Direct users means the direct users of every definition whose interface hash changed (paper section 5).
- note: Done: `paka::workspace::Workspace` keeps a program as definitions. An edit rechecks the edited definition when its body hash changed, the users of names that appeared or went, and the direct users of every definition whose interface hash changed; only those take fresh results, and `Recheck.broken` lists their errors. A kept error moves with its definition. The checker still runs over the whole program (cost is M4-45). Lowering adds reads only on a clean program, so when the program turns clean or broken every definition takes its fresh reads. `tests/workspace.rs` scripts eight edits with hand-written sets; adding a field to Point rechecks Line, Path, first, span and total, past Point's own users. After each edit the results equal a fresh check of the same text.

### M4-13 · L3 · Keep the last fully checked version runnable while newer edits are stored

- status: done
- needs: M4-12
- flags: none
- done when: Test: after a breaking signature edit the interpreter still runs the old version; fixing the users releases all changes together.
- why this level: Two versions of the program side by side, and an all-or-nothing change set once the newer one checks.
- note: Done: `Workspace::released` is the text of the last version without errors; it moves only when a newer version checks clean, so the edits between are released together. `tests/release.rs` breaks `count`'s signature, runs the released text with `paka run` (the old status 7), fixes the caller and runs the new one (status 9).

### M4-14 · L4 · Test that incremental state equals a clean rebuild after random edits

- status: done
- needs: M4-13
- flags: open-ended
- done when: 10,000 seeded edit sequences end with ids, index, hashes and check results identical to a rebuild from text; a planted bug is caught. A coverage report shows every edit kind produced (body, signature, type layout, add, delete, rename), and one planted staleness bug per kind is caught.
- why this level: It must find stale results; a weak edit generator passes silently, and M4-43 leans on it.
- note: A rename is generated as an add followed by a delete (M4-48).
- note: Done: `tests/edits.rs` runs eight seeded edits on each of 10,000 generated programs (`PAKA_EDIT_SEEDS`) and after every step compares the workspace with one built from its text: index, names, errors, reads and both hashes. Edits: a body gains a `let` reading a name, sometimes misspelled; a function gains a parameter; a record gains a field, with or without a default, or an enum a variant; an add restores a deleted definition or adds a new function; a delete; a rename. The report counts each kind made and applied (an edit with a program-wide error, such as an enum renamed beside itself, is refused). Planted bugs, at 2,000 sequences: index not updated for a changed definition (caught by body edits and others), function interface hash without parameters (signature), record or enum layout left out of the type hash (layout), appeared names not waking their users (add), gone names not waking theirs (delete and rename), the near rule dropped (add), the relowered refresh dropped (all). Not caught, by design: dropping the body-hash rule, since a definition whose text changed always takes fresh results; that rule only decides what an edit reports, which `tests/workspace.rs` checks.

### M4-15 · L3 · Print any definition in the one canonical layout, keeping comments

- status: done
- needs: M4-47
- flags: none
- done when: Hand-written golden files pass, and on all conformance sources printing is idempotent and reparses to the same tree. `paka check` treats non-canonical text as M4-47 rules, with a hand-written case for each outcome; stored sources are handled as M4-47 rules.
- why this level: Comments must survive and printing must be stable: print, parse, print again gives the same text.
- note: A plain tree walk with no width fitting; parentheses are kept as written (M4-47). It attaches comments in the parser where the printer puts them, and tests that the M4-47 samples print unchanged.
- note: Done: paka/src/print.rs. The parser tree stays without comments: the printer lexes the source once more and places each comment by lines, above the definition, statement, match arm or app key it sits on or before, or as a block's last line; comments after the last definition go above it. Golden files are in tests/print, with samples.paka holding the M4-47 samples. Over every source under tests and examples that parses, printing keeps the hashes and the comment count, and printing again changes nothing. `paka check` already accepted other layouts and refused `return` on the last line and a semicolon with their fixes; tests/check.rs now holds a case for each. Storing canonical text belongs to M4-17. Known limit: doubled parentheses such as `((x))` print once, `(x)`.

### M4-16 · L2 · Serve the tool interface: transport, JSON reading and writing, request dispatch

- status: done
- needs: M4-03
- flags: none
- done when: Hand-written request and reply transcripts pass, including malformed input and an unknown interface version.
- why this level: A request loop and a small JSON reader in std; shapes come from the approved interface.
- note: Reuses one shared standard-library-only JSON module; M2-04's writer and M3-08's reader move into it. Per M4-03: JSON lines on standard input and output, a `v` check on every request and no handshake; unknown fields are refused; one server per program is kept by a standard-library file lock.
- note: Done: `paka serve <path>` and paka/src/serve.rs. The JSON reader and writer now live in the `paka-json` crate (json/), used by the command and the harness. Each request is checked in this order: one JSON object, `id`, `v` (another version is `bad_version` before any field is judged, since a version may change the fields), `call`, then the call's fields by name, type and need, as M4-03 section 3 lists them; a repeated key is not one JSON object. The program is locked with `File::try_lock` for the server's life; a second server and a program that does not parse exit 1 before serving. Hand-written transcripts are in tests/serve. Until M4-17, M4-18, M4-19 and M4-39 land, a well-formed call is answered `unknown_call` with "... is not served yet".

### M4-17 · L3 · Implement edit: replace, add or delete one definition against a base version

- status: done
- needs: M4-13, M4-15, M4-16, M4-48
- flags: none
- done when: Transcript tests: a stale base is refused; an accepted edit returns canonical text, typed errors and the exact broken list; files match. Two edits to different definitions from one base both land; two edits to the same definition, or adding the same name twice, refuse the second.
- why this level: Source files, database and version check must change together or not at all, even when the edit breaks users.
- note: Edit accepts any layout that parses and stores canonical text for the edited definition only (M4-47). In a directory program an add writes a new file named after the definition, a delete removes an emptied file, and text whose name differs from the call's name is refused (M4-48). Per M4-03: the base version is the 12-digit hash of canonical text; null `base` adds, null `text` deletes; a stale base returns the current version and text; text that does not parse is refused with kind `parse`.
- note: Done: paka/src/session.rs holds the served program: its workspace and its files. Refusals come in this order: `not_found`, `name_taken`, `stale_base`, then `parse`, `not_one_definition` and `name_mismatch` on the text. Two refusals M4-03 does not list: `conflict` when the edit leaves the program unable to check at all (a built-in's name, a variant named like a type, a second app), and `io` when the file cannot be written; both store nothing. An add whose file name passes 255 bytes is a `bad_request`. Files are replaced through a copy renamed over them; a single-file program's lock moves to each new copy. A delete takes the comment lines right above the definition and the rest of its last line; comments in sent text outside the definition are written to the file but are not part of its text or version. `broke` lists the definitions that checked before the edit and do not after. The `run` field of an edit waits for M4-39. Transcripts tests/serve/edit.txt and dir.txt check replies and the files after.

### M4-18 · L2 · Implement find by name or user, and view within a token budget

- status: done
- needs: M4-10, M4-16, M4-15
- flags: none
- done when: Transcript tests return hand-written results; a view holds one body plus used signatures and never exceeds its budget.
- why this level: Index lookups plus trimming output to a budget; the rules come from the approved interface.
- note: Token budget uses M2-18's counting rule. Per M4-03: name patterns with `*`, the `users_of`, `kind` and `failing` fields, paging with `after`, the `more` count and current errors in a view. View and find print canonical text and take the version from it.
- note: Done: the server loads the program in canonical text, so view text, versions and error positions agree; files keep their layout. A budget keeps the longest prefix of entries that fits; a view whose body with no uses is over it fails with `budget`. An exact name finds built-ins and a variant's enum; `users_of` a variant or built-in lists the definitions that name it. `takes` and `gives` answer unknown_call until M4-19. The transcript `tests/serve/find.txt` matches M4-03 section 4.

### M4-19 · L3 · Implement find by type shape

- status: done
- needs: M4-18
- flags: none
- done when: Transcript tests over a fixture program return exactly the hand-written matches for each query form in the interface.
- why this level: Matching a query against signatures with generics and argument order needs a small, exact matching rule.
- note: Per M4-03: an exact takes-and-gives match over the program's own fn and view definitions, with no wildcards or ranking; may be re-graded to level 2.
- note: Done: each query type is parsed as a parameter type and printed canonically, so spacing does not matter; `inout` or text that is not one type is a bad_request. Each type in `takes` claims one parameter, `gives` must equal the result, and a fn without a result never matches `gives`. Own definitions take no type parameters, so the match is on canonical text; no view definitions exist yet. Transcript `tests/serve/shape.txt`.

### M4-21 · L4 · Generate random well-typed terminating Paka programs from a seed

- status: done
- needs: nothing
- flags: native-only
- done when: 10,000 seeds give programs that all check and finish in the interpreter; a coverage report shows every core-form node kind produced.
- why this level: Programs must pass the checker, terminate, and still reach overflow, bounds, NaN and ownership corner cases.
- note: Done: paka-gen writes one program per seed (`paka-gen SEED` prints it); `paka/tests/generated.rs` checks and runs 10,000 seeds (`PAKA_GEN_SEEDS` sets the count): all check and finish (exit 0: 6214, trap 3: 3119, chosen exit codes: 667) and every core-form node kind appears.

### M4-22 · L3 · Compute the native layout of every type

- status: done
- needs: M4-05
- flags: native-only
- done when: Sizes, alignments and field offsets equal hand-written expectations for fixture types covering every type form in the reference. The layout table is back-end neutral: release mode (M6) reads the same table.
- why this level: Enums with payloads, nesting and generics; offsets must match the specification exactly.
- note: Done: `paka/src/layout.rs` lays out every type the checked signatures meet: size, tag, each alternative's field offsets and boxed fields, and the owned-field lists that copy and release walk. Beside M4-05 section 10's String, List, Map and box entries, a tagged value held inline (an enum, `Option`, `Result`) is a `Value` entry walked through its own record; a nested record is flattened into its holder's list. `WriteFile`'s `then` is a box of the message or `u64::MAX`. The record's equality and order words name definitions, so the emitter fills them (M4-31).

### M4-23 · L4 · Create the installer crate: load code into memory that is never freed

- status: done
- needs: M4-05
- flags: unsafe, native-only
- done when: Tests install and call functions; memory is never writable and executable together; addresses never repeat; each unsafe block states its invariant.
- why this level: Unsafe executable memory with no Miri available; a wrong permission or length corrupts the process silently.
- note: Done: `installer/` (`paka-installer`) copies each batch of functions into one fresh anonymous mapping, each function 16-byte aligned, then turns the mapping from read-write to read-execute; it is never unmapped, so addresses never repeat. Linux x86-64 only. Every `unsafe` block carries a `SAFETY:` comment, enforced by the `undocumented_unsafe_blocks` lint. M4-04 frees replaced code; this follows the task title instead and keeps it, which is not a problem for a dev session's size.

### M4-24 · L4 · Add call slots, grouped slot swaps and safe entry into installed code

- status: done
- needs: M4-23
- flags: unsafe, native-only
- done when: Tests: a swapped slot changes the callee; a group swaps together; a swap attempted during a run is refused.
- why this level: Swaps must be impossible while code runs; a wrong function type at the entry point is undefined behaviour.
- note: Done: `Slots` in `installer/src/slots.rs`. A swap writes a whole group or, when an id has no slot, nothing. Swaps and runs exclude each other through one state word, so a swap during a run, or a run during a swap, is refused with `Busy`. `run` points the context's first word at the table, then calls the slot as `fn(ctx, out) -> i32`; it stays `unsafe` because only the caller knows the slot holds code of that type. The table has a fixed length; growing it for new definitions waits for M4-37.

### M4-25 · L4 · Add runtime allocation with a ledger that reports leaks and bad frees

- status: done
- needs: M4-05
- flags: unsafe, native-only
- done when: Tests: a planted leak, double free and wrong-size free are each reported; allocation failure returns a trap status, not an abort.
- why this level: Unsafe allocator whose ledger stands in for Miri; a wrong ledger hides every later memory bug.
- note: Done: `paka_rt::abi::dev`. `Heap` records every block by address and size; a free of an address not live (double or stray) or with another size is recorded as a `Fault` and never touches memory the ledger does not hold; a wrong-size free releases the block with its own size. `leaks()` lists live blocks for M4-40, and dropping the heap frees them. `rt_alloc` checks the run's cap before asking the host; a refusal either way sets the context's trap to `memory` (8) and returns status 1. `Context` holds §4's four words, then the trap kind and the heap; M4-27 extends its trap record.

### M4-26 · L3 · Emit code for integers, floats, booleans, locals and control flow

- status: done
- needs: M4-06, M4-24, M2-08
- flags: native-only
- done when: M2-08's integer and non-NaN float vectors pass in dev mode; division and shifts are guarded explicitly; a test scans emitted dev code for a banned-operation list (self-trapping divide, trapping float-to-int, trap instructions) and fails on any hit.
- why this level: Every checked operation needs an explicit compare and branch; no instruction may fault by itself.
- note: Done: `paka/src/emit.rs` compiles scalar locals, control flow, operators and scalar built-ins; every check branches to `rt.trap`, and a select makes each divisor safe. All 427 Int, Float and Bool vectors pass as machine code, NaN ones included (`installer/tests/vectors.rs`); `emit/tests.rs` scans the IR for banned operations.
- note: Targets the code generator and the machine-code crate named in M4-05's approved specification; do not choose another. Follows docs/decisions/M2-06.md for traps, holes and examples.

### M4-27 · L3 · Emit calls through slots with status returns and a stack-limit check

- status: done
- needs: M4-26
- flags: native-only
- done when: Tests: every trap kind the M4-26 subset can raise (overflow, division, shift, failed conversion) surfaces through nested calls with its kind and source position; unbounded recursion ends in a stack trap, not a crash.
- why this level: Every call site must test the status; deep recursion must trap cleanly instead of overflowing the real stack.
- note: Done: a call loads its slot and checks the status; a trap pushes the call site through `rt.push_frame`, which keeps eight frames and an overflow flag. Each function checks the stack pointer against `ctx.stack_limit` after the interrupt poll; `paka_rt::abi::dev::on_runner` runs code on a 64 MiB thread with the limit 1 MiB above its bottom. `installer/tests/calls.rs` covers all four trap kinds through nested calls and recursion.
- note: Follows docs/decisions/M2-06.md for traps, holes and examples.

### M4-28 · L3 · Canonicalise NaN in compiled floating-point code

- status: done
- needs: M4-26
- flags: native-only
- done when: All NaN vectors from M2-08 pass bit for bit in dev mode and the interpreter.
- why this level: Hardware and constant folding give different NaN bits; every producing operation must be covered without missing one.
- note: Done: no built-in row exposes Float bits (`fixed` prints `NaN` whatever the sign), so the emitter adds nothing. Every NaN vector passes as machine code and in the interpreter; `installer/tests/vectors.rs` fails if they stop being run.
- note: M2-03 made NaN bits unobservable: compiled arithmetic emits nothing, so this task shrinks to confirming that no operation exposes Float bits.

### M4-29 · L2 · Call maths functions through private runtime names from compiled code

- status: done
- needs: M4-26, M2-08b
- flags: unsafe, native-only, conditional
- done when: Maths vectors match the interpreter bit for bit; a test fails if emitted code names a libm function or maths intrinsic.
- why this level: Thin exported wrappers and one emitter rule, plus a check that no known maths name is emitted.
- note: Only if M2-03 approves maths functions. Its unsafe is export attributes on scalar functions only.
- note: Closed without work: M2-03 approves no maths functions beyond the table rows, which M4-26 compiles inline (M2-08b).

### M4-30 · L2 · Compile holes and examples; an example reaching a hole reports unfinished

- needs: M4-27
- flags: native-only
- done when: Fixtures: a program with an unreached hole runs; a reached hole traps naming it; example statuses equal the interpreter's.
- why this level: A hole is one trap kind carrying its name; examples are functions returning a status.
- status: done
- note: Done: a reached hole traps `hole` at a site naming it; examples compile as Bool functions. `installer/tests/examples.rs` runs every conformance example program the emitter compiles and matches each status with the expected output; a program that fails to check there fails the test. The emitter now refuses a function whose stack slots pass 256 KiB (M4-05 section 6), imports each signature once per function, and `installer/tests/calls.rs` shows an interrupt stopping a loop and its caller with status 2 and no trap.

### M4-31 · L3 · Emit code for records, enums and match

- needs: M4-22, M4-27
- flags: native-only
- done when: Hand-written fixture programs covering records, defaults, enums with payloads and nested match return the hand-written expected values when called through M4-24's entry, in dev mode and in the interpreter.
- why this level: Payload layouts and match lowering across nested patterns; default field values must follow the reference.
- status: done
- note: Done: a record, enum, `Option` or `Result` whose layout owns nothing is held in memory and passed as its address; a local gets its own stack slot, a temporary a new one, and a result is written whole through `out`. A record or variant stores each field as it is evaluated, and `match` dispatches on the tag word, copying bound fields into their locals. The layout table now reads the core form, so both back ends share it. The fixture is the conformance program `examples_records`: its examples compare the values, and `installer/tests/examples.rs` requires it to compile with the interpreter's statuses. Types that own a String, List, Map or box, equality on records and variants, and Effect wait on M4-32 and M4-33.

### M4-31b · L3 · Emit the error-handling forms

- needs: M4-31
- flags: native-only
- done when: Hand-written fixtures for every error-handling form in the reference, early-exit paths included, return the expected values when called through M4-24's entry, in dev mode and in the interpreter.
- why this level: Known desugaring; early-exit paths are easy to miss.
- status: done
- note: Done: `?` reaches the emitter as a match and a `return` (docs/core.md), so it needed nothing new; `unwrap_or` and `ok_or` compile in place. The fixture is the conformance program `examples_errors`: `?` on `Option` and `Result`, inside a loop, nested in a call and followed by a field, and through `ok_or`. Error types that own a String wait on M4-32.

### M4-32 · L4 · Emit strings and lists through shared runtime functions with bounds checks

- needs: M4-25, M4-31
- flags: unsafe, native-only
- done when: Hand-written fixtures for strings, interpolation and lists match in both back ends, and every out-of-range access traps.
- why this level: Unsafe runtime functions on memory laid out by compiled code; every index needs an explicit bounds branch.
- status: done
- note: Done: a String or List is held in memory as M4-05 section 8's words and passed as its address until M4-34's prefixes. Literals, interpolation, `+`, `+=`, comparisons, indexing, slicing, `len`, `is_empty`, `min`, `max`, `code_at` and `from_code` compile; the String rows run `paka_rt::ops::Text` through `rt.text`, so both back ends share them, and a List's items, slices and walks have their bounds checks inline, with new blocks from `rt.list`. The size cap moved into `paka_rt::ops`. Until M4-33 places copies and frees, no block is changed in place: a write through a List item copies the List first, which costs O(n) per write, and old blocks stay live until the run ends. The fixtures are the conformance programs `examples_text`, `examples_lists`, `examples_text_traps` and `examples_list_traps`, one trap per out-of-range access; `installer/tests/examples.rs` requires them to compile with the interpreter's statuses. The other text rows (`split`, `trim`, `replace`, `pad`, `repeat`, `join` and the rest) move into `ops` with the first compiled program that needs them; until then the emitter refuses them by name.

### M4-33 · L4 · Place moves, copies and drops in one pass shared by the native back ends

- needs: M4-32
- flags: native-only
- done when: Fixtures covering early exit, match arms and loops return the hand-written expected values through M4-24's entry in both back ends and finish with a clean ledger. Fixtures with hand-written drop lists pass; the pass is back-end neutral, so release mode (M6) reuses it.
- why this level: A wrong drop is a silent leak, double free or use after free on some control-flow path.
- note: Done: `paka/src/owners.rs` tracks the owner of each local and temporary by scope and knows nothing of the machine, so release mode reuses it; its unit tests check hand-written drop lists for statements, scopes, loops, `break`, `continue` and `return`. The emitter moves a temporary into its new owner and copies anything else through `rt.copy`; each owner is dropped through `rt.drop` at its statement's or scope's end, on overwrite, and before `break`, `continue` and `return`. Both walk M4-05 section 10's type records, which `paka_rt::abi::dev::records` builds from the layout table. A `for` name borrows the walked List, a match bind is copied, and a slice copies its view, so its items are deep copies. M4-32's copy on write is gone: List items are written in place. The fixture is the conformance program `examples_owned`, covering early exits, match arms, loops, overwrites and nested owners; `installer/tests/examples.rs` now requires every compiled fixture that finishes to leave no leak and no fault in the ledger. Maps and boxes wait on their tasks.

### M4-34 · L3 · Emit inout arguments and borrows, including slices of lists

- needs: M4-33
- flags: native-only
- done when: Fixtures for inout fields, list items and slices match the interpreter; the ledger shows no allocation for a borrow.
- why this level: Borrowed parts must point into the caller's value without copying; writes through inout must land exactly.
- note: Done: an `inout` argument's operands are evaluated in its position and each place is walked to its address after all arguments (docs/core.md); a whole local held in a variable is stored in a slot for the call and read back after it, and an `inout` parameter not held in memory is a pointer its reads load and its writes store through. A List slice is a view (ptr at its first item, len, cap 0) that a read copies and an argument, a `for` walk or a built-in borrows; M4-05 section 5 records why read-only Strings and Lists pass as pointers rather than prefixes. The fixture is the conformance program `examples_inout`: Int and Bool locals, fields, List items, writes that stay after a `return`, and lent slices. `installer/tests/borrows.rs` counts blocks with the heap's new `made` and finds none for a borrow. A String slice is still a new String, and built-ins with an `inout` parameter (`push`, `pop` and the rest) wait on the tasks that compile them.

### M4-36 · L4 · Drive compiled headless apps: first message, effects, console and file grants

- needs: M4-33, M4-34, M4-37b
- flags: unsafe, native-only
- done when: The wc sample prints the hand-written count in dev mode; an effect outside the grants is refused exactly as in the interpreter.
- why this level: Unsafe decoding of effect values in native layout; an ungranted effect must be refused identically.
- note: Follows M4-05's ruling on which runtime code the interpreter shares.

### M4-37 · L3 · Compile changed definitions one by one and install them together

- needs: M4-13, M4-27
- flags: native-only
- done when: A compile counter shows one compile per body edit; reverting an edit compiles nothing; a failed compile installs nothing. A signature edit that leaves its users valid recompiles exactly the edited definition and its direct users, checked against a hand-written set by the compile counter; the cache key includes the interface hash of every definition used.
- why this level: Glue from change set to slots; cache keys must include hashes, compiler version, target and mode.

### M4-37b · L3 · Emit function values and variant constructors used as functions

- needs: M4-33, M4-37
- flags: native-only
- done when: Hand-written fixtures for function values and for variant constructors used as functions (passed, stored, called through slots) return the expected values when called through M4-24's entry in both back ends, with a clean ledger.
- why this level: Indirect calls through slots with a known technique.
- note: Becomes level 4 if M2-09 allows a function value to capture.

### M4-38 · L4 · Compile generic definitions in dev mode as the generics decision prescribes

- needs: M4-33, M4-37, M2-09
- flags: native-only
- done when: Generic fixtures match the interpreter; editing a generic body recompiles only what the decision says, shown by a compile counter.
- why this level: Generic code meets slots, hashes and layouts at once; a stale or missing instantiation is silent.

### M4-38b · L4 · Emit the built-in map with its fixed iteration order

- needs: M4-32, M4-38, M4-40, M4-31b
- flags: unsafe, native-only
- done when: M2's collection conformance cases agree with the interpreter in dev mode with a clean ledger.
- why this level: Unsafe container with a fixed order; a wrong order or drop is silent.

### M4-39 · L4 · Rebuild and rerun the program after each accepted edit; report its output

- needs: M4-04, M4-17, M4-36, M4-37
- flags: unsafe
- done when: Transcript tests: each accepted edit yields the new run's output; a looping program is stopped; a broken edit leaves the old result. The interface's run call passes M4-03's hand-written exchanges (output, exit status, trap report).
- why this level: Races between a running program, a swap and the next edit; a stuck run must stop without harming the server.
- note: Runs happen in the runner process (M4-04); the server has no unsafe. Calling compiled code and the slot table are the unsafe, in the installer crate. Per M4-03: a run happens only when an edit carries a `run` field or a run call is made; the server keeps no run settings; time and output limits come from the request. An edit that is not installed returns `run` null, and a run call runs the last fully checked version or answers `not_runnable`. A stuck run ends at the interrupt poll; a runner that does not answer 100 ms after the deadline is killed and restarted, and a crash reports `stopped` "crash" (M4-04).

### M4-40 · L3 · Add dev mode to the differential harness and run all conformance tests

- needs: M4-36, M4-37
- flags: native-only
- done when: One command runs every M2 conformance test in both back ends and lists each as agree, differ or unsupported. A self-test with a planted difference in output, in exit status, in trap kind and in trap position lists each as differ; death by signal counts as differ. Agree means the same output, exit status and the trap fields M2-06 fixes as meaning. The command also reports, per case, whether the dev-mode run ended with a clean ledger (M4-25).
- why this level: Defining equal results across back ends, trap kind and place included, while excluding stack and memory limits.

### M4-41 · L3 · Build the fuzz loop: two back ends, compare, shrink, keep failures

- needs: M4-21, M4-40
- flags: native-only
- done when: Each of five planted code-generator faults is found from a seed and shrunk to under 30 lines.
- why this level: A reducer that keeps the divergence and stays well-typed while shrinking.

### M4-41b · L4 · Run 100,000 generated programs and fix every divergence

- needs: M4-41, M4-42
- flags: native-only, open-ended
- done when: 100,000 seeds agree in both back ends; each fixed divergence has a regression test.
- why this level: Each divergence is a miscompile or meaning bug across checker, generator and runtime.

### M4-42 · L4 · Sort dev-mode differences into one fix task per root cause, then see them closed

- needs: M4-28, M4-29, M4-30, M4-34, M4-38, M4-40, M4-31b, M4-37b, M4-38b
- flags: native-only, open-ended
- done when: Every differing or unsupported case in M4-40's report is assigned to a fix task; the caller adds one level-4 task per root cause (done when its tests leave the differ and unsupported lists and a regression test is added); with all merged, the differential command reports zero differ and zero unsupported with a clean ledger.
- why this level: Root causes span checker, code generator and runtime; the remaining divergences are the subtle ones.
- note: M4-35 is dropped by docs/decisions/M2-10.md.

### M4-43 · L4 · Compare dev mode with the interpreter after random edit sequences

- needs: M4-14, M4-21, M4-39, M4-42
- flags: native-only, open-ended
- done when: 10,000 seeded edit sequences: after every accepted edit dev output equals the interpreter on the last fully checked version; planted staleness is caught.
- why this level: Finds stale compiled code after signature, layout and generic edits, a failure nothing else would notice.

### M4-44 · L3 · Build the repeatable benchmark for edit-to-running time

- needs: M4-39, M4-01
- flags: none
- done when: One command prints median and 95th-percentile times for the six M4-01 edit classes (three gated) on a program whose size is an argument, 5,000 by default, written by its own fixed-shape generator as one file per definition; timing runs from request written to reply read.
- why this level: Honest timing from request to first handler of the new run, on a generated program of stated size.
- note: Per M4-03: one timed sample is one edit request with a `run` field.

### M4-44b · L4 · Profile edit-to-running time and list fixes with expected gains

- needs: M4-44
- flags: conditional
- done when: A committed profile splits the time across checker, database, code generator and installer; each proposed fix names its expected gain.
- why this level: Finding where the time goes on a speed-critical path across four components.
- note: Dropped, and counted as done for M4-45, if M4-44 already reports a 95th percentile under 100 ms for all three sizes.

### M4-45 · L4 · Bring edit-to-running time under 100 ms

- needs: M4-44b, M4-42, M4-43
- flags: open-ended
- done when: The benchmark's 95th percentile is under 100 ms for all three edit sizes, and M4-40's command and M4-43's edit-sequence run still pass.
- why this level: Performance-critical path across checker, database, code generator and installer; fixes must not change any result.

### M4-46 · L2 · Run the whole M4 exit check with one command and record results

- needs: M4-19, M4-41b, M4-42, M4-43, M4-45
- flags: owner
- done when: One command on the build machine reports pass for every item of the exit check; the owner signs the recorded results.
- why this level: Wires existing tests and the benchmark into one script with pass or fail per exit-check item.

### M4-47 · L4 · Decide the canonical layout and how tools treat other layouts

- status: done
- needs: nothing
- flags: decision, owner
- done when: Owner approves the layout rules with one sample per construct, and whether check rejects or rewrites other text. It also rules what happens to sources stored before the layout existed.
- why this level: One layout is a promise of the paper; every tool and every stored file follows it.
- note: Done: approved by the owner on 2026-10-15; see docs/decisions/M4-47.md.

### M4-48 · L4 · Decide a definition's full name and the file tools store it in

- status: done
- needs: nothing
- flags: decision, owner
- done when: Owner approves the full-name form, the file a new definition lands in, and that packages stay undecided through M7.
- why this level: Stable ids and every tool call are built on the name; changing it later invalidates stored versions.
- note: Done: approved by the owner on 2026-10-15; see docs/decisions/M4-48.md.

## M5 — Live GUI on Linux

**Exit check.** One script in the repository, run in the owner's GNOME Wayland session on the build machine, exits 0 after all seven parts. (1) The counter sample and the to-do sample each open a window and log a first presented frame. (2) Through Look, a click raises the count and typing plus Add adds a row; replacing ??reset through Edit turns the placeholder into a button while the count in the tree stays the same. (3) An edit that changes the state type with no migrate and no reset is refused and the old version still answers a click; the same edit with a reset is accepted; the same edit with a migrate is accepted and the count is kept. (4) A view that traps leaves the tree text and the screenshot unchanged; a handler that traps leaves the tree unchanged and performs none of its effects. (5) With WAYLAND_DISPLAY and DISPLAY unset, the to-do UI example (start, type, click, read the tree) runs and the toolchain reports the status that was written by hand beforehand. (6) Look on the windowed app returns the tree text and a PNG screenshot and accepts a click and a keystroke. (7) M5-26's two-back-end command reports no tree difference and the reference with views passes its size test. The owner watches parts 1 to 3 on screen once. If the owner's M5-08 ruling requires a screen reader or IME, one clause is added for each.

- M5-08 gates everything that draws and needs downloads (winit, vello_cpu, Parley, AccessKit). Ask for the permission on day one.
- UI examples act on the element tree, not on pixels, so they need no window, font or outside crate.
- No golden screenshots: drawing is tested by hand-chosen pixels, layout by hand-computed rectangles.
- M5-17 to M5-19 and M5-31 to M5-35 assume the trial's stack and elements built from scratch; if M5-08 rules otherwise they are re-cut.
- Left out: clipboard, scrolling, partial redraw, themes, more than one window, X11.
- Can start first: M5-01, M5-07, M5-08.
- The owner is needed in: M5-01, M5-02, M5-03, M5-04, M5-05, M5-06, M5-07, M5-08, M5-09, M5-10, M5-36, M5-37, M5-42.
- Downloads: M5-08, M5-10.

### M5-01 · L5 · Decide what a view is and what its body may contain

- needs: nothing
- flags: decision, owner
- done when: Owner approves a written proposal with alternatives; it shows both sample views and says how a visual app starts and ends.
- why this level: Shapes the language: child lists, loops and conditions in a view, and views calling views, checkable from signatures.

### M5-02 · L5 · Decide key rules: who needs one, scope, tree-wide ids, duplicates

- needs: M5-01
- flags: decision, owner
- done when: Owner approves the rules, including what a duplicate key shows and when kept element state is dropped.
- why this level: Element identity across edits and loops; sibling-scoped keys must still yield ids unique across the tree.

### M5-03 · L5 · Decide the text input model: text held by app or element

- needs: M5-02
- flags: decision, owner
- done when: Owner approves a proposal with both models written out for the to-do input, including preedit and a trap.
- why this level: Decides who owns typed text; interacts with IME preedit, state rollback, keys and what examples can observe.

### M5-04 · L5 · Decide the elements M5 builds and every argument of each

- needs: M5-01, M5-02, M5-03
- flags: decision, owner
- done when: Owner approves a table of the elements M5 builds (about seven) with arguments, events and default look, and a list by name of the other elements as not decided; the owner either places them in a milestone or records the paper's "about 20" as open.
- why this level: Fixes the built-in vocabulary of every Paka GUI; hard to change once programs and the reference use it.

### M5-05 · L5 · Decide the layout rules and units

- needs: M5-04
- flags: decision, owner
- done when: Owner approves rules for size, gap, alignment, overflow and text wrap, each with a worked example in numbers.
- why this level: Layout is part of the language's meaning: one tree must give one result, with no toolkit to lean on.

### M5-06 · L5 · Decide UI example functions, the tree text form and hole status

- needs: M5-02, M5-03, M5-04
- flags: decision, owner
- done when: Owner approves the function list, the tree text form and the hole status; a to-do example is written out.
- why this level: Defines what counts as evidence for a GUI: how examples address elements and what a drawn hole means.

### M5-07 · L5 · Decide how migrate and explicit reset are written and applied

- needs: nothing
- flags: decision, owner
- done when: Owner approves a proposal covering detection of changed stored types, migrate, reset, and a migrate that traps.
- why this level: Migrate must name a type that no longer exists in the source; a wrong rule corrupts live state.

### M5-08 · L4 · Run the library trial: one window, one text field, one button

- needs: nothing
- flags: decision, owner, download
- done when: Owner rules on the report: elements from scratch or on an existing toolkit (paper section 8), the stack, the full crate list including PNG encoding, the default font, and yes or no on screen-reader support and on IME in M5.
- why this level: Judging IME, Wayland and screen-reader behaviour from a small trial; a wrong call commits the whole stack.

### M5-09 · L4 · Decide the preview route on Wayland and the Look protocol

- needs: M5-06, M5-08
- flags: decision, owner
- done when: Owner approves one route and the four Look calls: tree, screenshot, click, keystroke, with their inputs and outputs.
- why this level: Wayland allows no embedding, screen grab or injected input; the route must work without them.
- note: Per M4-03: the Look calls are added to its envelope and error shape as new `call` values, with no new version.

### M5-10 · L2 · Vendor the approved GUI crates, pinned and built offline

- needs: M5-08
- flags: download, owner
- done when: cargo build --locked --offline passes; a check fails on any crate off the approved list; README names needed system packages.
- why this level: Routine vendoring; the care is in matching the approved list exactly, licences included.

### M5-11 · L3 · Parse and print view definitions and the app's view field

- needs: M5-01, M5-02
- flags: none
- done when: Tests with hand-written trees pass; printing then parsing the samples changes nothing; errors quote the offending text.
- why this level: Parser and canonical printer must agree on nested child blocks, keys and holes, with precise errors.
- note: Views follow the M4-47 block and flat-list rules; only the trailing-block sample is new.

### M5-12 · L2 · Add the element table as built-in signatures

- needs: M5-04
- flags: none
- done when: A test lists every element, argument, type and needs-key mark, and matches the approved table written by hand.
- why this level: Transcribes an approved table into data; one module, checked item by item.

### M5-13 · L3 · Resolve and type-check views, holes, required keys and app wiring

- needs: M5-11, M5-12
- flags: none
- done when: Each rule has an accepted and a rejected case with a hand-written error; the counter sample checks clean.
- why this level: Several rules across resolver and checker: message types, element-position holes, mandatory keys, all from signatures alone.

### M5-14 · L4 · Carry views through the derived database and the agent tools

- needs: M5-13
- flags: none
- done when: Tests: a view body edit rechecks only that view; a Msg edit, an edit to a view it calls, to the state type and to the app's view field each give exactly the hand-written rechecked and broken sets; M4-14's incremental-equals-rebuild test runs with views in its edit generator and passes.
- why this level: Touches ids, use index, hashes and Find, View, Edit; a missed dependency edge silently skips a recheck.
- note: Per M4-03: views travel through find, view and edit with kind `view` and no new fields.

### M5-15 · L2 · Define the runtime element tree and print it as text

- needs: M5-04, M5-06
- flags: none
- done when: Three hand-built trees print exactly as hand-written text, including a placeholder and an error node.
- why this level: One data type and one printer for an approved text form.

### M5-16 · L3 · Lower views to core form and build trees in the interpreter

- needs: M5-13, M5-15
- flags: none
- done when: The interpreter gives the hand-written tree text for the counter view at counts 0 and 3, placeholder included.
- why this level: Lowering child blocks, loops and holes to core, then turning interpreter values into the runtime tree.

### M5-17 · L4 · Diff trees by key and keep element state with its key

- needs: M5-02, M5-15
- flags: none
- done when: Hand-written cases pass for reorder, insert, remove, kind change, placeholder to element, and duplicate sibling keys.
- why this level: State silently lands on the wrong element after reorder, insert or kind change; duplicates must show in place.

### M5-18 · L3 · Route clicks and keystrokes to messages, with focus

- needs: M5-17
- flags: none
- done when: On hand-built trees, a click on a button yields its message and keys reach only the focused input. Focus stays with its key after a reorder; an event aimed at an element the next diff removed is dropped.
- why this level: Focus must follow keys through diffs; events aimed at a removed element must be dropped, not misdelivered.

### M5-19 · L3 · Build the text input's editing state: text, cursor, selection

- needs: M5-03, M5-17, M5-10
- flags: none
- done when: Hand-written cases pass for insert, delete, move and select, including non-ASCII text, and emit the approved messages.
- why this level: Cursor and selection over multi-byte text are easy to get wrong; must follow the approved input model.
- note: Standard library only. If M5-03 makes grapheme clusters the cursor unit, the table comes from a crate already on M5-08's approved list; otherwise the task gains download and owner.

### M5-20 · L3 · Run a visual app with no window in the interpreter

- needs: M5-16, M5-18, M5-19
- flags: none
- done when: With no display set, a test clicks + and reads hand-written tree text; a trapping view keeps the old tree.
- why this level: Joins state, update, view, diff and input into one loop; a trapping view must leave the tree untouched.

### M5-21 · L3 · Parse and check UI examples and their built-in functions

- needs: M5-06, M5-13
- flags: none
- done when: The to-do example from the approved decision checks clean; each misuse case gives its hand-written error.
- why this level: Extends parser, resolver and checker for the approved functions; wrong element references must fail at check time.

### M5-22 · L3 · Run UI examples with no window and compute their status

- needs: M5-20, M5-21
- flags: none
- done when: With no display set, three counter examples report pass, fail and the decided hole status, as written by hand.
- why this level: Status must come from the toolchain alone; hole status per decision.
- note: No cache: every UI example runs each time. The cache is M7-09.

### M5-23 · L3 · Write the to-do sample and UI examples for both samples

- needs: M5-22
- flags: none
- done when: Both samples check clean; with no display set, their UI examples report the hand-written expected statuses.
- why this level: A new GUI app in a language with no examples; the author must tell their own error from a toolchain defect.

### M5-24 · L4 · Compile views in dev mode and decode their trees in the runtime

- needs: M5-14, M5-16
- flags: unsafe, native-only
- done when: Compiled counter view prints the same hand-written tree text as the interpreter; a trapping view returns trap status; unsafe block reviewed.
- why this level: Unsafe reading of compiled values holding messages; a layout mistake is silent, and Miri is not available.

### M5-25 · L3 · Run a visual app with no window on dev-mode code

- needs: M5-20, M5-24
- flags: native-only
- done when: The windowless counter test from M5-20 passes unchanged with dev-mode code in place of the interpreter.
- why this level: Plugs call slots into the windowless loop; message values must stay valid between view and update.

### M5-25b · L4 · Roll back state and drop effects when a handler traps

- needs: M5-20, M5-25
- flags: unsafe
- done when: In the interpreter and in dev mode, a windowless app whose handler traps shows the old tree, answers the next click, and performed none of that handler's effects.
- why this level: State and effect list must be restored together on every trap path; a miss leaves half-applied state silently.

### M5-26 · L3 · Compare interpreter and dev mode on view trees and UI examples

- needs: M5-22, M5-23, M5-25
- flags: native-only
- done when: One command runs every UI example in both back ends, fails on any tree difference, and catches a planted one.
- why this level: Extends the two-back-end harness to trees and event sequences; it must be shown to catch a planted difference.

### M5-27 · L4 · Re-run the view after a code swap, between frames only

- needs: M5-25
- flags: none
- done when: Windowless test: replacing ??reset with a button shows the button, count unchanged; a click queued before the swap delivers no stale message.
- why this level: Input queued against the old tree carries old-layout messages; dispatching one after a swap corrupts memory silently.

### M5-28 · L4 · Refuse state-type edits that carry no migrate or reset

- needs: M5-07, M5-25
- flags: none
- done when: Tests: editing the state type, or a type held in it through a list, an enum payload, a map, a shared value or a pool, is refused; the old version still answers clicks.
- why this level: Types stored through lists are outside the interface hash; missing one lets old data run under a new layout.
- note: Follows docs/decisions/M2-09.md: the interface hash covers every named type a signature reaches, and no app state may hold a function type.

### M5-29 · L3 · Apply an explicit reset when the state type changes

- needs: M5-28, M5-27
- flags: none
- done when: Test: the refused edit, resent with a reset, installs; state shows its defaults and the tree is rebuilt.
- why this level: Crosses the edit tool, installer and runtime: state and kept element state restart together, between frames.

### M5-30 · L4 · Run migrate at swap time; keep the old version if it traps

- needs: M5-28, M5-27
- flags: none
- done when: Both back ends: adding a field with migrate keeps the count; a trapping migrate leaves the old version running.
- why this level: Old and new layouts of one type live side by side; a failed migrate must leave everything as it was.

### M5-31 · L3 · Lay out the element tree and find the element under a point

- needs: M5-05, M5-15
- flags: none
- done when: Each approved layout rule has a test with hand-computed rectangles, using a fixed-width stand-in for text measuring.
- why this level: Known technique, but sizing, wrapping and rounding rules are easy to get subtly wrong.

### M5-32 · L3 · Measure and shape text through Parley

- needs: M5-10
- flags: none
- done when: Tests: a string measures the same twice, wraps into more lines at half width, and a missing glyph falls back.
- why this level: Unfamiliar library; font loading, fallback and line breaking must give the same sizes to layout and drawing.

### M5-33 · L3 · Draw the laid-out tree to an image through vello_cpu

- needs: M5-19, M5-31, M5-32
- flags: none
- done when: With no display set, the counter draws to a PNG; hand-chosen pixels inside and outside each button have the expected colours.
- why this level: Several element kinds, text runs, placeholder and error boxes; checked by chosen pixels, since golden images are banned.

### M5-34 · L3 · Add the Look tool on the windowless app: tree, screenshot, click, keystroke

- needs: M5-09, M5-25, M5-33
- flags: none
- done when: A script calls Look on windowless apps: the tree matches hand-written text, a PNG returns, click and keystroke change the tree.
- why this level: Wires four calls through the tool channel into a running app without racing its handlers or frames.

### M5-35 · L3 · Open a window and present frames with winit and softbuffer

- needs: M5-10, M5-25, M5-33
- flags: none
- done when: The counter opens a window in the Wayland session, logs its first presented frame, and exits cleanly on a close request.
- why this level: Fits the window event loop to the dev-mode host: redraw on change, resize, scale factor, clean close.
- note: The windowed test skips, and says so, when WAYLAND_DISPLAY is unset; the caller runs it once on the build machine inside the session.

### M5-36 · L3 · Turn window pointer and key events into element input

- needs: M5-18, M5-31, M5-35
- flags: owner
- done when: Translation tests pass; a person confirms by hand that clicks and typing work in the counter and a text input.
- why this level: Click, focus and key translation with scale factor; the winit side cannot be unit-tested here, only hand-checked.

### M5-37 · L4 · Handle IME preedit and commit in the text input

- needs: M5-19, M5-36
- flags: owner, conditional
- done when: Hand-written preedit and commit sequences pass as tests; a person confirms composing with ibus in the window, recorded in the pull request.
- why this level: Preedit text must never leak into app state; end to end it can only be checked by hand.
- note: Only if the owner's M5-08 ruling requires IME in M5.

### M5-38 · L3 · Serve Look from the windowed app

- needs: M5-34, M5-36
- flags: none
- done when: The M5-34 script passes against windowed apps, and a new frame is presented after each change.
- why this level: Calls cross into the window's event loop; the screenshot must be redrawn from the tree, not grabbed.
- note: The windowed test skips, and says so, when WAYLAND_DISPLAY is unset; the caller runs it once on the build machine inside the session.

### M5-39 · L4 · Expose the element tree through AccessKit

- needs: M5-08, M5-17, M5-36
- flags: conditional
- done when: Tests map hand-built trees to AccessKit nodes with tree-wide unique ids, and an AccessKit click action delivers the button's message.
- why this level: Tree-wide ids from sibling keys, incremental updates and focus; the Linux bridge fails silently.
- note: Only if the owner's M5-08 ruling requires screen-reader support.

### M5-40 · L4 · Script an AT-SPI and Orca check

- needs: M5-23, M5-39
- flags: conditional
- done when: One script finds both samples' buttons and input by role and name over AT-SPI and in Orca's debug log; it fails with the bridge disabled.
- why this level: No upstream harness exists; the check must fail when the bridge breaks silently, without being flaky.
- note: Only if the owner's M5-08 ruling requires screen-reader support. The windowed test skips, and says so, when WAYLAND_DISPLAY is unset; the caller runs it once on the build machine inside the session.

### M5-41 · L4 · Add views, elements, keys, UI examples and migrate to the language reference

- needs: M5-05, M5-06, M5-07, M5-21, M5-29, M5-30
- flags: none
- done when: A checklist maps every decision point of M5-01 to M5-07 to a line; every element of the M5-04 table and every UI-example function appears in a sample that parses and checks; M2-18's checklist still holds; the size stays within the budget under M2-18's counting rule.
- why this level: Same work and risk as M2-18: one wrong sentence misleads every model.
- note: If it cannot fit, stop and ask the owner whether the budget moves; never trim M2 text to make room.

### M5-42 · L3 · Script the M5 exit check and run it on the build machine

- needs: M5-23, M5-25b, M5-26, M5-27, M5-29, M5-30, M5-37, M5-38, M5-40, M5-41
- flags: owner
- done when: On the build machine the script exits 0 for every exit-check part, including any clause the M5-08 ruling added; the owner watches the windowed parts once.
- why this level: Drives windowed and windowless apps, edits and Look in one repeatable script; includes a view that traps on purpose.

## M6 — Release mode

**Exit check.** `paka build` compiles a whole program ahead of time through the pinned clang at full optimization, with direct calls instead of slots, and refuses any program with a hole by naming it. The release skip file is deleted and the CI job fails if any conformance test differs in output, exit status or trap between interpreter, dev and release, with two fixed exceptions: a program with a hole must be refused by `paka build`, and example-status cases are compared in interpreter and dev only until M7-11. The owner-agreed benchmark set (float and array loops, integer loops checked and wrapping, pointer-heavy programs) is built in Paka, Rust and C++ with matching checksums, and a committed results table shows each measured number beside its section 3.2 expectation. The same table holds a measured figure for each section 8 cost: overflow and bounds checks, reference counting and handle checks, state copy per handler, dev-mode slowdown on a visual app.

- `paka build` is the release command since M1; there is no second spelling.
- The lowering tasks M6-07 to M6-15 assume dev mode chose Cranelift. If it chose LLVM's JIT they shrink but keep their levels.
- rustc carries LLVM 22 and clang is LLVM 21, so the Rust benchmarks are optimized by a newer LLVM. M6-02 must say how the published table states this.
- A task whose done-when names a conformance group is done only when that group's tag is out of the skip file.
- Can start first: M6-01, M6-02, M6-03, M6-22a.
- The owner is needed in: M6-01, M6-02, M6-23, M6-25, M6-26, M6-32.

### M6-01 · L5 · Decide what release IR may promise the optimizer and what proves it

- needs: M2-06
- flags: decision, owner, native-only
- done when: Owner approves a table: each allowed LLVM flag or attribute, its required proof, and the tail-call rule; what a trap shows and which trap wins are taken from M2-06 as given.
- why this level: One wrong promise silently brings undefined behaviour back; no test can prove the table right.

### M6-02 · L4 · Propose the benchmark set, fairness rules and cost-measurement method

- needs: nothing
- flags: decision, owner, native-only
- done when: Owner approves the written program list, compiler flags per language, run protocol, and how each section 8 cost is isolated, before any measurement. The owner also rules whether measurement-only builds that omit a check family are accepted, and how the published table states the LLVM 22 versus 21 difference.
- why this level: Fair comparison across three compilers must be fixed before numbers exist; bias is invisible afterwards.

### M6-03 · L2 · Pin clang and refuse release builds with any other version

- needs: nothing
- flags: native-only
- done when: A test with a fake clang reporting another version makes `paka build` fail, naming the pinned version.
- why this level: One version check and one error message; the test is obvious.

### M6-04 · L3 · Extend paka build to the whole language: direct calls, no slots, holes refused

- needs: M6-03, M6-02
- flags: native-only
- done when: `paka build` builds M1's light program at full optimization; IR has no slot loads; a program with a hole is refused by name. paka build passes clang exactly the optimization flags and target CPU that M6-02 approves for Paka, from one constant; the benchmark runner (M6-27) adds none.
- why this level: Joins database, emitter, clang and linker; reachability and link order are easy to get wrong.

### M6-05 · L2 · Make the IR linter enforce the approved promise table

- needs: M6-01, M6-04
- flags: native-only
- done when: Hand-written IR carrying each unapproved flag or attribute fails the linter; the linter is an allow-list over every IR keyword token, so anything not listed fails; every release build in the test suite passes through it. M1-18's banned-word test is replaced by this linter: a word on M1-18's list passes only where the approved table allows it.
- why this level: Token scan against a given allow-list; tests follow directly from the table.

### M6-06 · L2 · Add release mode to the differential harness with a skip file

- needs: M6-04
- flags: native-only
- done when: Each conformance test runs in interpreter, dev and release; output, exit status and the trap fields M2-06 makes part of meaning (kind and source position, the same fields as M4-40) are compared; unsupported tests sit in one skip file. A case whose program has a hole has the release expectation: paka build refuses it and names the hole. Example-status cases are compared in interpreter and dev only, until M7-11. Both sets are a fixed hand-written exemption list, separate from the skip file. Every M2 conformance case carries one tag for each feature its program uses, from a fixed hand-written list (integers, control flow, calls, floats, types, enums, match, parameter passing, strings, lists, maps, shared data, pools, generics, function values, error handling, effect values, apps and grants); the skip file lists tags, never single cases, and a case is skipped while any of its tags is listed.
- why this level: Adds one more mode to an existing two-mode harness; the spec is clear.

### M6-07 · L3 · Lower integers, booleans, control flow and calls with trap status returns

- needs: M6-05, M6-06
- flags: native-only
- done when: Conformance groups for integers, control flow and calls leave the skip file and match the interpreter, including overflow and assertion traps.
- why this level: Known technique, but every call and checked operation needs a correct trap exit.

### M6-08 · L4 · Lower Float operations so full optimization cannot change any result

- needs: M6-01, M6-07
- flags: native-only
- done when: Float conformance tests are bit-identical in three modes; the linter finds no fast-math flag and no maths call LLVM can fold. M2-08's float vectors, every NaN row included, and M2-08b's maths vectors if they exist, pass bit for bit in release; maths functions are called through the same private runtime names as M4-29.
- why this level: Optimizer folding or fused operations change results silently; only bit-exact comparison shows it.

### M6-09 · L3 · Lower struct and enum values, match, inout and borrowed parameters

- needs: M6-07, M4-22, M4-33
- flags: native-only
- done when: Conformance groups for types, enums, match and parameter passing leave the skip file and match the interpreter.
- why this level: Layouts and passing conventions across several modules; known technique with easy-to-miss cases.

### M6-10 · L4 · Share runtime header layouts with the compiler and verify both sides agree

- needs: M6-04
- flags: unsafe, native-only
- done when: The layout table dev mode already reads (M4-22, no second table) gains the runtime header layouts for list, string, shared box and pool slot and the slow-path entry points; a test fails when compiler and runtime disagree.
- why this level: A layout mismatch between emitted IR and unsafe runtime code corrupts memory silently.

### M6-11 · L4 · Lower strings and lists: inline bounds checks, runtime slow paths, drops

- needs: M6-09, M6-10, M4-22, M4-33
- flags: unsafe, native-only
- done when: String and list conformance groups match the interpreter, including bounds traps; an injected allocation failure ends in an out-of-memory trap, not an abort (not compared with the interpreter); M4-25's ledger is clean at exit.
- why this level: A wrong drop or bounds check is a silent leak, double free or bad read.

### M6-11b · L4 · Lower the built-in map for release

- needs: M6-10, M6-11, M6-14, M6-15
- flags: unsafe, native-only
- done when: M2's collection conformance cases agree in all three modes.
- why this level: Same unsafe container as dev mode with inline fast paths; a layout or order mismatch is silent.

### M6-14 · L4 · Specialize generic definitions per concrete type in release builds

- needs: M6-11
- flags: native-only
- done when: Generics conformance group matches the interpreter; optimized IR for a generic loop over List<Float> contains no indirect call and no boxing.
- why this level: Whole-program pass tied to the generics design; a wrong instance choice miscompiles silently.

### M6-15 · L3 · Lower the remaining constructs: function values, error handling, effect values

- needs: M6-11
- flags: native-only
- done when: Their conformance groups leave the skip file and match the interpreter.
- why this level: Several small lowerings with known techniques; their forms were fixed in milestone 2.
- note: Becomes level 4 if M2-09 allows a function value to capture (as M4-37b).

### M6-16 · L4 · Emit explicit stack-limit checks that hold under inlining and large frames

- needs: M6-01, M6-07
- flags: unsafe, native-only
- done when: Deep-recursion and large-frame tests end in a stack trap, never a segmentation fault, at full optimization; no signal handler is installed.
- why this level: A missed check is a crash outside the language; frame sizes are unknown when IR is written.

### M6-17 · L3 · Link headless apps for release: direct handler calls, grants

- needs: M6-15
- flags: unsafe, native-only
- done when: The paper's wc sample runs as a release executable; the apps and grants conformance group leaves the skip file and matches the interpreter, grant-refusal cases included; a trapping handler ends the program as M2-15 rules, with none of its effects performed; the binary holds no installer symbols.
- why this level: Reuses the dev runtime without slots; entry point, effect loop and link line need care.
- note: Stays level 3 only while its unsafe is export attributes; any pointer work makes it level 4.

### M6-18 · L3 · Link the GUI runtime so visual apps run windowless in release

- needs: M6-17, M6-18a, M5-25b
- flags: unsafe, native-only
- done when: With no display set, a release build of the hole-free counter from M6-18a passes a windowless test that clicks + and reads hand-written tree text; M5-25b's handler-trap test passes in release: old tree, next click answered, none of that handler's effects performed; the same build opens a window. UI examples are not run in release before M7-11.
- why this level: Many system libraries on the link line and view code compiled without slots; known technique.
- note: Stays level 3 only while its unsafe is export attributes; any pointer work makes it level 4. The windowed test skips, and says so, when WAYLAND_DISPLAY is unset; the caller runs it once on the build machine inside the session.

### M6-18a · L4 · Lower views and element trees for release

- needs: M6-10, M6-15
- flags: unsafe, native-only
- done when: The to-do view, and the counter view with ??reset replaced by the button M5-27's test uses (the hole-free counter, kept as a test fixture), give the same tree text in release as in the interpreter; the hole-free counter's tree text at counts 0 and 3 is written by hand first; a trapping view returns trap status; a test fails when compiler and runtime disagree on the tree layout.
- why this level: The runtime decodes trees with unsafe code; a layout mismatch is silent.
- note: The paper's counter keeps ??reset and stays refused by `paka build` (M6-04). Release never lowers a placeholder.

### M6-19 · L4 · Shape loop IR so bounds and overflow checks are proven away

- needs: M6-01, M6-11
- flags: native-only
- done when: For a hand-written loop list, optimized IR has no trap branch inside the loop; each off-by-one twin still traps in release.
- why this level: Removal must rest on a proof; a wrong fact removes a needed check with no symptom.

### M6-20 · L2 · Add release mode to the M4 fuzz loop

- needs: M6-06, M4-41, M6-07
- flags: native-only
- done when: A deliberately divergent release build is detected, shrunk to under 20 lines and saved as a regression test, reproducibly from a seed.
- why this level: Adds a third back end to an existing loop and shrinker.

### M6-21 · L4 · Extend the program generator to reach release-only risks

- needs: M6-08, M6-11b, M6-14, M6-15, M6-16, M6-19, M6-20, M4-21
- flags: native-only
- done when: With a fixed seed the fuzzer finds each of ten hand-planted release back-end bugs within ten minutes; the bug list is in the pull request. The ten bugs include at least one each of: a folded float operation, a wrongly removed bounds check, a wrongly removed overflow check, a missed stack check after inlining, a wrong reference-count update, a wrong stale-handle check, a wrong generic instance. The generator's coverage report shows every core-form node kind compiled by release, none rejected, except that a generated program with a hole must be refused by `paka build`.
- why this level: Programs must pass ownership and exclusivity checks yet still reach rare traps and optimizer paths.
- note: No reference counts or handles exist (docs/decisions/M2-10.md): the two planted bugs of those kinds are replaced or the count lowered by the owner.

### M6-22 · L4 · Build the hard test suite for the inout overlap check

- needs: M6-21, M6-22a
- flags: native-only
- done when: Hand-written accept and refuse cases pass; the interpreter's run-time overlap detector finds no overlap in accepted calls across one million generated programs. The report counts accepted calls that pass an inout argument together with another argument drawn from the same value (field, list item, slice, pool node), at least 100,000 in total (the owner may change the number); with one overlap rule of the checker switched off in a test build, the same run flags an overlap.
- why this level: Must find holes in a rule that later becomes an optimizer promise; misses are silent.
- note: Follows docs/decisions/M2-05.md, including its two tightened call rules.

### M6-22a · L4 · Add a run-time inout overlap detector to the interpreter

- needs: nothing
- flags: none
- done when: Hand-built core programs with overlapping inout arguments are flagged at the call; non-overlapping ones are not; normal runs are unchanged.
- why this level: A missed path silently weakens the evidence for the irreversible no-overlap promise.

### M6-23 · L5 · Decide whether release IR may promise no overlap for inout

- needs: M6-01, M6-22
- flags: decision, owner, native-only
- done when: Owner signs a written decision citing the suite's results, the exact attribute placement and the plan if a hole appears later.
- why this level: Turns a checker rule into undefined behaviour if wrong; a trust decision only the owner can take.
- note: Process: one evidence report and one reviewer told to break it, then the owner; not three proposals. If the owner declines, M6-24 is dropped.

### M6-24 · L4 · Emit the no-overlap promise on inout parameters as signed

- needs: M6-05, M6-19, M6-23
- flags: native-only, conditional
- done when: The linter allows the attribute only where signed; a two-array kernel vectorizes without a run-time overlap check; conformance stays identical in three modes.
- why this level: LLVM's noalias rules are subtle; one misplaced attribute miscompiles only under optimization.
- note: Dropped if the owner declines in M6-23.

### M6-25 · L2 · Run the 24-hour three-back-end fuzz and file each divergence

- needs: M6-08, M6-16, M6-18, M6-21, M6-24, M6-11b, M6-19
- flags: owner, native-only
- done when: The skip file is empty when the run starts. A 24-hour, 24-core run from recorded seeds finishes; every divergence is saved, shrunk, and in the known-failing list.
- why this level: Starts a given command and files its output; no design.
- note: The caller's script starts the run; no agent waits 24 hours.

### M6-25b · L4 · Fix the filed fuzz divergences until a rerun is clean

- needs: M6-25
- flags: native-only, open-ended
- done when: Every filed regression test passes; rerunning M6-25 files nothing.
- why this level: Each divergence is a miscompile or meaning bug across three back ends.

### M6-26 · L1 · Delete the release skip file and require three-mode conformance in CI

- needs: M6-25b
- flags: native-only, owner
- done when: The skip file, already empty, is gone; a planted pull request with one differing conformance case shows the three-mode job red; the owner sees it and marks the job as required.
- why this level: Deletes a file and adds given commands to CI; no decisions.

### M6-27 · L2 · Build the benchmark runner: pinned compilers, fixed flags, repeated runs, checksum agreement

- needs: M6-02, M6-04
- flags: native-only
- done when: One command builds a sample in Paka, Rust and C++, refuses mismatched checksums, and writes medians and spread to a results file.
- why this level: The approved protocol fixes every step; one module that runs commands and records times.

### M6-28 · L3 · Write the numeric benchmark kernels in Paka, Rust and C++

- needs: M6-02, M6-08, M6-11, M6-27
- flags: native-only
- done when: Every agreed float, array and integer kernel, checked and wrapping, exists in three languages and the runner reports matching checksums.
- why this level: Same algorithm in three languages under the fairness rules; small differences skew results.

### M6-29 · L3 · Write the pointer-heavy benchmarks in Paka, Rust and C++

- needs: M6-02, M6-11b, M6-14, M6-15, M6-27
- flags: native-only
- done when: Every agreed parser and graph program exists in three languages and the runner reports matching checksums.
- why this level: Larger programs whose data structures must stay comparable across three ownership models.

### M6-30 · L3 · Add measurement-only builds that each omit one check family

- needs: M6-02, M6-19
- flags: native-only
- done when: Only a compiler built with the `measure` feature accepts the switch; the normal compiler rejects it; such binaries print a warning banner. For each family named in M6-02 (bounds checks, reference-count updates, handle checks) a hand-written fixture built with the switch has none of that family's checks in its optimized IR; without the switch, IR for the conformance suite is byte-identical to the normal compiler's.
- why this level: Threads a switch through several lowerings without changing normal output.
- note: Measures bounds checks and copies in place of reference-count updates and handle checks (docs/decisions/M2-10.md).

### M6-31 · L3 · Measure handler state-copy cost and dev-mode slowdown on a visual app

- needs: M6-02, M6-18, M6-27
- flags: native-only
- done when: The runner reports copy time per handler for the agreed state sizes, and dev versus release frame times for the agreed visual app.
- why this level: Timing inside the runtime and windowless app runs; measurement overhead must stay out of results.

### M6-32 · L2 · Run the agreed set and publish numbers beside the paper's expectations

- needs: M6-24, M6-26, M6-28, M6-29, M6-30, M6-31
- flags: owner, native-only
- done when: Raw results are committed; a generated table shows each figure beside its section 3.2 expectation and each section 8 cost; owner approves.
- why this level: Runs a given command and fills a given table; the owner checks the wording.

## M7 — Evidence and gates

**Exit check.** The example listing labels every example agent-written or human-owned, taken from the approval store and never from the agent. The store holds the approved version of each human-owned example and each app's grant list; the gate (a required CI check) and the release command both refuse when one differs. A rerun with no edit executes zero examples, and editing a dependency reruns only the examples that depend on it. The release command runs the human-owned examples against the very binary it ships, proven by file hash. A test C library can be called only under its own named grant and runs in a separate process in interpreter, dev and release. A test fails when a library Paka uses is missing from the trusted list, and every release contains the generated third-party notices file. An edit that changes a grant list or a human-owned example returns the pending item in the agreed shape, and the owner approves it with the approve command.

- A separate process is not a sandbox: a C library keeps the user's operating-system rights.
- M7-03 names the command that ships. Until then `paka build` is not gated by approvals.
- If approvals are stored in the repository, a pull request could change them together with the code; M7-01 must close that hole.
- Can start first: M7-01, M7-02, M7-09.
- The owner is needed in: M7-01, M7-02, M7-03, M7-06, M7-10, M7-11, M7-16.

### M7-01 · L5 · Decide how the owner writes and approves examples and grant changes

- needs: nothing
- flags: decision, owner
- done when: Owner approves a written workflow: where labels and approvals live, what makes an approval unforgeable by an agent, what a refused agent sees, and exactly what the owner is shown for an example change and for a grant change.
- why this level: The evidence pillar rests on it; an agent able to approve itself breaks it silently.

### M7-02 · L5 · Decide the foreign-code model: declaration, named grant, process boundary, trusted list

- needs: nothing
- flags: decision, owner
- done when: Owner approves a written model with alternatives: how a C library is declared and called, what crosses the process boundary, how failures surface; that a foreign call is an effect answered by a message; which crate (or generated C built by the pinned clang) holds the host's unchecked code; and where the trusted list is kept.
- why this level: Security model for code outside every Paka guarantee; tests cannot repair a wrong boundary.

### M7-03 · L4 · Propose how human-owned examples run against the shipped release binary

- needs: M7-01
- flags: decision, owner, native-only
- done when: Owner approves a written mechanism covering function examples and UI examples, how the tested binary is shown to be the shipped one, and the command that ships (a new paka release, or paka build under a stated rule) so that conformance, fuzz and benchmark builds are not gated.
- why this level: If the tested and shipped binaries can differ, the release evidence is empty.

### M7-04 · L4 · Build the approval store: record and verify approved examples and grant lists

- needs: M7-01
- flags: none
- done when: Tests show a recorded approval verifies, and that an edited example, an edited grant list and a tampered record are each detected.
- why this level: A forged or stale approval accepted here goes unnoticed; this is the trust anchor.
- note: Gains download and owner if M7-01's ruling needs a cryptography crate; no hand-written cryptography.

### M7-05 · L2 · Label every example agent-written or human-owned from the approval store

- needs: M7-04
- flags: none
- done when: The example listing and every example result carry the label; a test shows an unapproved example never reads as human-owned.
- why this level: One lookup per example and one output field; tests are obvious.

### M7-06 · L4 · Build the owner's approve command: show pending changes plainly, record approval

- needs: M7-04, M7-05
- flags: owner
- done when: For a hand-written set of example and grant changes, including two that differ only in one expected value, the command prints the hand-written expected text; the owner completes one approval of each in a walk-through.
- why this level: Must describe code changes truthfully to a non-reader; a misleading summary gets a wrong approval with no error anywhere.

### M7-07 · L4 · Build the gate: refuse when approvals differ or a human-owned example fails

- needs: M7-04, M7-05
- flags: none
- done when: The gate exits non-zero for a changed, deleted or renamed human-owned example, a changed grant list, a new app whose grant list was never approved, a missing or tampered store record, or a failing or unfinished human-owned example; each case is tested.
- why this level: Unlisted cases (a deleted example, a new app with grants) pass silently; this gate stands in for code review.

### M7-08 · L2 · Report pending approvals and refusals to the agent in the agreed shape

- needs: M7-07
- flags: none
- done when: Edit results and gate output list each item awaiting approval in the agreed shape; expected outputs are written by hand.
- why this level: Formats known data into a given shape.
- note: Per M4-03: items awaiting approval ride in edit replies as a new optional field; they are not errors, and M7-01 fixes their shape.

### M7-09 · L4 · Cache example results by closure hash, mode and compiler version

- needs: M4-11, M5-22
- flags: none
- done when: A rerun with no edit executes zero examples; editing a transitive dependency reruns exactly its dependent examples; tests assert both counts. The closure hash is added here, on top of M4-11's body and interface hashes. A cached result holds no label and no approval state; the label is read from the store each time.
- why this level: A stale hit is false evidence; the key must cover everything an example depends on.

### M7-10 · L1 · Add the gate to CI as a required check on pull requests

- needs: M7-07
- flags: owner
- done when: A test pull request changing a grant list without approval shows a failed check; the owner sees it go red and marks the check as required. A second test pull request that changes a grant list and its stored approval together also shows a failed check.
- why this level: Adds a given command to CI; the required-check switch is the owner's.

### M7-11 · L3 · Run human-owned examples against the release binary itself

- needs: M7-03, M7-06, M7-07, M6-18, M6-26, M5-26
- flags: native-only, owner
- done when: The owner has approved, through M7-06, at least one function example for each sample (wc, counter, to-do) and one UI example for each visual sample (counter, to-do); the counter is the hole-free copy from M6-18a. Every human-owned sample example, function and UI, passes against a release binary whose file hash equals the artefact's; M5-26's command, with release added, reports no tree difference on every UI example whose program has no hole; a planted failure is reported.
- why this level: Drives a compiled binary from outside across runtime, release build and example runner.
- note: Re-grade once M7-03 is approved: if examples are compiled into the binary, lowering them for release is split out as its own task.

### M7-12 · L2 · Make the release command refuse unless gate and binary examples pass

- needs: M7-07, M7-11
- flags: native-only
- done when: Release leaves an artefact only after the gate and the binary examples pass; tests show a refusal and no artefact for each failing case. M1's light check and M6's three-mode conformance still pass unchanged; a test shows the artefact's file hash equals the hash of the binary the examples ran against, and a rebuild planted between the two is refused.
- why this level: Sequences existing steps and checks their exit codes.

### M7-13 · L3 · Add foreign library declarations and their named grants to the checker

- needs: M7-02, M7-07
- flags: none
- done when: A program using a declared C library builds only with that library's named grant; adding the grant needs approval at the gate; reference updated. The declaration prints in the canonical layout and reparses unchanged; Edit, Find and View accept it; editing it rechecks its users.
- why this level: New declaration through parser, resolver, checker and grants; several modules with clear rules.
- note: Updates the language reference in the same pull request, within M2-18's size rule.

### M7-14 · L4 · Build the foreign host process with generated glue and a pipe protocol

- needs: M7-02
- flags: unsafe
- done when: A test C library is called through the host in a separate process; malformed requests and a crashing library never take down the caller.
- why this level: Marshalling into C and crash containment; a mistake corrupts data without any trap.

### M7-15 · L3 · Perform foreign calls as effects: start host, exchange messages, report failures

- needs: M7-13, M7-14, M6-17
- flags: none
- done when: A conformance test calls the test C library identically in three modes; without the grant the call is refused; host death becomes a message.
- why this level: Process lifetime, timeouts and grant enforcement in the shared runtime; known technique.
- note: Stays level 3 only while arguments and results pass through M4-36's existing decoder and message encoder unchanged; any new unsafe makes it level 4 with the unsafe flag.

### M7-16 · L2 · Name the trusted libraries and ship a generated third-party notices file

- needs: M7-02, M7-12
- flags: owner
- done when: A test fails when a vendored or linked library is missing from the trusted list; releases include a generated notices file; owner approves the list.
- why this level: Collects names and licence texts from the vendor directory in a fixed order.
