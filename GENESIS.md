# Paka

**A programming language for code written by AI agents.**

Genesis paper · draft 1 · 15 October 2026 · Status: prototype built, first experiment run, rule says stop

---

## Summary

Paka is a compiled language whose code is written and read by AI agents. People do not read Paka code. They describe what they want, decide what the program may do, and, for a visual app, watch it take shape on screen.

Paka stands on six pillars:

1. **Instant build.** In dev mode an edit to one definition's body is running in under 100 ms (target).
2. **Native speed.** In release mode Paka aims to match Rust and C++ on floating-point and array code, and expects to run somewhat behind on checked integer arithmetic and pointer-heavy code.
3. **Live GUI.** GUI is part of the language. A visual app appears on screen with its first piece and grows as the agent writes.
4. **Local reasoning.** Any definition can be type-checked from itself and the signatures it uses.
5. **One meaning, granted permissions.** There is no undefined behaviour: the same program with the same inputs behaves the same in dev and release. A program can touch only what a person granted it.
6. **Evidence over review.** Nobody reads the code, so trust comes from examples that people own and the toolchain runs.

The first three are what Paka must deliver. The last three make them possible and safe to rely on.

Built so far: the language reference, a parser, a type checker and a slow interpreter, plus native builds for a small subset. Dev mode, the live GUI and release mode are not built. Every other number about Paka is a target or an estimate until it is measured. Figures cited from other work are the sources' own.

**First result.** One model wrote the same tasks in Paka, Rust and TypeScript. It wrote Paka almost as correctly as Rust but spent 2.3 times the output tokens, past the pre-set limit of 2.0, so the rule says stop ([section 9](#9-the-first-experiment)). The owner's decision is still open.

## 1. Why a new language

Mainstream languages were shaped by human limits. Operator overloading, macros, implicit conversions and inferred types exist to save people typing and reading. An agent has different limits:

- **It sees little at once.** It works through a bounded context window and cannot hold a large program in view.
- **It is confidently wrong.** It finds a mistake only when something reports it, so the speed and precision of feedback set its pace.
- **It works in parallel.** Several agents may edit one program at the same time.
- **It knows what it was trained on.** No Paka code exists to learn from. This is the project's largest risk, and section 9 tests it first.

The person's role changes too. They stop reading code but still own the result. Code review is one of the main checks on software today, and it goes away. Something has to replace it.

## 2. What "not for human eyes" means

It does not mean a binary format or a graph database. Models learned from text code. A new encoding has no training data and gives up git, grep and diff. A syntax tree written out in full also costs more tokens than the text it stands for: about 4 to 5.6 times, computed from the token counts in [one Python study](https://arxiv.org/html/2602.06671v1). So Paka source is plain text with a familiar shape, close to Swift and Kotlin, in ordinary files under git.

What goes is everything that exists only for human convenience and makes meaning depend on code out of sight:

- operator and function overloading
- macros and compile-time code execution
- implicit conversions
- inheritance
- type inference across definitions
- global mutable state
- formatting freedom: there is one layout and one spelling for each construct

What is added is facts a machine can check: full signatures, declared permissions, examples.

Names and comments stay. The reader is a language model, and it leans on them.

## 3. The six pillars

Three terms are used throughout:

- A **definition** is one named unit of code: a function, a type, a view, an app or an example.
- A **handler** is one run of an app's `update` function for one message.
- A **trap** is a defined stop: integer overflow, out-of-bounds access, a failed assertion, a stale handle, or running out of stack or memory.

### 3.1 Instant build

**Promise.** In dev mode an edit to one definition's body is checked, compiled and running in under 100 ms. An edit to a signature or a type costs in proportion to its users.

**How.**

- The unit of compilation is one definition, not a file or a package.
- Checking a definition needs only its own body and the signatures of what it uses (pillar 4).
- Dev mode emits quick, unoptimized machine code.
- There is no link step. For a visual app, new code is swapped into the running program. A headless program is rebuilt and rerun.
- Unfinished parts are *holes*. `??name` takes the type its position expects. A hole where a UI element belongs draws a labelled placeholder. A hole reached anywhere else traps. A half-built program therefore still runs.

**Cost.** Dev-mode code is slow on heavy computation. Published figures put unoptimized code at about 2–3× slower than optimized code for C and WebAssembly, and commonly 10–100× slower for Rust dev builds (section 10). Where Paka lands is unmeasured. The slowness is accepted: dev mode is for building and watching, release mode is for speed. If Paka lands near the Rust end, a visual app that computes heavily will feel slow while it is being built.

**Why 100 ms and not 1 ms.** An agent's loop is dominated by the time the model takes to write. Below about 100 ms, a faster compiler changes nothing anyone can see.

**Depends on.** Generics that can be checked from signatures alone. That design is open (section 8).

### 3.2 Native speed

**Promise.** A release build is meant to match Rust on floating-point and array loops, and C++ wherever bounds checks are proven away. It is expected to run somewhat behind on integer loops that keep overflow checks and on pointer-heavy code.

**How.**

- Release mode compiles the whole program at once, ahead of time, with a full optimizing backend. There is no tracing garbage collector and no interpreter.
- Every value has one owner. A call may not pass the same data as `inout` and through another argument, and this is checked at the call site. The compiler therefore knows that `inout` data overlaps nothing else, the same fact Rust gives its optimizer, so loops need no run-time overlap checks. The check is conservative: it refuses some calls that would be safe.
- A function can borrow a caller's value, or part of it such as an array slice, for the length of the call. The borrow cannot be stored or returned, so nothing is copied and no lifetime is written down.

**Where it holds.** These are estimates for single-threaded code. None is measured.

| Code | Expectation in release mode |
|---|---|
| Floating-point and array loops | Parity with Rust; parity with C++ wherever bounds checks are proven away |
| Integer loops | Parity when wrapping arithmetic is chosen; behind with overflow checks on, by an amount to be measured |
| Pointer-heavy code (parsers, graphs) | Behind, by an estimated 10–30% |
| Hand-written SIMD, lock-free structures, zero-copy tricks on unsafe code | Paka loses |

**Why accept the gap.** Matching Rust everywhere would need stored references with lifetimes and an unsafe escape hatch. Lifetimes take reasoning across many functions at once, and unsafe mistakes are silent. Both are a poor fit for a writer that sees little at once and is confidently wrong.

**Checks.**

- Integer overflow traps in both modes. A loop can choose wrapping arithmetic instead, which is still defined, and the choice is visible in the source.
- Out-of-bounds access traps in both modes. The check cannot be switched off; the compiler removes the ones it can prove safe.
- What these checks cost in Paka is unmeasured. In other compilers, published costs run from under 1% to about 12% on average, and one loop ran 7× slower where a check kept the compiler from using SIMD instructions (section 10).

**Cost.** A release build takes minutes.

### 3.3 Live GUI

**Promise.** A visual app appears as soon as its first piece exists and grows piece by piece as the agent writes, without restarting. State is kept unless an edit explicitly resets it. A piece is one definition.

**How.**

- `view` and `app` are kinds of definition, not a library. A view is a function from state to a tree of built-in elements (about 20: text, button, input, row, column, list, canvas and so on). An app names a state type, an update function and a view.
- A visual app opens in its own window or in a preview pane. There is no other hosting concept.
- The runtime holds the state, not the compiled code, so swapping code leaves state alone.
- The agent writes the skeleton first, with holes. Each finished piece replaces its placeholder.
- If the state type, or any type stored in it, changes, the edit must carry a `migrate` function or an explicit reset. Otherwise it is rejected and the last good version keeps running.
- An element that holds its own state, such as a text input or a scroll position, must carry a key so the state follows the right element. The checker enforces that the key is present. Duplicate keys among siblings are a runtime error, shown in place of the elements.
- One tree describes the UI. The same tree is drawn, used by tests, and read by the agent to see what it built. It can also feed accessibility tools.

An app with no view is headless. There is nothing to watch: each edit is a fast rebuild and rerun.

**Limits.**

- Updates land per piece, not per keystroke, because half-typed code cannot run.
- Paka defines the elements and the layout rules. Whether they are built from scratch or on an existing toolkit is open (section 8). It draws through an existing 2D library and takes text shaping, line layout and font fallback from existing libraries. It does not write its own renderer.
- Linux comes first. Other platforms are not planned yet.

### 3.4 Local reasoning

**Promise.** Whether a definition is type-correct can be decided from that definition and the signatures of what it uses. For a type, the signature is the whole definition.

**How.**

- Every signature is written out in full. Types are inferred only inside a body.
- Most of the features removed in section 2 are ones that would break this.

**What it buys.**

- **Fast rebuild.** A body edit rechecks one definition. A signature edit rechecks its direct users, and the toolchain lists exactly which ones broke.
- **Small context.** An agent reads one body and a handful of signatures, not whole files.
- **Parallel work.** Two agents conflict only when they change the same definition, add the same name, or when one changes or removes something the other relied on.

**Limits.**

- This is a promise about types, not behaviour. A function can keep its signature and change what it does. Only examples catch that (pillar 6).
- It holds only if generics can be checked from signatures alone (section 8).

### 3.5 One meaning, granted permissions

**One meaning.** Paka has no undefined behaviour.

- Evaluation order is fixed. Floating-point maths follows IEEE strictly, and both modes use the same maths library.
- A trap ends the current handler. The runtime restores the state from before the handler and drops the effects it asked for, so an app is never left half-updated. The cost is a copy of the parts of the state the handler writes.
- A trap in a view keeps the last frame that drew. A trap in a headless program ends it with an error.
- A program that builds in both modes behaves the same in both, given the same inputs. The modes differ in speed and in where they run out of stack or memory. Holes exist in dev mode only.

**Granted permissions.** Code has no authority of its own.

- Functions have no hidden effects: their result and their `inout` parameters depend only on their arguments. They cannot touch files, network, clock or screen.
- A handler returns the effects it wants as data. The runtime performs them after the handler finishes, and only those the app's `grants` allow.
- The grant list is owned by a person. The toolchain records the approved version and refuses to merge or release when the list differs from it.

**What it buys.** A wrong edit, or one steered by hostile input, has bounded reach.

**Limits.**

- Grants bound the damage. They do not show the code is right.
- Foreign code such as C libraries is outside these guarantees. It needs its own named grant and by default runs in a separate process. The guarantees are claimed for Paka code only. The libraries Paka itself uses for drawing and text are foreign code too, and are named as trusted.
- Step-by-step input and output is clumsy when written as messages and effects. A friendlier notation over the same model is open (section 8).

### 3.6 Evidence over review

**Promise.** A person who never reads the code can still know what was checked.

**How.**

- `example` is a kind of definition: a small checked case for a function, or a sequence of actions on the UI tree with the expected result.
- People own two things: acceptance examples and permission grants. The toolchain records the approved version of each and refuses to merge or release when one differs.
- The toolchain computes the status of every example. An agent never reports its own results. An example that reaches a hole is reported as unfinished.
- Results are cached by a hash of everything the example depends on, so unchanged code is not retested.
- Evidence gates merging and release, not the live preview. Before release, the human-owned examples run against the release binary itself.

**Limits.**

- Examples an agent writes for its own code tend to share its mistakes. They are labelled as agent-written. Only human-owned examples count as independent evidence.
- How a person writes examples and approves changes in practice is open (section 8).

## 4. The language at a glance

The syntax below is illustrative and not final.

A visual app:

```
type Counter { count: Int = 0 }

enum Msg { Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
}

example dec_stops_at_zero {
  var s = Counter {}
  update(&s, Dec)
  s.count == 0
}

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text("{s.count}")
    Row(gap: 8) {
      Button("-", on: Dec)
      Button("+", on: Inc)
    }
    ??reset
  }
}

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}
```

Running `counter` in dev mode opens a window or preview pane with the number, two buttons and a placeholder box labelled `reset`. When the agent replaces `??reset` with a real button, the box becomes the button and the count on screen stays where it was.

A headless program that counts the words in a file:

```
type Tally { words: Int = 0 }

enum Msg { Start(path: String), Loaded(text: String) }

fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path)  => [ReadFile(path, then: Loaded)]
    Loaded(text) => {
      s.words = count_words(text)
      [Print("{s.words} words"), Exit]
    }
  }
}

app wc {
  state:  Tally
  update: update
  grants: [ReadFiles, Console]
}
```

The runtime sends `Start` with the command-line argument. `update` asks for the file, and the runtime answers with `Loaded`. `count_words` is an ordinary function, not shown. An `update` that needs nothing from outside, like the counter's, returns nothing.

**Definition kinds.** `type`, `enum`, `fn`, `view`, `app`, `example`.

**Names.** Within one program there are no imports and no build scripts. How definitions are grouped, and how programs depend on each other, is open (section 8).

**Memory.** Values are owned. A read-only parameter is passed without copying. Borrows cannot be stored or returned, so there are no lifetimes to annotate. Shared data is immutable, cannot form cycles, and is reference-counted. Graph-shaped data is kept in a pool, and its nodes refer to each other by handle. Using a stale handle traps. Handle checks are the main cause of the pointer-heavy gap in 3.2.

## 5. The compiler

**Source of truth.** Canonical text in ordinary files under git.

**Derived database.** Built from the text and safe to delete:

- a stable id per definition
- an exact index of who uses what
- caches of check results, compiled code and example results
- three hashes per definition, each used only where it is sound:

| Hash | Covers | Decides |
|---|---|---|
| Body | The definition's own text and what its names resolve to | Whether it must be rechecked and recompiled |
| Interface | Its signature, plus everything a user's compiled code is built from: the layout of types held by value (transitively), enum variants and default values | Whether its users must be rechecked and recompiled |
| Closure | Everything it depends on, transitively | Whether example results, and release code that inlined it, are still valid |

Every cache key also includes the compiler version, the target and the mode.

Editing a widely used type fans out to everything that uses it. That cost is real and accepted.

**Two modes, one meaning.**

| | Dev mode | Release mode |
|---|---|---|
| Unit | One definition | Whole program |
| Build time | Under 100 ms per body edit (target) | Minutes |
| Code | Quick, unoptimized | Fully optimized |
| Speed | Slow on heavy compute (see 3.1) | See 3.2 |
| Used for | Writing, watching, testing | Shipping, measuring |

**Dev-mode path for one edit.** Parse the definition, resolve names, check it against the signatures it uses, emit quick, unoptimized machine code, install it.

- Each definition is called through a slot, and installing new code swaps the slot.
- Swaps happen between handlers and between frames, never inside one. A long-running handler therefore delays the swap.
- The running program is always the last version in which every definition checks. An edit that breaks its users is stored, but nothing is installed until those users check too. Then all are swapped together.

**Release-mode path.** Compile the whole program with a mature optimizing backend. There are no slots and no holes: a program with a hole does not build in release mode.

## 6. How an agent works with it

The agent works through a few tool calls. The tools write the source files; the agent does not edit them directly.

- **Find** definitions by name, type shape, or who uses them.
- **View** a slice sized to a token budget: one definition's body plus the signatures of what it uses.
- **Edit** by replacing a whole definition. The edit names the version it was based on and is refused if that version has moved.
- **Look** at a running app: read the UI tree, take a screenshot, send a click or a keystroke.

Every edit returns:

- the stored text, in canonical form
- errors with the offending text, the expected and found types, and a suggested fix where one exists
- the exact list of definitions the edit broke

Target: the full language reference fits in 8,000 tokens, so it can travel with every task.

## 7. Non-goals and rejected ideas

**Non-goals.**

- Syntax that is pleasant for people to write.
- Beating Rust or C++.
- Kernels, drivers and hand-tuned SIMD.
- A renderer of its own.
- Platforms other than Linux, for now.

**Rejected.**

| Idea | Why not |
|---|---|
| Binary or graph format as the source of truth | See section 2 |
| A background optimizer for the live program | Slow dev-mode code is accepted (3.1), so it is not needed yet |
| Stored references with lifetimes; an unsafe escape hatch | See 3.2 |
| Permissions written on every function signature | Adding one file read would force an edit to every caller up the chain. Languages that avoid those edits infer effects across definitions, which pillar 4 rules out. With effects returned as data at the edge of the app, signatures need no permissions |
| Automatic proofs as a gate | An automatic prover can change its answer under edits that should not matter: 2.6% of queries were unstable in one study (section 10). The gate would fail at random |
| Banning foreign code | A new language has no libraries; foreign code is quarantined instead |
| A hosting concept for apps beyond a window | Not needed yet |
| Hard limits on definition size | They fragment code and multiply round trips; a soft warning is enough |

## 8. Open questions

Each needs a decision before the part that depends on it is built.

- **Generics and interfaces.** How code is written once for many types without overloading, and what that costs in dev mode. Pillars 1 and 4 depend on the answer.
- **Sequential effects.** A notation for step-by-step input and output that still compiles to messages and effects.
- **Concurrency.** What runs in parallel and how state is shared, if at all. Results must stay the same for the same inputs.
- **Error handling.** One mechanism, to be chosen.
- **Names and packages.** How definitions are grouped, how name clashes are avoided, and how one program depends on another without a version solver.
- **Approval.** How a person writes acceptance examples and approves a change to a grant, and what the agent sees when it is refused.
- **Costs to measure.** Overflow and bounds checks on numeric code; reference counting and handle checks on pointer-heavy code; the state copy per handler; how slow dev-mode code is on a visual app.
- **Implementation language.** What the compiler itself is written in.
- **Drawing library.** Which existing 2D library the GUI sits on, which libraries supply text layout and accessibility, and whether the elements are built on an existing toolkit. Candidates are in section 10.

## 9. The first experiment

The largest risk is that agents write Paka worse than languages they already know, because no Paka code exists to learn from. The bet is that a small language, familiar syntax and fast, precise feedback outweigh that. The bet can be tested cheaply, before any code generation is built.

**Build:** the language reference, a parser, a type checker, and a slow interpreter that exists only to run the tests.

**Measure:** the same model on the same tasks in Paka, Rust and TypeScript, each held to the same tests:

- first-try pass rate
- edits needed to reach a passing program
- tokens needed to reach a passing program

**Decide:** the tasks, the model and the losing margin are written down before the first run. If Paka loses by more than that margin, the project stops.

**What published results suggest** (sources in section 10):

- On a language models have seen less of, the cost showed up in tokens, not correctness. Five recent models wrote OCaml about as accurately as Python, one small model excepted, but spent 1.28–1.69× the output tokens.
- On a language models had never seen, correctness dropped too. Models fine-tuned on one learned its syntax quickly and still scored up to 19 percentage points below Python in the main setup, and more in others.
- Easy tasks separate nothing. A benchmark built for another agent-targeted language averaged 96.7–99.7% across Vera, Python and TypeScript, and its author calls it saturated. The tasks here must be hard enough that the Rust and TypeScript runs fail some.

**Result.** The rules were frozen in [`experiment/protocol.md`](experiment/protocol.md) before any counted run. One model (`claude-sonnet-5-5`, medium effort) solved 14 tasks in seven classes, 4 times per language, with up to 8 submissions each and build and test feedback after every one. Two task sets were calibrated on Rust and TypeScript first. The first was too easy, so 11 of its 14 tasks were replaced with harder ones before the counted runs.

| | Paka | Rust | TypeScript |
|---|---|---|---|
| First try | 75.0% | 83.9% | 87.5% |
| Mean edits | 0.34 | 0.20 | 0.21 |
| Output tokens | 445,593 | 192,932 | 157,316 |

Against Rust, the first-try gap of 8.9 points and the edits gap of 0.14 are within the margins. The token ratio of 2.31 is past the limit of 2.0, so the pre-set rule says **stop**. Every run in every language was solved. The pattern matches the OCaml result above, cost in tokens rather than correctness, but the cost is larger. Paka runs also send the whole language reference with every call, about 9,400 input tokens; input tokens are outside the rule.

What this does not settle: one model, one effort setting, and a language no model has been trained on. Whether the token cost falls as models see Paka code is untested. Rulings on single runs and the go / no-go analysis are still open, and the decision is the owner's. Raw logs are in `experiment/runs/`.

**Order of work if Paka passes:**

1. Dev mode for headless programs, with console and file grants.
2. Live GUI on Linux.
3. Release mode, with benchmarks against Rust and C++.
4. Human-owned examples, the merge and release gate, and quarantined foreign code.

## 10. Prior art

Paka invents little. It combines known ideas and removes what gets in an agent's way.

It is also not the first language aimed at agents. A [public catalogue](https://agentlanguages.dev/) listed 42 of them on 12 October 2026.

The figures below come from the linked sources and were measured on their systems. A few are computed from their tables and say so. None is a Paka result.

**Languages for agents.**

- [MoonBit](https://docs.moonbitlang.com/en/latest/language/index.html) calls itself AI-native and has a full toolchain. It is in beta-preview.
- [Zerolang](https://github.com/vercel-labs/zerolang) stores the program as a semantic graph and treats text as a view of it. That is the design section 2 rejects.
- [Vera](https://github.com/aallan/vera) has no variable names, and requires contracts and typed effects.
- [AILANG](https://agentlanguages.dev/languages/ailang/) and [Mog](https://agentlanguages.dev/languages/mog/) already grant permissions to agent-written code.
- The combination of definition-level hot swap, a built-in live GUI, and human-owned examples and grants was not found in the projects surveyed. That describes the survey. It is not a claim to be first.

**How models handle unfamiliar languages.** This is the evidence behind section 9.

- Models do best on the languages they saw most ([MultiPL-E](https://arxiv.org/abs/2208.08227)).
- On a language absent from pretraining, fine-tuned models scored up to 19 percentage points below Python in the main setup, and more in others. They learned the syntax quickly; the meaning did not transfer ([Syntax Without Semantics, 2026](https://arxiv.org/abs/2605.15607)).
- Five recent models spent 1.28–1.69× the output tokens writing OCaml as writing Python, with accuracy mostly unchanged except for one small model ([Wu, Anderson and Guha, 2026](https://arxiv.org/abs/2607.22807)).
- [VeraBench](https://github.com/aallan/vera-bench) scored 98.7% for Vera, 96.7% for Python and 99.7% for TypeScript. Its author calls it saturated.
- In generated TypeScript, 94% of compile errors were type errors and 6% were syntax errors ([Mündler et al., 2025](https://arxiv.org/abs/2504.09246)). This suggests precise type feedback matters more than syntax.
- No controlled study was found on which syntax shape models handle best. Paka's choice of a Swift-like shape rests on familiarity, not on evidence.

**Instant build.**

- *Rebuilding only what changed.* The Rust compiler is built around [memoized queries](https://rustc-dev-guide.rust-lang.org/query.html) and [reuses results whose inputs are unchanged](https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation.html). [Salsa](https://github.com/salsa-rs/salsa) packages the same idea and [drives rust-analyzer](https://rust-analyzer.github.io/book/contributing/architecture.html). Stopping when a unit's output is unchanged is called cutoff recompilation. [Adams, Tichy and Weinert (1994)](https://www.cs.tufts.edu/~nr/cs257/archive/walter-tichy/p3-adams.pdf) measured it and name a 1984 system as an earlier user; [Build Systems à la Carte (2018)](https://www.microsoft.com/en-us/research/uploads/prod/2018/03/build-systems-a-la-carte.pdf) gives the general account.
- *One definition as the unit.* [Unison](https://www.unison-lang.org/docs/the-big-idea/) identifies each definition by a hash of its syntax tree and keeps code in a database. Paka takes the unit and keeps text as the source.
- *No separate link step.* [Zig 0.17](https://ziglang.org/download/0.17.0/release-notes.html) ships incremental compilation for most projects on x86_64 Linux. A core developer [describes](https://mlugg.co.uk/posts/incremental-compilation-internals/) it as recompiling only the changed declarations and patching their bytes into the output binary through a linker built into the compiler, and reports rebuilds of 50–70 ms on one application. It is opt-in and not yet stable. This is the nearest evidence that the 100 ms target can be met.
- *Quick unoptimized code.* [Cranelift](https://cranelift.dev/) produced code about as fast as unoptimized LLVM while compiling 20–35% faster ([Engelke and Schwarz, 2024](https://home.cit.tum.de/~engelke/pubs/2403-cgo.pdf)). [Copy-and-patch](https://arxiv.org/abs/2011.13127) stitches code from precompiled templates. [TPDE](https://arxiv.org/abs/2505.22610) reports a back-end 8–24× faster than LLVM's unoptimized one at the same code speed.
- *How slow unoptimized code is.* About 2.5× for GCC on 17 small C programs (computed from [Leroy, 2009](https://arxiv.org/abs/0902.2137)), 2–3× for [WebAssembly baseline compilers](https://arxiv.org/abs/2305.13241), and commonly 10–100× for [Rust dev builds](https://nnethercote.github.io/perf-book/build-configuration.html).
- *Holes.* [Hazelnut](https://www.doi.org/10.1145/3009837.3009900) type-checks programs with holes and [Hazelnut Live](https://arxiv.org/abs/1805.00155) runs them. GHC can [defer a hole](https://downloads.haskell.org/ghc/latest/docs/users_guide/exts/typed_holes.html) so that it fails only when evaluated. [Blinn et al. (2024)](https://arxiv.org/abs/2409.00921) use typed holes to give a model its context.

**Native speed.**

- *Ownership without lifetimes.* In [mutable value semantics](https://www.jot.fm/issues/issue_2022_02/article2.pdf), references exist only at function boundaries and cannot be stored, and a syntactic check keeps `inout` arguments from overlapping. Paka's rule is this one. [Hylo](https://github.com/hylo-lang/hylo) is built on it and has no stable release. Swift [enforces exclusive access](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0176-enforce-exclusive-access-to-memory.md) partly at run time.
- *A warning.* Swift is [experimenting with lifetime annotations](https://forums.swift.org/t/experimental-support-for-lifetime-dependencies-in-swift-6-2-and-beyond/78638) for a case the simple model does not reach: returning a borrowed view of data from a function. Paka forbids exactly that, and may feel the same pressure.
- *No-overlap facts.* Rust has passed them to its optimizer [by default since 1.54](https://releases.rs/docs/1.54.0/). No published controlled benchmark of what they buy was found. When Rust switched them off in 2016, it [measured a very small loss](https://github.com/rust-lang/rust/pull/31545). Without them, LLVM can still vectorize a loop [behind a run-time overlap check](https://llvm.org/docs/Vectorizers.html). Parity on loops is a design expectation, not a published result.
- *Cost of checks.* Overflow checks slowed the SPEC CINT 2006 benchmarks by 11.8% under unmodified LLVM in [2016](https://blog.regehr.org/archives/1384). Putting back the bounds checks that the authors of one Rust decompressor had removed by hand cost 2.5–7.7%, and one copy loop ran 7× slower because the check blocked SIMD ([Popescu et al., 2021](https://www.cs.princeton.edu/~npopescu/papers/nader.pdf)). Google reports [0.30% on average](https://security.googleblog.com/2024/11/retrofitting-spatial-safety-to-hundreds.html) for bounds-checked C++ across its services.
- *Whole-program optimization.* About 4% on the SPEC CPU 2017 integer benchmarks [with GCC 9](https://hubicka.blogspot.com/2019/05/gcc-9-link-time-and-inter-procedural.html). It buys a few percent, not a multiple.

**Live GUI.**

- *State, update, view, and effects as data.* Paka's `app` is [the Elm Architecture](https://guide.elm-lang.org/architecture/). Elm hands effects to its runtime as [command values](https://guide.elm-lang.org/effects/).
- *Views as functions of state.* [SwiftUI](https://developer.apple.com/documentation/swiftui/model-data) and [Jetpack Compose](https://developer.android.com/develop/ui/compose/mental-model).
- *Swapping code while state survives.* [Flutter's hot reload](https://docs.flutter.dev/tools/hot-reload) does it, lists the edits it cannot handle, and warns that kept state may not match the new code. Paka's migrate-or-reset rule is aimed at that gap. Since [Flutter 3.44](https://flutter.dev/blog/whats-new-in-flutter-3-44), a coding agent finds the running app and hot-reloads it with no manual setup. This is the closest existing work to pillar 3.
- *Older still.* Erlang [replaces code in a running system](https://www.erlang.org/doc/system/code_loading.html) one module at a time and [migrates state through a callback](https://www.erlang.org/doc/system/release_handling.html).
- *Keys.* Without a key, [React](https://react.dev/learn/rendering-lists) falls back to position in the list. Paka makes the key mandatory on every element that holds its own state.
- *Drawing.* Candidates are [Skia](https://skia.org/), [Vello](https://github.com/linebender/vello) and [Blend2D](https://blend2d.com/). Flutter, once Skia's best-known user outside Chrome and Android, now [defaults to its own renderer](https://docs.flutter.dev/perf/impeller) on every platform but the web (on Android, from API 29).
- *Text and accessibility.* [HarfBuzz](https://harfbuzz.github.io/what-harfbuzz-doesnt-do.html) shapes text and nothing more: line breaking and bidirectional text need other libraries. [AccessKit](https://github.com/AccessKit/accesskit) connects a self-drawn UI to screen readers.
- *Toolkits that already draw their own declarative UI.* [Slint](https://github.com/slint-ui/slint) and [Xilem](https://github.com/linebender/xilem). Building on one of them is the open choice in section 8.

**Permissions.**

- *Authority passed as values.* The [object-capability model](https://en.wikipedia.org/wiki/Object-capability_model), used by [Pony](https://tutorial.ponylang.io/object-capabilities/derived-authority.html) and [Austral](https://austral-lang.org/spec/spec.html). Both lose the guarantee at the boundary with C ([Pony's account](https://tutorial.ponylang.io/object-capabilities/trust-boundary.html)), the same limit as in 3.5.
- *Grants at the edge of the program.* [WASI](https://wasi.dev/security) and [Deno](https://docs.deno.com/runtime/fundamentals/security/) start a program with no access and grant it from outside. Paka's grants are of this kind: per app, not per object.
- *Effects in signatures.* The complaint that they spread through every caller is old: [checked exceptions](https://www.artima.com/articles/the-trouble-with-checked-exceptions) in 2003, [function colouring](https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/) in 2015. [Koka](https://koka-lang.github.io/koka/doc/book.html) answers it by inferring each function's effects. [Effekt](https://effekt-lang.org/) keeps effects out of the signatures of functions that take functions, and [infers them](https://effekt-lang.org/tour/effects) for non-recursive functions. [Flix](https://flix.dev/) lets a function take on the effects of the functions passed to it, but every function still declares its own. Paka infers nothing across definitions, for the reason in section 7.

**Evidence.**

- Tests written by a model tend to record what the code does, not what it should do ([Konstantinou et al.](https://arxiv.org/abs/2410.21136)). Given buggy code, a model writes tests that assert the bug ([Zhao, Zhou and Cohen, 2026](https://arxiv.org/abs/2607.22883)). This is why agent-written examples are labelled.
- The evidence is not one-sided. In one study, model-written checks caught 43% of seeded faults and human-written ones 45% ([Molinelli et al., 2025](https://www.lucadigrazia.com/papers/ase2025.pdf)). Counting only human-owned examples is a cautious rule, not a settled finding.
- Proofs as a gate: the Z3 solver was unstable on 2.6% of 17,043 queries from six verification projects ([Mariposa, 2023](https://www.andrew.cmu.edu/user/bparno/papers/mariposa.pdf)).
