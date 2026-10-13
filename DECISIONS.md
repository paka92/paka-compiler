# Paka owner decisions

Every design decision made so far, by task (see TASKS.md). Each entry lists the options weighed, the
one taken and why. The owner chose the first two points of M1-06 directly and, on 2026-10-13,
accepted the recommended option for the rest. Any entry can be overruled.

That acceptance does not cover downloads, installs, repository settings or anything that costs
money. Each of those needs the owner's own yes; such points are marked "owner must still".

## Project settings

- Branches: work lands on `development`. `main` changes only through a pull request the owner
  merges: one per milestone, the first when milestone 1 passes its exit check.
- CI: GitHub Actions, with clang 21 installed on the runner.
- No paid services: nothing in the plan needs an API key or a paid service. The experiment's model
  calls (M3) go through the Claude Code command line (`claude -p`).
- Downloads allowed: the TypeScript compiler (M3-05) and the Cranelift crates (M4-01b). LLVM 21 is
  already on the build machine. Each task still names what it fetches before it fetches.

Not decided, because each waits on a measurement or a result: M3-21, M3-23, M4-01b, M4-04, M4-05, M5-08, M5-09, M6-23.

## At a glance

| Task | Question | Decision |
|---|---|---|
| M1-06 | A program asks for an effect its grants do not allow. What happens when it runs? | Treat it as a trap |
| M1-06 | Print cannot write its output (disk full, or the reader closed the pipe). What happens? | Always an error |
| M1-06 | How does the runtime know which message to send first? | The app block says it |
| M2-01 | Are graph tasks in the experiment, and does M2 build pools for them? | Graphs by list index |
| M2-01 | Must any task need generic code written by the agent, or are the built-in generic collections enough? | Built-in generics only |
| M2-01 | How much input and output must a task be able to do? | Whole files, several steps |
| M2-01 | Are floating-point tasks in, and with how much maths? | Float, basic maths |
| M2-01 | Do task inputs hold only ASCII text, or any Unicode text? | ASCII inputs only |
| M2-02 | How does a function give back its result: the last expression, an explicit `return`, or both? | Last expression, early return |
| M2-02 | Are enum variants written bare (`Start`) or with their enum (`Msg.Start`), and may two enums use the same variant name? | Bare, unique per program |
| M2-02 | Does the first letter's case decide what kind of thing a name is? | Case enforced, carries meaning |
| M2-02 | When does a call write argument names, and who decides which arguments carry one? | Bare unless marked labelled |
| M2-02 | How are local variables declared, and may a name be bound again while it is still visible? | let, var, no rebinding |
| M2-02 | May a call's result be thrown away without saying so? | Must use or discard |
| M2-02 | Must a `match` on an enum name every variant, or may `_` cover the rest? | Every variant named |
| M2-02 | Does Paka have comments, and which ones are kept? | Anywhere, all kept |
| M3-01 | What can the model do inside one run: only answer with a whole program, or also call tools? | No tools, whole file |
| M3-01 | How many submissions may one run make before it is recorded as never passing? | Eight submissions |
| M3-01 | After a failed submission, how much of the test results does the model see? | Visible full, hidden counted |
| M3-01 | Do the tokens of the Paka reference in the prompt count toward the token metric? | Output tokens decide |
| M3-01 | A Paka submission fails only on a harmless spelling slip the checker can name a fix for. Does that cost a counted edit? | Counted like any error |
| M3-01 | Is a TypeScript type error a failed submission, or does the program run anyway? | Strict check must pass |
| M3-02 | Which model writes the programs? | Sonnet 5.5 |
| M3-02 | Which language is Paka's result measured against for the stop rule? | Rust decides, TypeScript reported |
| M3-02 | How does a run that never passes enter the edit and token numbers? | Cap plus one, real tokens |
| M3-02 | How many times is each task run in each language? | Five per task |
| M3-02 | How far behind Rust must Paka be on a metric to have lost it? | Just beyond published gaps |
| M3-02 | How is chance kept from deciding the verdict? | Fixed band, no statistics |
| M3-02 | How many of the three metrics must be lost for the verdict to be stop? | Any one lost |
| M3-02 | What is the usage cap for all model calls in M3? | Twice the expected tokens |
| M3-03 | After the owner tags the `run` commit, may anything be fixed and run again? | Only harness faults rerun |
| M3-03 | A counted run fails because the Paka tools and the reference disagree: the checker or interpreter does not do what the text says. What happens? | Drop the task everywhere |
| M3-03 | A counted Paka run fails because the reference was silent or unclear, while the tools do what the text says. Does the run count? | It counts as it fell |
| M3-03 | When is a counted task too easy or too hard to keep, as measured by the Rust and TypeScript calibration runs? | Swap at 6 of 6 |
| M3-03 | A chosen task turns out to be impossible, or just clumsy, to write in Paka. What happens to it? | Fix bugs, swap gaps |
| M3-03 | Between the `protocol` and `run` tags, what may be changed using what the dry run shows? | Wording yes, rules no |
| M3-04 | How many counted, reserve and practice tasks are there? | Thirty tasks, 22 counted |
| M3-04 | Are the tasks written fresh for this experiment or taken from public exercise sets? | Written fresh |
| M3-04 | What makes a task hard? | Many exact rules |
| M3-04 | Does the list deliberately include tasks where Paka is expected to pay? | Yes, eight tagged |
| M3-04 | How are the 22 counted tasks spread over the seven task classes of M2-01? | Even spread |
| M4-01 | What does the stopwatch cover when a generator compiles one definition? | Core form to first call |
| M4-01 | How do the same three definitions reach each generator? | Two lowerings from core form |
| M4-01 | What goes into the three test definitions, and how long are they? | 6, 40, 200 lines, numbers |
| M4-01 | Is each generator fed and set the way Paka would really use it, or tuned to its best time? | As Paka would use it |
| M4-01 | What decides which generator wins? | Budget gate, then default |
| M4-01 | Is the LLVM side built and timed when Cranelift already passes the gate? | Cranelift first, LLVM if needed |
| M4-01 | How big is the program on which M4-44 times edits? | 5,000 definitions |
| M4-01 | Which edits does M4-44 time, and which must pass 100 ms? | Body edits gated, rest reported |
| M4-03 | How do an agent and the tools talk? | JSON lines on stdio |
| M4-03 | What is the base version an edit names? | Hash of the canonical text |
| M4-03 | What does one edit call change, and does it run the program? | One definition, optional run |
| M4-03 | What does a find query by type shape mean? | Takes and gives, exact |
| M4-03 | What gives way when a view does not fit its token budget? | Drop signatures, never the body |
| M4-47 | Does the layout depend on line width, or only on the structure of the code? | Structure only, no width |
| M4-47 | Which constructs break across lines? | Blocks broken, lists flat |
| M4-47 | Where does a comment sit in canonical text? | Own line, above |
| M4-47 | What does a tool do with text that parses but is in another layout? | Accept, store canonical |
| M4-47 | Do tools quietly fix harmless wrong spellings: a stray semicolon, return on the last line, wrong letter case? | Errors with a fix |
| M4-47 | What happens to sources stored before the layout existed? | Leave them as stored |
| M4-48 | What is a definition's full name? | The bare name |
| M4-48 | In a directory program, which file does a new definition land in? | One file per definition |
| M5-01 | How does a view write the children of an element? | Child block after element |
| M5-01 | What may a child block contain besides element lines? | let, if, for, match |
| M5-01 | Is an element an ordinary value that any code can build, store and pass around? | Only views, only in place |
| M5-01 | Does a view give exactly one root element, or any number of elements? | Exactly one root |
| M5-01 | May a view call another view, and may the two have different message types? | Plain call, same messages |
| M5-01 | How does a visual app get its first state and its first frame? | Defaults, then optional start |
| M5-01 | What happens when the person closes the window? | Closing ends the app |
| M5-02 | Outside a loop, which elements must carry a key? | Elements that hold state |
| M5-02 | Which elements made inside a `for` must carry a key? | Every loop child keyed |
| M5-02 | Within what scope must a key be unique, and how does that give every element an id unique in the whole tree? | Siblings; id is path |
| M5-02 | How is a child without a key told apart from its siblings? | Place in the source |
| M5-02 | What does a key on a view call mean? | Call key names the root |
| M5-02 | What kinds of value may a key be? | Int or String |
| M5-02 | What is shown when two siblings carry the same key? | One error node, in place |
| M5-02 | When is the state an element holds thrown away? | Gone from tree, gone |
| M5-03 | Who holds the text a person types: the app's state or the input element? | App holds the text |
| M5-03 | What does the input send the app when the person edits? | The whole new text |
| M5-03 | What is shown when the app does not store the text the input sent, because the handler changed it, ignored it or trapped? | App wins, cursor follows |
| M5-03 | Where does half-composed text live while a person types through an input method? | Element only, until committed |
| M5-03 | Can the app read or set the cursor, the selection or the focus? | No, element only |
| M5-03 | What is one step of the cursor: a code point or a whole character as a person sees it? | One code point |
| M5-04 | Which elements does M5 build? | Five the samples use |
| M5-04 | May a call leave out an element argument that has a default? | Table defaults may be omitted |
| M5-04 | Besides `gap:`, may a view set sizes, colours or fonts? | No look arguments |
| M5-04 | Which elements take keyboard focus, and does a click on a button take it from the input? | Inputs only |
| M5-04 | What happens to the paper's other elements? | Recorded as open |
| M5-05 | What unit are sizes written and computed in? | Whole units, scaled |
| M5-05 | What decides the width and height of an element? | Width down, height up |
| M5-05 | How are children lined up across a Row or Col? | Left; centred in rows |
| M5-05 | How does Text wrap? | Always wraps to its width |
| M5-05 | What happens to content that does not fit? | Sticks out, window cuts |
| M5-06 | How does an example get a running app and act on it? | Ui value, changed in place |
| M5-06 | How does an example say which element it means? | Path as a String |
| M5-06 | What is the text form of the tree? | One line per node, indented |
| M5-06 | How much does the tree text show? | All the view passed |
| M5-06 | What can an example read back? | Tree, one text, presence |
| M5-06 | Which UI examples are reported as unfinished because of a hole? | Only if it touches one |
| M5-06 | What is an example's status when the app traps or shows an error node while it runs? | Trap ends the example |
| M5-06 | Where does typed text in an example go? | To the focused input |
| M5-07 | Where are migrate and reset written: in the source, or in the edit that needs them? | Carried by the edit |
| M5-07 | What counts as a change to a stored type? | Any change but defaults |
| M5-07 | Does one migrate turn the whole old state into the new, or is there one per changed type? | One for the whole state |
| M5-07 | How does a migrate refer to the old types? | Parameter old, result new |
| M5-07 | When is the choice demanded, and what happens to an edit without one? | Refused at the edit |
| M5-07 | What happens when a migrate fails to check, or traps when it runs? | Old version keeps running |
| M5-07 | What exactly does a reset do? | Launch again, same window |
| M7-01 | What makes an approval something an agent cannot produce? | Signature from owner's key |
| M7-01 | Where do approvals live, and what exactly is signed? | One signed file, repository root |
| M7-01 | What stops a pull request from weakening the gate itself? | Gate runs from target branch |
| M7-01 | How does an example become human-owned? | Owner adopts by name |
| M7-01 | What does the approval of an example cover: its own text, or also the code it calls? | The example's own text |
| M7-01 | When an agent changes a grant list or a human-owned example, when is it stopped? | At merge and release |
| M7-01 | What exactly does the owner see when asked to approve? | One item, old and new |
| M7-02 | Where is a C library declared? | A seventh definition kind |
| M7-02 | How does Paka code call a C function? | One effect per C function |
| M7-02 | What may cross the boundary between Paka and C? | Four plain types, copied |
| M7-02 | Where does the C code run, and for how long? | One host per library, kept |
| M7-02 | What does the owner's approval of a foreign grant cover? | Declaration and named files |
| M7-02 | What holds the host's unchecked code? | Generated C, pinned clang |
| M7-02 | Where is the list of trusted libraries kept, and how does it change? | One owned text file |
| M7-03 | How are human-owned examples run against the release binary? | Compiled in, one switch |
| M7-03 | Which command ships? | A new `paka release` |
| M7-03 | Which builds carry examples inside the binary? | Release and test builds only |
| M7-03 | How is the tested binary shown to be the shipped one? | One build, hash, run, rename |

## M1-06 · Write the M1 subset spec: grammar, start, end and grant rules, hand-written cases

### M1-06.1 A program asks for an effect its grants do not allow. What happens when it runs?

**Treat it as a trap** (chosen by the owner). The rule the paper already has for traps: a headless program ends with an error. The effect is not performed, the effects after it are dropped, one line on standard error names the effect and the missing grant, exit status 3 (1 and 2 belong to the paka tool). Same on both back ends. The wording of the line is not fixed here.

```
$ ./light_no_console
light: effect Print refused: grant Console is missing
$ echo $?
3
```

- Why: A missing grant is a bug or an attack, and silence hides both. It adds no new concept.
- Locks: Little: provisional until M2-15. M1's run cases and both run loops follow it.
- Not chosen: Skip it, but say so (The effect is dropped with a line on standard error; the rest still run, status 0.) · Skip it quietly (The effect is dropped with no message; status 0.)

### M1-06.2 Print cannot write its output (disk full, or the reader closed the pipe). What happens?

**Always an error** (chosen by the owner). One rule for every cause: the program ends, one line on standard error, exit status 3, same as a trap. The program never dies by signal, so both back ends behave the same.

```
$ ./light > /dev/full
light: effect Print failed: no space left on device
$ echo $?
3
```

- Why: One rule, nothing lost silently, and both back ends agree by construction.
- Locks: Little: provisional until M2-15. `prog | head` shows an error line when head stops reading.
- Not chosen: Error, quiet on closed pipe (As above, but no message when the reader has gone.) · Ignore write failures (The program carries on and exits with what it asked for.)

### M1-06.3 How does the runtime know which message to send first?

**The app block says it** (recommended). A `start:` line in the app block names the first message. No reserved names. A visual app that needs no first message leaves the line out; a headless app without it is rejected at check time. Costs one line per headless app; the paper's wc sample gains that line.

```
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

- Why: Nothing depends on a name or on order; the app block alone says how the program starts.
- Locks: Little: provisional until M2-15. light.paka and the M1 grammar carry the `start:` line.
- Not chosen: A variant named Start (The runtime sends the variant called Start. Shortest, but a reserved name: a visual app with a Start variant for a button would have it fired at launch.) · First variant of the enum (The variant listed first is sent. Reordering variants silently changes the program.)

### M1-06 · settled without asking

- The rest of the M1 spec (tokens, grammar, diagnostic codes, a program that stops without Exit): Written by the M1-06 agent and checked by its reviewer; the owner sees it in that pull request.

## M2-01 · Decide which kinds of experiment tasks the headless language must cover

### M2-01.1 Are graph tasks in the experiment, and does M2 build pools for them?

**Graphs by list index** (recommended). Graph tasks are in; nodes sit in a List and point at each other by Int position. Pools are not built in M2 and are recorded as not decided; a wrong position traps only when it is out of range (line 107), otherwise it reads the wrong node.

```
type Graph { next: List<List<Int>> = [] }

fn link(g: inout Graph, a: Int, b: Int) {
  append(&g.next[a], b)
}
```

- Why: The usual graph tasks build the graph once and never remove a node, so a pool would add new spelling without ever exercising its one new rule, the stale-handle trap. Index lists are what a model already writes in Rust and TypeScript, and they keep a class hard enough to make those runs fail (line 382 asks for that).
- Locks: The go/no-go will say nothing about pools and handles; they are designed after 'go' with no evidence from the experiment. TASKS.md drops the pool part only 'if M2-01 rules graphs out', so the decision must say it in those terms: graph tasks in, pools not built, pool part dropped. Picking pools instead freezes the handle and stale-handle rules that M4-35 and M6-13 must copy.
- Not chosen: Graphs in pools (Graph tasks are in and use the paper's pool; a handle to a removed node traps. Costs the full M2-10 decision, the pool work in M2-35 and M2-40, and a share of the 8,000-token reference, all before the go/no-go.) · Trees only, no graphs (No task has items that point at each other, so shortest-path, dependency and cycle tasks are out. Cheapest, but the experiment loses a class of hard tasks.)
- Against: The keeper of the paper picked pools: handles are the least familiar part of Paka, and an experiment without them judges a simpler language than the one line 269 describes.

### M2-01.2 Must any task need generic code written by the agent, or are the built-in generic collections enough?

**Built-in generics only** (recommended). List<T>, Map<K, V> and the other built-in types take any element type, but the agent cannot define a generic function or type and there are no interfaces. Smallest build; a reusable helper is written once per type.

```
type Stack { items: List<Int> = [] }

fn push(s: inout Stack, x: Int) {
  append(&s.items, x)
}
```

- Why: A one-file task never needs a user-written generic; a concrete type always works. The experiment measures writing only (lines 372-374), not build time or local checking, so it cannot test the generics risk of pillars 1 and 4 under any option; user generics would add work and rules before the go/no-go and no evidence.
- Locks: The go/no-go will not cover user generics or interfaces; M2-09 records them as not decided, like pools. Equality, ordering and to-text for built-in and user types become fixed rules of the language (M2-09, M2-14), and a later interface design must fit around them. Adding type parameters later breaks no existing program.
- Not chosen: Type parameters, no interfaces (The agent may write a function or type with a type parameter, but a value of that type can only be moved, stored and passed on, not compared or printed. Costs generic declarations in the parser and checker, and M2-09 must rule before the go/no-go how such code is compiled in dev mode.) · Interfaces too (A type parameter can demand operations such as ordering through an interface, a named set of operations and a seventh definition kind (line 265 lists six). This is the unproven part: M2-22 and M2-29 at full size, the most reference tokens and the most new rules for a model to get wrong.)
- Against: Two proposers picked type parameters without interfaces: they are checked from the signature alone, cost little, and spare the agent writing each helper once per type, which costs Paka tokens against Rust and TypeScript.

### M2-01.3 How much input and output must a task be able to do?

**Whole files, several steps** (recommended). A task may read more than one file and write files, each as one whole text, by chaining messages by hand. Costs a write effect with its grant and failure message, and a harness that also checks written files; no stream and no new notation.

```
fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(src)   => [ReadFile(src, then: Loaded)]
    Loaded(text) => [WriteFile("out.txt", convert(text), then: Saved)]
    Saved        => [Print("done"), Exit]
  }
}
```

- Why: A small class of file tools keeps the message-and-effect chain inside the verdict for the price of one write effect; the read effect and the file grant are built anyway for wc (M2 exit check, part 6). It also matches the first step after 'go', console and file grants (line 386).
- Locks: The effect and grant names agents learn (M2-15), what the run loop performs (M2-36, M2-37) and the harness contract frozen with the M3 protocol. Streaming and the sequential notation stay undecided; adding them later changes how an interactive program is written.
- Not chosen: One file in, print out (Every task reads the one file named in its arguments, computes, prints and exits with a status. Nothing extra is built, since the paper's wc sample needs exactly this (lines 242-260); the effect model is then the same few fixed lines in every program.) · Line-by-line sessions (Also tasks that answer each input line as it arrives. Needs a read-line effect, an end-of-input rule and an order between prints and reads, and makes the open notation (line 355) hard to put off.)
- Against: The engineer picked one file in, print out: it is the least to build and test, and hand-written chains carry a state type Rust and TypeScript do not need, a token handicap that cannot be removed after the runs.

### M2-01.4 Are floating-point tasks in, and with how much maths?

**Float, basic maths** (recommended). Float with + - * /, comparison, rounding, square root, conversion to and from Int and one fixed-decimals text form. IEEE fixes each arithmetic result exactly, so no maths library is needed; the cost is the NaN and float-to-text rows of M2-03 with bit-exact tests.

```
fn report(xs: List<Float>) -> String {
  var total = 0.0
  for x in xs {
    total += x
  }
  var mean = fixed(total / to_float(len(xs)), digits: 2)
  "mean {mean}"
}
```

- Why: Averages and ratios are ordinary agent work, and with no implicit conversions (line 47) mixing Int and Float is a place where a model's habits can fail in Paka; the experiment should see it. sin and pow add a library and last-digit disputes for tasks that test nothing new.
- Locks: The Float rows of M2-03 (NaN bits, float-to-text, conversion traps), which every later back end is tested against. Adding sin and pow later breaks nothing; changing a float-to-text rule after programs depend on it does.
- Not chosen: Integers only (No Float in M2; averages and money use whole numbers. Drops the float rows of M2-03 and M2-08 and all of M2-08b, and float rules are decided after 'go'.) · Full maths functions (Adds sin, cos, exp, log and pow for geometry and growth tasks. Needs M2-08b, one pinned maths library with the same bits in every mode (line 158), and expected outputs that Rust and TypeScript also hit to the last digit.)
- Against: The engineer picked integers only: float tasks add the most exact-meaning work for the least power to separate languages, and float text is the likeliest source of disputed tests.

### M2-01.5 Do task inputs hold only ASCII text, or any Unicode text?

**ASCII inputs only** (recommended). Every test input is ASCII, so length, index and slice mean the same in all three languages. No task depends on what M2-03 rules for other text; tasks about accents or emoji are out.

```
example initials_ascii {
  initials(["Ada Lovelace", "Alan Turing"]) == "AA"
}
```

- Why: ASCII removes the largest source of disputed tests and a tilt toward Paka that has nothing to do with the paper's bet (line 366). String still gets one defined meaning for non-ASCII text in M2-03, and M2-33 tests it.
- Locks: Test inputs are frozen with the M3 protocol, so the verdict will say nothing about non-ASCII text. M2-03 still fixes what one position in a String is, without evidence from the experiment.
- Not chosen: Unicode by code point (Inputs may hold any Unicode text, and tasks count and index by code point, one Unicode character number. M2-03 must make positions code points and M2-14 must supply the operations; Rust and TypeScript runs fail where the agent forgets their units.)
- Against: The agent-side proposer picked Unicode by code point: real text is not ASCII, and one view of a string is where Paka is simpler and the other two languages fail honestly.

### M2-01 · settled without asking

- Form of the written list: One table: class name, example task, types, effects and grants, data shapes, what is out. Under the picks the classes are: text processing; parsing and small interpreters; state machines and simulations; grids and dynamic programming; graph algorithms by list index; integer and float reports; file tools. M3-04 cites one class per task.
- Tree-shaped data (parsers, expression trees): In, through a type that contains itself. How such a value is owned and copied is M2-05's ruling. No class needs one subtree shared by two parents.
- Shared immutable data: No class requires it. M2-10 still decides it with a tree sample, as its note in TASKS.md says.
- State machines and simulations: In. They need only enum, match and a loop over the input, so they add no feature.
- Grids and dynamic programming: In. They need lists changed in place; the in-place rule is M2-05's.
- Lookup by key (Map): In for every class; M4-38b already assumes a built-in map with a fixed iteration order. Classes may rely on Int and String keys only. Record keys and a Set type are M2-14's call after M2-09 fixes equality for user types; widening later breaks no program.
- Functions passed as values: No class needs a function that captures surrounding variables. Named functions and variant names as values are already needed by the wc sample (then: Loaded, line 248). Whether closures exist is M2-09's ruling.
- Wrapping arithmetic and bit operations (hashes, checksums, random-number generators): Out as a task class; 64-bit hash tasks would tilt against TypeScript for a reason outside the bet. M2-03 still fixes the wrapping spelling, as its done-when and line 106 demand; no task depends on it.
- Standard input: No class needs it. Input arrives as files named in the arguments, as in the wc sample, so there is one way in. M2-15 still rules on standard input and may rule 'none in M2'.
- Malformed input and missing files: In. A class may demand an error message and a non-zero exit status. The mechanism is M2-12's and M2-15's.
- Text patterns (regular expressions): Out. No built-in pattern matcher; text is parsed by hand. A matcher would be permanent vocabulary and permanent back-end work.
- Clock, random numbers, network, environment variables, directory listing, foreign code: Out. They break same-input-same-output tests across three languages, and each would add a grant.
- Concurrency: Out. Every task is single-threaded; the paper leaves concurrency open (line 356) and M2-15 records it as deferred.
- GUI tasks: Out. The experiment is headless (line 368); pillar 3 is not measured and the list says so.
- Program size and files: One file per program, matching M3's rule that one edit replaces the whole file. Packages stay undecided (line 358).
- Speed, input size and recursion depth: No class is about speed. Inputs stay small enough for the slow interpreter and within the call-depth limit M2-06 sets.
- Big integers: Out. No task needs more than one machine Int; its width is M2-03's.
- Holes and examples inside task solutions: Not a task class. Both are built regardless (M2-06); whether a run may use them is M3-01's ruling.
- Tuples, sorting, splitting, joining and similar helpers: Assumed needed by the kept classes. Whether tuples exist is M2-02's and M2-03's; helper names and signatures are M2-14's.
- Task difficulty and the counted list: Left to M3-04. M2-01 fixes classes only; each class must admit tasks hard enough that Rust and TypeScript fail some (line 382).
- The paper's wc and counter samples: Stay expressible under every pick, as part 6 of the M2 exit check requires.

## M2-02 · Decide bindings, control flow, return, argument labels, discarded results and names

### M2-02.1 How does a function give back its result: the last expression, an explicit `return`, or both?

**Last expression, early return** (recommended). A block's value is its last expression, so `if` and `match` give values; `return` only leaves early and is an error on the last line. Costs one extra rule, and an edit each time a model ends a body with `return` by habit.

```
fn first_neg(xs: List<Int>) -> Int {
  for x in xs {
    if x < 0 { return x }
  }
  0
}
```

- Why: It keeps the light program and every paper sample as written and lets a loop stop with an answer. Rejecting `return` on the last line keeps one spelling per case (line 51).
- Locks: Every function body, match arm and example; whether a block carries a value in the core form (the small internal form the interpreter runs); how M2-06 examples and M2-12 error handling read. Going to statements later rewrites every program and every conformance case.
- Not chosen: Last expression, no return (There is no `return`; every function has one exit. Smallest rule, but stopping a loop with an answer needs a flag variable and every habitual `return` is an error.) · Always return, statements only (`if` and `match` are statements and every result leaves through `return`, as in TypeScript. Costs more tokens and temporaries, and the light program and the paper's match arms (lines 248-252) must be rewritten, so M1 cases change.)
- Against: All three proposers picked this. The strongest counter-case: Swift, Kotlin and TypeScript writers end bodies with `return`, so the last-line error may cost counted edits (lines 372-373). Starting with no `return` at all is the smaller rule and adding it later breaks nothing.

### M2-02.2 Are enum variants written bare (`Start`) or with their enum (`Msg.Start`), and may two enums use the same variant name?

**Bare, unique per program** (recommended). A variant is written bare and its name may exist once in the whole program, built-ins included. Costs: common names (Done, Loaded) clash as programs grow, and every built-in name is taken from users.

```
enum Msg { Start, Loaded(text: String) }

fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start        => [Exit]
    Loaded(text) => [Print(text), Exit]
  }
}
```

- Why: It is the spelling of the paper and of the light program, uses the fewest tokens, and a clash is an exact error at the declaration. It is the only option that can be loosened later without rewriting programs.
- Locks: Name lookup as a pass before type checking (M2-13). The built-in names table: every built-in takes a name from users, and a built-in added later (for example the M5 elements Text, Row, Button) can break an existing program. Every match arm and constructor. Moving to qualified names later rewrites every program; allowing shared names later does not.
- Not chosen: Always with the enum (Every use names its enum, patterns included, so nothing ever clashes. Costs tokens on every arm and constructor, and the light program becomes `Effect.Print(...)`, so M1 cases change.) · Bare, chosen by type (Two enums may share a name; a bare `Start` means the variant of whichever enum the position expects. Costs: name lookup must wait for type checking, which is overloading by another door (line 45), and errors get vaguer.)
- Against: All three picked this. The engineer's warning: always-qualified costs the same to build and never clashes, which matters when several agents add variants at once (line 147 already counts 'add the same name' as a conflict).

### M2-02.3 Does the first letter's case decide what kind of thing a name is?

**Case enforced, carries meaning** (recommended). First letter Upper for types, variants, effects, grants and elements; lower for functions, views, apps, examples, locals, fields and labels; wrong case is an error with a fix. In a pattern an Upper name is always a variant and a lower name always a new variable; every paper sample already conforms.

```
fn size(m: Msg) -> Int {
  match m {
    Start        => 0
    Loaded(text) => count_words(text)
  }
}
```

- Why: It turns the most dangerous silent mistake in `match` into an exact error at no token cost, and a name's kind can be read from the name alone. Relaxing it later is free; adding it later breaks programs.
- Locks: Two kinds of identifier in the lexer (M2-07), the pattern grammar (M2-11), the spelling of every built-in name (M2-14), and every name in every program.
- Not chosen: Free case, lookup decides (Any name may have any case; a bare name in a pattern is a variant if one exists, otherwise a new variable. Costs: a misspelt variant is accepted and matches everything, and a definition added elsewhere can change what an untouched body means.) · Free case, bindings marked (No case rule; a pattern makes a new variable only with `let`. Unambiguous, but costs a token per bound field and rewrites the paper's patterns at lines 248-249.)

### M2-02.4 When does a call write argument names, and who decides which arguments carry one?

**Bare unless marked labelled** (recommended). Parameters go by position unless the signature marks them as labelled (shown here as `;`); a call writes exactly those labels, in declared order. Costs one mark in signatures, and two same-typed bare arguments can still be swapped.

```
fn clamp(x: Int; lo: Int, hi: Int) -> Int {
  max(lo, min(x, hi))
}

fn tidy(n: Int) -> Int {
  clamp(n, lo: 0, hi: 9)
}
```

- Why: It is the only rule that keeps every call in the paper as written and still has one spelling per call (line 51). The label is read from the signature, which the agent has in view (line 315), so a mismatch is a mechanical fix.
- Locks: Every signature and call. Labels join the interface hash (the hash that decides whether users are rechecked, line 285), so renaming a labelled parameter breaks its callers. The signatures of all built-ins (M2-14) and later GUI elements (M5-04), and whether labels are part of a function's type (M2-09). Going from 'Positional only' to this rule later breaks no user code, but built-ins already published without labels would change.
- Not chosen: Labelled unless marked bare (Swift's rule: a parameter's name is its label unless the signature marks it bare with `_`. Costs a `_` on every bare parameter, and the paper's signatures at lines 205 and 246 change.) · Positional only (No labels anywhere: `Col(8)`, `ReadFile(path, Loaded)`. Least to build and fewest tokens, but swapped same-typed arguments are a silent bug that only an example catches (line 151), and the paper's calls at lines 219, 222 and 248 change.) · First bare, rest labelled (One fixed rule with nothing to mark: the first argument is bare, every later one carries its parameter name. Costs: the paper's `update(&s, Dec)` and `max(0, ...)` (lines 214, 208) become `update(&s, m: Dec)` and `max(0, b: ...)`.)
- Against: The engineer picked Swift's rule (labelled unless `_`): it is the one rule models were actually trained on, and line 41 names Swift as the shape Paka follows, so no new mark has to be learnt.

### M2-02.5 How are local variables declared, and may a name be bound again while it is still visible?

**let, var, no rebinding** (recommended). `let` is fixed, `var` can be reassigned, and a name cannot be bound again while an earlier binding is still visible (separate loops and match arms may reuse one). A local may hide a top-level function, so adding a function never breaks other bodies; the cost is a second name where Rust habit would rebind.

```
fn total(xs: List<Int>) -> Int {
  let bonus = 10
  var sum = 0
  for x in xs { sum += x }
  sum + bonus
}
```

- Why: A name means one thing wherever it is visible, and both errors (assigning to a `let`, binding a name again) come with an exact fix. Allowing rebinding later is free; forbidding it later breaks programs. The proposers wrote 'once per definition'; read strictly that forbids two match arms both binding `text`, so the rule here is 'not while still visible'.
- Locks: What M2-05 may assume about locals that never change, scoping in the name resolver (M2-13), and the example spelling `var s = Counter {}` (line 213).
- Not chosen: let, var, rebinding allowed (Same keywords, but a later `let` with the same name hides the earlier one, as in Rust. Matches a habit, but an unintended reuse is silent and a name no longer means one thing in a body.) · One keyword, all changeable (Only `var`; every local can be reassigned. One rule fewer, but an accidental reassignment is never reported and the ownership rules (M2-05) lose 'this never changes' as a known fact.) · No clash with any name (As the first option, but a local may not reuse a top-level name either, so this sample is rejected. Simplest to state, but adding a function named `sum` breaks every body with a local `sum` that never called it (against lines 145-147).)
- Against: The engineer's default was 'No clash with any name', so that a name means one thing in the whole program. All three warn that models rebind by Rust habit and will fail some first tries.

### M2-02.6 May a call's result be thrown away without saying so?

**Must use or discard** (recommended). A statement that produces a value is an error unless it is the block's result or is written `_ = expr`. Costs three tokens on each intended discard, and an edit when a model writes a bare call by habit.

```
fn drain(q: inout List<Int>) {
  _ = pop(&q)
  _ = pop(&q)
}
```

- Why: With pure functions an unused result is always a mistake or a deliberate drop, and the checker can say which at once instead of leaving it to examples (line 151). Loosening later is free; tightening later breaks programs.
- Locks: What counts as a statement in the grammar, `_` as a reserved name, and the ground M2-12 stands on: a returned failure cannot be ignored by accident.
- Not chosen: Silent discard allowed (A value nobody uses is dropped without a word, as in TypeScript. Nothing to learn, but a helper that returns effects can be called and ignored and still looks like correct code.) · Only marked types (Only results of marked types (effects, the error type from M2-12) need `_ =`; the rest may be dropped. Costs a marking mechanism and a second rule in the reference, and a pointless pure call still passes.)

### M2-02.7 Must a `match` on an enum name every variant, or may `_` cover the rest?

**Every variant named** (recommended). A match on an enum names every variant; several may share an arm with `|`, and `_` is allowed only for Int, String and a field inside a pattern. Costs longer matches on wide enums, and an edit when a model writes `_ =>` by habit.

```
enum Job { Queued, Running(id: Int), Done }

fn is_idle(j: Job) -> Bool {
  match j {
    Queued | Done => true
    Running(_)    => false
  }
}
```

- Why: It turns 'someone added a message' into an exact list of broken definitions, which is the feedback the paper promises on every edit (lines 145, 323). Allowing `_` later is free; banning it later breaks programs.
- Locks: The pattern grammar (`|` arms), the coverage check in M2-20, and the cost of adding a variant to a widely used enum: every match on it must be edited.
- Not chosen: Catch-all allowed (`_` may end any match, as in Rust and Swift. Shortest and most familiar, but a new variant falls into `_` with no error and no entry in the list of what broke.) · Partial, trap on miss (Missing arms are allowed and reaching one stops the handler with a trap (a defined stop, line 63). Least to write, but a check-time report becomes a run-time stop, and it adds a trap kind the paper's list does not have.)
- Against: The engineer picked 'Catch-all allowed': it is what Rust and Swift writers produce by habit and it saves tokens on wide enums, and the experiment counts both edits and tokens (lines 373-374).

### M2-02.8 Does Paka have comments, and which ones are kept?

**Anywhere, all kept** (recommended). `//` to end of line is legal anywhere and never an error; every comment is stored and belongs to the definition it is in, or to the next one if it stands between definitions. Costs: the printer must carry them all (M4-15 plans to) and a stale note can mislead the next agent.

```
// Counts runs of non-space characters.
fn count_words(text: String) -> Int {
  // Empty text has no words.
  if is_empty(text) { return 0 }
  ??scan
}
```

- Why: A comment never changes meaning, so rejecting one only adds counted edits to the experiment (lines 372-373), and keeping all of them is what line 55 and M4-15 already say. In M2 it is also the least to build: the lexer skips them. This is the cheapest of the eight to change later, because any tighter rule can be applied by a mechanical rewrite.
- Locks: The lexer's comment token and the M1 comment reject case (deleted under every option but 'No comments'); what the printer in M4-15 must carry and where M4-47 places a comment written after code; what the View tool shows beside a signature.
- Not chosen: Above a definition only (One `//` block directly above a definition belongs to it and is shown with its signature; a comment anywhere else is an error with the fix 'remove'. Cheapest to build and can be loosened later for free, but a habitual body comment costs an edit.) · Anywhere, header kept (`//` is legal anywhere, but only the block above a definition is stored; comments inside bodies are dropped from the stored text. Never an error and cheap to print, but a note an agent left in a body silently disappears.) · No comments (No comment syntax; names and examples carry intent. Fewest tokens and nothing to go stale, but it reverses line 55 and every habitual `//` is an error.)
- Against: All three picked differently. Engineer ('Above a definition only'): it is one field on one node and answers 'whose comment is it' by construction. Agent lens ('Anywhere, header kept'): body notes cost tokens on every read and go stale, so keep only the one that travels with the signature. Paper keeper (own line only, a comment after code is an error): input then has one spelling too (line 51).

### M2-02 · settled without asking

- Harmless other spellings: In M2 a stray semicolon, a `return` on the last line and wrong letter case are errors with a suggested fix (line 322). Each one costs a counted edit in the experiment. Whether the checker should accept and rewrite such text instead is M4-47's ruling; raise it in M3-01 before the protocol is frozen.
- Blocks and line endings: Braces for every block, a newline ends a statement, no semicolons, as in every paper sample. A line continues only inside an open ( or [ or after a trailing operator or comma. Until M4-47 rules on layout, the checker accepts any spacing and indentation.
- if / else: Condition is a Bool, no parentheses needed, braces always, `else if` chains. Used as a value it needs an `else` and both branches must have the same type; without `else` it is a statement with no value. No ternary operator.
- Loops: `for x in <list or integer range>` and `while cond`, with `break` and `continue`. Loops are statements and give no value. No loop labels, no do-while, no C-style for. The range spelling goes to M2-03 and M2-14; what else `for` can walk goes to M2-09 and M2-14.
- match form: One arm per line, no commas, no fall-through. An arm body is an expression, a block, or one assignment as the paper writes it (line 207). A match that misses a variant is an error listing the missing ones with a fix (M2-20). Patterns are flat: one variant with names for its fields, a literal, or `_`. No nesting, no conditions on arms, no `if let` until a task needs them; adding them later breaks nothing.
- Assignment: `=`, `+=` and the like are statements, not expressions, and work only on `var` locals and `inout` parameters and their fields (lines 207-208). No `++` or `--`, no chained assignment. The exact operator list belongs to M2-03.
- Local declarations: Every `let` and `var` has an initial value. Its type is inferred inside the body (line 140); writing it (`let n: Int = 0`) is optional. Parameters cannot be reassigned unless they are `inout`.
- Functions with no result: No `->` means the function returns nothing (line 263); its last statement must not leave a value, and a bare `return` leaves early. Code after `return`, `break` or `continue` in the same block is an error, as is `break` or `continue` outside a loop.
- Unused names: An unused local or parameter is a warning, not an error, so it never costs an edit. A `var` that is never reassigned is accepted without a message. `_` ignores a field in a pattern.
- One pool of names: All definitions in a program share one pool; with case enforced, Upper and lower names cannot clash. A duplicate definition is an error naming both places. Fields, labels, variant fields and the keys inside `app { }` belong to their owner and clash with nothing, so `update: update` (line 231) stays legal. All definitions are top level, order has no meaning, and a definition may be used before it appears.
- Identifiers and keywords: ASCII letters, digits and underscore. Only the first letter's case is enforced; the reference recommends snake_case and CamelCase. Keywords are reserved everywhere, including ones kept for later milestones (view, migrate). `_` alone is the discard target and the wildcard.
- Label details: The mark is `;` between bare and labelled parameters in these samples; the written grammar fixes the spelling. Bare parameters come first, labels are written in declared order. No default values for function parameters yet (adding them later breaks nothing); type fields keep theirs (line 201). Variant fields follow the function rule, so `Start(path)` builds and matches by position. Records are built with braces and field names (`Counter {}`, line 213). Trailing blocks such as `Col(gap: 8) { }` wait for M5-01.
- Comment details: `//` only, no block comments. A comment after the last definition of a file belongs to that definition. Comments never change meaning. Whether comment text enters a definition's hashes (lines 284-286) is left to M4-11.
- Which files make one program: The single file given to the command (as in M1), or every `.paka` file directly in the directory given. File boundaries and order carry no meaning (line 71). Packages stay undecided (line 358 and the M2 notes in TASKS.md).
- M1-06 rules kept or changed: Under the picks the light program parses and means the same, so no M1 run case changes; only the six planned reject cases stop holding (comment, hole, example, match, integer literal, labelled argument). Picking 'Always return' or 'Always with the enum' would change the light program itself. M1-06 is not written yet, so the line-by-line kept-or-changed list is made against it when it lands (M2-02 needs M1-06).
- Corrections to the proposals: The paper keeper cited lines 139 and 150; the text it meant is at lines 136/140 (local checking) and 147 (when two agents conflict). The engineer wrote that labels can only be added later by rewriting every call; under 'Bare unless marked labelled' adding them later breaks no user code.
- Left to other decisions: `&` at inout call sites and the overlap rule (M2-05); operators, string interpolation and wrapping arithmetic (M2-03); holes and example bodies (M2-06); function values and whether labels are part of a function type (M2-09); error-handling forms (M2-12); the built-in names table (M2-14); start, end and missing-grant rules (M2-15); canonical layout (M4-47).

## M3-01 · Decide what one run is: prompt, tools, feedback and counting rules

### M3-01.1 What can the model do inside one run: only answer with a whole program, or also call tools?

**No tools, whole file** (recommended). Each model reply is one complete program in one code block; the harness builds it, runs the cases and sends the result back as the next message. Costs nothing beyond the planned harness, but it does not measure Paka's per-definition tools (GENESIS 312-316).

```
system:  rules of the run (+ the Paka reference, Paka runs only)
user:    task statement with 2 visible cases
model:   ```paka ... whole file ... ```
user:    build and case results
model:   ```paka ... whole file again ... ```
```

- Why: It is the smallest loop that gives all three languages the same thing to do, and it makes edit and token counts unambiguous. The protocol states the limit in plain words: per-definition tools are not measured.
- Locks: The shape of every raw log, the trial loop (M3-09) and what the words edit and first try mean in the verdict.
- Not chosen: Shell tools, agent loop (The model gets build and run tools and decides itself when to test. Closer to real agent work, but an edit is no longer one reply, tool chatter counts as tokens, and the harness and sandbox grow.) · Paka's own definition tools (Paka runs use Find, View and Edit per definition while Rust and TypeScript replace files. Those tools are not built before M4, and the three languages would no longer be held to the same loop.)
- Look again when: Only for a second experiment after 'go', when M4's tools exist.

### M3-01.2 How many submissions may one run make before it is recorded as never passing?

**Eight submissions** (recommended). The first try plus at most seven edits. A failing run costs about eight calls.

```
submission 1 = first try
submissions 2..8 = edits 1..7
no pass after submission 8 = never passing
```

- Why: It leaves room for a few habit slips on top of real bugs without letting a stuck run burn usage. The evidence is thin, but the cap is a counting rule, so it is fixed at the `protocol` tag and not tuned after the dry run.
- Locks: The never-passing rule and the edits formula in M3-02, and the token budget.
- Not chosen: Four submissions (First try plus three edits. Cheapest, but a language that needs two extra rounds for syntax habits is pushed into never passing.) · Sixteen submissions (First try plus fifteen edits. Few runs are cut off, but failing runs cost twice as much and late context gets long.)
- Look again when: Only before the `protocol` tag. If the dry run shows more than a quarter of runs still gaining cases at submission 8, the report states it as a limit; the cap does not move, because M3-19 may not change counting rules.

### M3-01.3 After a failed submission, how much of the test results does the model see?

**Visible full, hidden counted** (recommended). A failed build returns the compiler's own text. A build that works returns each failing visible case in full (arguments, expected and actual output, exit status, standard error) and for hidden cases only how many failed and in which way; it costs the hidden-case split M3-06 and M3-07 already plan.

```
build: ok
visible case 2: FAIL
  args: in2.txt
  expected stdout: "3 words\n"   status 0
  got stdout:      "2 words\n"   status 0
  stderr: (empty)
hidden cases: 5 of 8 pass (2 wrong output, 1 wrong exit status)
submission 2 of 8
```

- Why: The model gets concrete evidence to work from, the same in all three languages, and a pass still means the statement was met and not the test file. Standard error is shown because that is where a Paka trap, a Rust panic and a Node error say what went wrong.
- Locks: The task folder format (statement, visible cases, hidden cases), the sandbox rule that hidden cases are unreadable, and how hard every task is in practice.
- Not chosen: All cases shown (No hidden cases; every failure is shown in full. Simplest harness, but a model can special-case the expected outputs and still pass.) · Pass or fail only (After a working build the model learns only that some case failed. Hardest to game, but most runs would end by guessing, which measures the statement more than the language.)
- Look again when: Only before the `protocol` tag. Runs stuck on hidden cases in all three languages alike are reported as a limit, not fixed between the tags.

### M3-01.4 Do the tokens of the Paka reference in the prompt count toward the token metric?

**Output tokens decide** (recommended). The verdict's token metric is output tokens only, thinking included, summed over every call of the run. Input tokens are logged and reported per language twice, with and without the reference, but do not enter the verdict.

```
tokens(run) = sum over calls of usage.output_tokens
reported beside it: input tokens, and input tokens minus the reference
```

- Why: The published result the paper leans on is about output tokens (GENESIS 380, 411), and output is where an unfamiliar language shows its cost. The reference cost is a fixed, known number and is printed in the report instead of hidden in a ratio.
- Locks: The token formula in M3-02, the counter in M3-08 and M3-09, and the meaning of the third metric in the verdict.
- Not chosen: All tokens, reference included (Input plus output decide. Honest about everything sent, but Paka then loses by a constant known today (about 8,000 times the number of calls), so the metric measures the reference size and not the writing.) · All tokens, reference excluded (Input plus output decide, with the reference subtracted from Paka's input. Needs a subtraction rule in the counter, and input still grows with each language's compiler message length, which says little about writing.)
- Against: Counting the reference is the stricter reading: an agent really pays for it on every task. The report shows that figure, so the owner can read the stricter result in M3-23.

### M3-01.5 A Paka submission fails only on a harmless spelling slip the checker can name a fix for. Does that cost a counted edit?

**Counted like any error** (recommended). The checker rejects the file, the model sees the message with the fix, and the next reply is one edit. Nothing extra is built; Paka pays in first-try rate and edits for every habit slip.

```
fn double(n: Int) -> Int {
  return n * 2
}
// error: `return` on the last line. Fix: write `n * 2`.
// This reply was a failed first try; the next one is edit 1.
```

- Why: It tests the language as decided and builds nothing new. The harness logs the diagnostic codes of every failed check, so the report can say how many Paka edits were spelling slips only; that is the evidence M4-47 needs.
- Locks: Paka's first-try rate and edit count include habit slips. This cannot be recomputed into a verdict afterwards.
- Not chosen: Harness applies fixes free (The harness applies the suggested fix itself and judges the repaired file. Needs a fix-applying tool that M2 does not build, and Rust and TypeScript would get no such help.) · Checker accepts other spellings (The checker takes the harmless forms without an error. That reopens M2-02 and decides M4-47 early, on no evidence.)
- Look again when: If the dry run on practice tasks shows most Paka first tries fail on these slips alone, the reference wording may be improved before the `run` tag (see M3-03); the counting rule stays.

### M3-01.6 Is a TypeScript type error a failed submission, or does the program run anyway?

**Strict check must pass** (recommended). The file must pass tsc with strict mode on before it runs, just as Rust must compile and Paka must check. Needs the TypeScript compiler and Node's type package installed (M3-05).

```
tsc --strict --noEmitOnError --target es2022 --module commonjs \
    --types node --outDir out main.ts
node out/main.js <args>
```

- Why: All three languages are then held to their own checker before any test runs, which is the comparison the paper describes.
- Locks: TypeScript's numbers in the verdict tables, and the pinned packages of M3-05.
- Not chosen: Types ignored, just run (Node strips the types and runs the file. No install at all, but the baseline is then JavaScript, not the TypeScript the paper names (GENESIS 370).)

### M3-01 · settled without asking

- Prompt: One fixed system text, the same for all three languages except the language name, the file name and the build line. Paka runs add the whole language reference and one line saying the app may list any grant the reference names; Rust and TypeScript get no extra material. The user message is the task statement with its two visible cases.
- What the model sees on later turns: Each run is one `claude -p` conversation: the first call sends the system text and the statement, each later call adds the result of the last submission. Nothing earlier is edited and the model's replies stay as they were. M3-08 picks how the conversation is kept (one open process or a resumed session) and proves on the fake that earlier turns are unchanged. Input grows with each edit, but input is outside the verdict.
- Stated limit: The protocol says in these words: one edit replaces the whole file in all three languages; Paka's per-definition Find, View and Edit tools are not measured; nothing about GUI, build speed or run speed is measured.
- Pass: A submission passes when it builds and every visible and hidden case matches: standard output byte for byte, the exit status the statement names, and every file the statement says is written. Standard error and any other file are not compared.
- First try, edit: First try is submission 1. An edit is each later submission. A reply with no code block, with more than one, or cut off at the reply limit is a failed submission and counts.
- Token count: Taken from the `usage` block `claude -p` prints for each call. Output tokens, thinking included, are the counted number. Input is reported as the sum of its three input counts (plain, cache write, cache read), because `claude -p` caches on its own and adds about 800 tokens of its own text to every call, the same in all three languages. A call that failed or was refused and then retried is not counted.
- Budgets per run: Eight submissions; 32,000 output tokens per reply, thinking included, or the model's own ceiling if lower (written down at M3-18); 120 seconds per build; 10 seconds and 1 GB per case. A timeout or memory stop fails that case. The report counts cut-off replies per language.
- Program shape: One file: main.paka, main.rs or main.ts. Input comes as files named in the arguments; output is standard output, an exit status and, in file-tool tasks, written files. No standard input. Rust uses the standard library only; TypeScript uses Node's built-in modules only; Paka uses its built-ins.
- Build lines: Paka: `paka check main.paka`, then `paka run main.paka` with the arguments, in the interpreter. Rust: `rustc --edition 2024 -C debug-assertions=on -C overflow-checks=on -o main main.rs` with the toolchain M1-01 pins, then `./main`; an overflow stops the program as it does in Paka. TypeScript: the strict line of the last point. A build passes when its line exits 0. Toolchain versions are written into the protocol.
- Compiler feedback: Each language's own diagnostic text, unchanged, suggested fixes included, cut after 8,000 bytes with a note. Warnings are shown only when the build fails.
- Case feedback detail: For a failing visible case, standard output and standard error are each cut after 2,000 bytes with a note. A failing hidden case is counted under the first that applies: over time or memory, wrong exit status, wrong output, wrong written file. No hidden text is ever shown.
- Sandbox environment: Every case runs in a fresh directory with an empty environment except TZ=UTC and the C locale, so dates and text order do not depend on the machine.
- Interpreter speed: The 10-second limit must not be decided by the slow interpreter (GENESIS 368). Every Paka reference solution finishes every case in under 1 second, checked in M3-17.
- Holes, examples, in-file tests: Allowed in the file, in all three languages. The harness adds no step for them: only the build line and the cases decide. A Paka hole is ordinary dev-mode code (GENESIS 75, 161): reached during a case it traps and fails that case, as `todo!()` does in Rust.
- Grants: Statements name no grants, because M3-04 keeps them free of language features. The Paka system text says any grant in the reference may be listed. No approval step is part of a run; the sandbox bounds what any program can touch.
- Model settings: The same model, the same effort setting and the provider's default sampling for every call. No fallback model (a second model that answers when the first declines; `--fallback-model` is never passed): every reply comes from the one named model. The values are fixed in M3-02. The harness checks the model name `claude -p` reports with each reply; a run answered by any other model is void.

### M3-01 · what this changes in TASKS.md

- M3-05: Grows slightly: it pins and installs the TypeScript compiler and the Node type package with a lockfile, it names every package in that lockfile with source and size before fetching (the owner allowed the download on 2026-10-13), and it proves the strict build line works offline.
- M3-06: The three command pairs and the task folder shape (statement, 2 visible cases, 8 hidden cases, expected output, status and written files) are now fixed.
- M3-07: The limits to enforce are fixed: 120 s per build, 10 s and 1 GB per case, a fresh directory per case, an empty environment except TZ=UTC and the C locale. It reports how each case ended: exit status, over time, or over memory.
- M3-08: Calls `claude -p` with no key. Counts tokens from its `usage` block, never passes `--fallback-model`, and checks the reported model name. `claude -p` has no flag for a reply limit, so the 32,000 output-token limit is held by counting: a longer reply counts as cut off. A refused or empty reply is retried as a provider error and logged.
- M3-09: Sends the whole conversation with earlier replies unchanged, stops at eight submissions, builds the feedback text in the shape shown here, logs the diagnostic codes of each failed check, and counts cut-off replies.
- M3-17: Also checks that every Paka reference solution finishes every case in under 1 second in the interpreter.
- M3-18: The protocol records the reply limit actually used and the three build lines with versions.
- M3-22: The report also prints input tokens with and without the reference, the share of Paka edits that were spelling slips only, and cut-off and refused replies per language.
- M4-47: Gains evidence: the logged share of spelling-slip edits tells it whether accepting other spellings is worth building.
- M2-06: Should know that M3 judges a Paka build by the exit status of `paka check` alone; if a failing example makes that non-zero, a wrong example costs a counted edit.

## M3-02 · Decide metric formulas, losing margin, model, repetitions and usage cap

### M3-02.1 Which model writes the programs?

**Sonnet 5.5** (recommended). The mid-size model, run at effort medium, at relative weight 20 in TASKS.md. Uses about 20 times what Haiku does, and says nothing about how the strongest or the smallest model would do.

```
model:    claude-sonnet-5-5   (checked in every reply)
effort:   medium, `--effort medium` on every call
thinking: provider default, counted as output tokens
sampling: provider default
```

- Why: It is the lightest model that is not the known worst case (GENESIS 380), so a stop cannot be blamed on model size, and it leaves Rust and TypeScript room to fail. TASKS.md's levels say which model builds the toolchain, not which one writes Paka, so they decide nothing here.
- Locks: Every number in the verdict belongs to this model and effort setting. Changing it after the `run` tag means running everything again. Calls go through `claude -p`, with no key (rule 12).
- Not chosen: Opus 5.5 (Stronger, at relative weight 40. More tasks would be passed first try by Rust and TypeScript, so harder tasks would be needed, at twice the usage.) · Haiku 5.5 (Relative weight 1. But the one small model in the cited study is the one that lost accuracy on a less-seen language (GENESIS 380), so it is a worst case and not the writer Paka is aimed at.) · Fable 5.1 (Relative weight 100, used in TASKS.md for design only. Most likely to pass everything first try in every language, which separates nothing.)
- Look again when: If calibration shows Rust above 85% first try even after every swap, write harder tasks; do not change the model.

### M3-02.2 Which language is Paka's result measured against for the stop rule?

**Rust decides, TypeScript reported** (recommended). The stop rule compares Paka with Rust only. TypeScript is run and published the same way, as the mark of the most familiar language; the cost is that a large gap to TypeScript does not stop the project by rule.

- Why: Rust is the language Paka would be used instead of: compiled, owned values, native speed (GENESIS 83-91). If agents write Paka about as well as Rust, the risk in GENESIS 366 is retired.
- Locks: The verdict formula in M3-10, the Rust window of the calibration rule (M3-03) and the claim the report may make.
- Not chosen: Better of the two (On each metric Paka is compared with whichever baseline scored better. Strictest; Paka could stop for not matching a language that does none of what it is for.) · Average of the two (Paka is compared with the mean of Rust and TypeScript. One number, but it stands for no real language.)
- Against: GENESIS 366 speaks of 'languages they already know', plural. A Paka that matches Rust but trails TypeScript by 25 points would get 'go' under this rule; the report must show that gap plainly.

### M3-02.3 How does a run that never passes enter the edit and token numbers?

**Cap plus one, real tokens** (recommended). A never-passing run counts as 8 edits (one more than the 7 allowed) and the output tokens it really spent. It also counts as a failed first try.

```
edits(run)  = submissions - 1   if it passed
            = 8                 if it never passed
tokens(run) = output tokens spent, passed or not
```

- Why: Every run stays in every number and each value can be worked out by hand. In edits, failing always costs more than passing late; in tokens a run that gives up cheaply can look cheap, but it still loses a first try and scores 8 edits.
- Locks: The meaning of the edit and token metrics, and M3-10's formulas and test set.
- Not chosen: Leave them out (Edit and token means are taken over passing runs only, with the solve rate beside them. Simple, but hard tasks drop out for the weaker language.) · Medians over all runs (Use the median, with never-passing runs as the largest value. Robust, but the median edit count is likely 0 or 1 for every language and would separate nothing.)

### M3-02.4 How many times is each task run in each language?

**Five per task** (recommended). 22 counted tasks x 3 languages x 5 = 330 counted runs, 110 per language. Chance alone then moves the first-try gap by about 7 points, at about two thirds more cost than three per task.

```
chance movement of the first-try gap
  = square root of (2 x 0.25 / runs per language)
110 runs -> 6.7 points | 66 runs -> 8.7 | 220 runs -> 4.8
```

- Why: Five keeps chance movement at about 7 points for 330 runs, small enough for the margin and band that follow to mean something. Runs of one task tend to fail together, so the true movement is larger, and more repetitions cannot fix that.
- Locks: The budget and the run matrix the harness must complete.
- Not chosen: Three per task (198 counted runs, 66 per language. Cheapest, but chance movement rises to about 9 points, which a 10-point line cannot tell from a real gap.) · Ten per task (660 counted runs. Chance movement falls to about 5 points; cost doubles, and repeating the same 22 tasks adds less than more tasks would.)

### M3-02.5 How far behind Rust must Paka be on a metric to have lost it?

**Just beyond published gaps** (recommended). Lost if first-try rate is more than 20 points below Rust, or mean edits per run more than 1.0 above, or output tokens more than 2.0 times Rust's. That is just past the worst main-setup figures the paper cites (19 points, 1.69 times; GENESIS 380-381), so a Paka 15 points behind is not stopped by the rule.

```
F = first-try passes / runs          lost if F_paka < F_rust - 0.20
E = mean edits per run               lost if E_paka > E_rust + 1.0
T = total output tokens, all runs    lost if T_paka / T_rust > 2.0
```

- Why: A result past the worst main-setup figures for a language the model never saw is a clear loss in the paper's sense (GENESIS 26), and it stands clear of the noise of this sample. The paper notes larger gaps in other setups (GENESIS 381), and the edits line has no published figure: 1.0 is a judgement, one extra round of feedback per run.
- Locks: The stop rule. It is tagged before the first model call and cannot move.
- Not chosen: Tight margin (Lost at 10 points, 0.5 edits or 1.5 times. With about 110 runs per language, chance alone moves the first-try gap by about 7 points, so this would stop the project on noise.) · Loose margin (Lost at 30 points, 2.0 edits or 3.0 times. Almost nothing stops the project; the experiment would no longer be a test.)
- Look again when: Only before the `protocol` tag: a published result for a never-seen language given its reference in the prompt would be better evidence than the fine-tuned models of GENESIS 381.

### M3-02.6 How is chance kept from deciding the verdict?

**Fixed band, no statistics** (recommended). Each metric has two lines: held within 10 points, 0.5 edits or 1.7 times of Rust, lost beyond the margin, and close in between. Every value can be checked by hand, but the lines are a rule of thumb and not a probability.

```
first-try gap:  held <= 10 points < close <= 20 points < lost
edits gap:      held <= 0.5     < close <= 1.0      < lost
token ratio:    held <= 1.7     < close <= 2.0      < lost
```

- Why: The band is about one and a half times the chance movement, so a true gap at the held line is read as lost about 1 time in 15, and the rule stays three comparisons a person can redo. For edits this assumes a spread of about 2.5 edits per run, which is a guess.
- Locks: The words held, close and lost in M3-10, M3-18 and M3-22.
- Not chosen: Bootstrap interval over tasks (Resample the tasks many times and call a metric lost only if the whole 90% interval lies beyond the margin. Sounder, but it adds a random procedure to the tool whose output M3-10 must match by hand.) · Single threshold only (One line per metric: beyond it lost, otherwise held. Simplest, and a 19-point and a 21-point gap get opposite verdicts on noise.)
- Against: A statistician would want the interval. The raw logs are published, so anyone can compute one; it is just not the rule.

### M3-02.7 How many of the three metrics must be lost for the verdict to be stop?

**Any one lost** (recommended). One lost metric gives stop, all three held gives go, and anything else is inconclusive. Plain reading of the paper; with margins this wide a single loss is already a clear one, but there are three chances to stop by bad luck.

```
inconclusive  if more than three tasks were dropped (M3-03)
stop          else if any metric is lost
go            else if all three are held
inconclusive  otherwise
```

- Why: It is what the paper says, and it needs no weighing of one metric against another.
- Locks: The verdict rule in M3-10 and what M3-23 may conclude. Inconclusive means the owner decides in M3-23 with the limits written out; no extra runs are made under this protocol.
- Not chosen: Two of three (Stop needs two lost metrics. Paka could lose a quarter of its first tries and still pass.) · First-try rate alone (Only correctness decides; edits and tokens are reported. Ignores the published finding that the cost of a less-seen language shows in tokens (GENESIS 380).)

### M3-02.8 What is the usage cap for all model calls in M3?

**Twice the expected tokens** (recommended). Cap: 20 million input tokens and 11 million output tokens across dry run, calibration and counted runs. The harness stops when either is reached, and going on needs the owner's yes.

```
runs: counted 330 + calibration 132 + swap checks 36 + dry run 54 = 552
per run (guess): 2.5 calls, 10k output tokens, thinking included
input per run (guess): 13.5k, or 33.5k for Paka (reference in each call)
expected: about 10.0M input, 5.5M output
cap = 2 x expected = 20M input, 11M output
money: none; share of the usage limit: measured in the dry run
```

- Why: The per-run figures are guesses until the dry run measures them, and thinking counts as output. Twice gives room for five calls per run on average without leaving the usage open.
- Locks: Little. The cap can be changed by the owner at any time; it changes no result.
- Not chosen: Expected tokens exactly (Cap at 10 million input and 5.5 million output. Lowest ceiling, but the per-run guess is unmeasured and a stop in the middle of the counted runs needs the owner before it can resume.) · No cap, watch by hand (The owner watches the usage page. Nothing to configure, but nothing stops a harness fault that loops, and M3-08's done-when asks for a cap.)
- Look again when: After the dry run: if measured tokens per run differ from the guess by more than half, recompute the cap before calibration. The dry run also reports how much of the usage limit it took, and the caller projects the counted runs from that. Under rule 11 the runs pause at the limit and go on when it resets, so M3 may take more than one week.
- Owner must still: A restart of the counted runs is outside this cap and needs your yes.

### M3-02 · settled without asking

- First-try pass rate: Per language: runs whose first submission passes, divided by all counted runs of that language. Also printed per task.
- Edits to a passing program: Per language: the mean over all counted runs of edits(run), where a passing run has submissions minus 1 and a never-passing run has 8. Also printed per task.
- Tokens to a passing program: Per language: the sum of output tokens over all counted runs. The verdict uses the ratio of Paka's sum to Rust's. Also printed per task.
- Extra figures, not in the verdict: Solve rate within eight submissions; input tokens with and without the reference; the same three metrics for TypeScript; the three metrics with no task dropped; one table with tagged runs removed (M3-03); cut-off and refused replies per language. Each is labelled as not the verdict.
- Effort, thinking and sampling: Effort medium, sent on every call because the provider's default for this model is high; medium is the provider's suggested starting level for coding work and the lighter one. Thinking stays at the provider's default and is counted as output. Sampling is the provider's default; this model rejects other values. All of it is written into the protocol and confirmed on the first dry-run call.
- Calibration repetitions: Three runs per counted task in Rust and in TypeScript: 22 x 2 x 3 = 132 runs, plus up to 36 for swapped-in reserve tasks.
- Dry-run size: 2 practice tasks x 3 languages x 3 runs = 18 runs per pass, at most three passes (54 runs). A fourth pass needs the owner's yes.
- Rulings: Runs voided as harness faults are rerun inside the cap and so do not change the counts. Tasks dropped under M3-03 are removed from all three languages before any formula is applied. More than three dropped tasks makes the verdict inconclusive whatever the metrics say.
- Go: Go means only that the verdict rule did not stop the project. The owner still records go or stop himself in M3-23.

### M3-02 · what this changes in TASKS.md

- M3-08: The cap is two token counters (20M input, 11M output) that the owner may lower; the model id and effort come from the protocol file, and effort is sent on every call. Calls go through `claude -p`; no key.
- M3-10: Implements exactly the three formulas, the cap-plus-one rule, the two-line band against Rust and the four-line verdict; no resampling; also prints the extra figures.
- M3-18: The protocol also names the model id as the dry run's replies reported it, the effort setting, the two token caps, the band lines and Rust as the deciding baseline.
- M3-19: Sized at 18 runs per pass and three passes; it must report measured input and output tokens per run so the cap can be rechecked.
- M3-19b: Sized at 132 runs plus up to 36 for swaps.
- M3-20: Sized at 330 runs: 22 tasks, 3 languages, 5 repetitions.
- M3-23: An inconclusive verdict is decided by the owner from the written limits; the analysis must show the TypeScript gap even though it is not in the rule.

## M3-03 · Decide rules for reference defects, task swaps and late changes

### M3-03.1 After the owner tags the `run` commit, may anything be fixed and run again?

**Only harness faults rerun** (recommended). Nothing changes after the tag. A run broken by the harness itself (provider error after retries, refused reply, sandbox failure, machine crash, full disk) is void and run again from the same commit, with the fault and the rerun logged; everything else stands as it fell.

```
rulings file, one line per ruling:
run 0142  void     harness: provider error after 3 retries; rerun as 0331
task G2   dropped  tools and reference disagree, regression case cases/g2_slice
run 0207  tagged   reference gap: section 4.2 silent on empty list
```

- Why: A harness fault says nothing about any language, so rerunning it bends nothing; every other change after the tag could.
- Locks: The `run` commit is final for language, reference, tasks, tests and counting rules.
- Not chosen: Fix and rerun affected runs (A defect found during counted runs is fixed and the affected runs repeated on a new commit. Cleaner data, but it takes the usage again, comes late, and decided by people who have seen the first results.) · No reruns at all (Even a run cut by a provider outage counts as it fell. Simplest, but a network fault would be scored as a language failure.)

### M3-03.2 A counted run fails because the Paka tools and the reference disagree: the checker or interpreter does not do what the text says. What happens?

**Drop the task everywhere** (recommended). When such a mismatch changed the outcome of any run of a task, the task leaves the verdict in all three languages, whichever side is wrong; it needs a small program that shows the mismatch against a sentence of the reference. More than 3 dropped tasks of 22 makes the verdict inconclusive by rule.

- Why: Dropping the same task for all three languages keeps the comparison like for like and needs no new runs. The limit of three, and a table with nothing dropped, stop the rule from quietly removing Paka's bad tasks.
- Locks: The rulings file format M3-10 reads and the work of M3-21.
- Not chosen: It counts as it fell (The failure stays in Paka's numbers. Simplest and strictest, but one interpreter bug on a common construct could decide the project.) · Fix and rerun Paka (Fix the tool and repeat that task's Paka runs. It takes usage again, comes late, and Paka alone gets a second attempt.)
- Against: The strict view: a language is its toolchain, so tool bugs should count. The M2 conformance suite and the M3-16 reference solutions exist to make such drops rare; if more than three are needed, the tools were not ready and the verdict says so.
- Owner must still: Approve each dropped task in M3-21.

### M3-03.3 A counted Paka run fails because the reference was silent or unclear, while the tools do what the text says. Does the run count?

**It counts as it fell** (recommended). The run stays in the verdict. M3-21 tags it, and the report adds one table with tagged runs removed, marked as not the verdict.

- Why: The reference is part of what is tested: a small language that fits in 8,000 tokens is the bet (GENESIS 325, 366). The extra table lets the owner see how much such gaps cost without letting them move the rule.
- Locks: The reference's quality at the `run` tag is final for the verdict.
- Not chosen: Remove the run (Tagged runs leave the verdict. Deciding what is a gap in the reference is a judgement made after the results are known, by a party that wants Paka to pass.) · Fix reference and rerun (The reference is corrected and the task's Paka runs repeated. The reference would then be tuned on the counted tasks themselves.)

### M3-03.4 When is a counted task too easy or too hard to keep, as measured by the Rust and TypeScript calibration runs?

**Swap at 6 of 6** (recommended). A task is swapped when all six calibration runs (3 Rust, 3 TypeScript) pass first try, or when none of the six passes at all; the replacement is the unused reserve task of the same class, else the next in list order, calibrated the same way. The final list may be tagged `run` only if Rust's first-try rate on it is between 30% and 85% and TypeScript's is at most 85%.

```
task T2: rust 3/3 first try, ts 3/3 first try -> swap
task G1: rust 2/3 first try, ts 3/3 first try -> keep
task R3: rust 0/3 solved,    ts 0/3 solved    -> swap
kept list: rust first try 58% (30..85), ts 71% (<= 85) -> may tag `run`
```

- Why: It removes only tasks that showed no failure at all, by a rule a script can apply. The window is on Rust because Rust decides (M3-02): above 85% little is left to see, and near 20% Paka could not lose the first-try metric at all.
- Locks: What M3-19b does, and the size of the reserve list in M3-04.
- Not chosen: Swap at 5 of 6 (Also swaps a task with a single failed first try. Keeps only clearly hard tasks, but needs a larger reserve and treats one unlucky run as proof.) · No swaps, ceiling only (Tasks are never swapped; only the overall check applies. Less to do, but up to a third of the tasks could separate nothing.)
- Look again when: If more than six tasks qualify for a swap, or the window fails with the reserve used up, the list cannot be repaired from the reserve. New tasks then mean a new protocol and a new `protocol` tag before any counted run.
- Owner must still: Merge the amended task list into `development` yourself and tag `run`. If the window fails, decide whether to restart with new tasks; the old logs are kept and listed.

### M3-03.5 A chosen task turns out to be impossible, or just clumsy, to write in Paka. What happens to it?

**Fix bugs, swap gaps** (recommended). Found before the freeze, in M3-16: a bug (the tools disagree with the reference) is fixed through M3-16's defect task, and a task that cannot be written at all with the approved language and built-ins is swapped for a reserve task and listed in the report under 'Paka could not express'. Long or clumsy does not qualify, and more than two such swaps goes to the owner.

- Why: The experiment tests the writing risk (GENESIS 366); missing features are known without spending model calls, and listing them keeps the report honest. Only 'cannot be written at all' qualifies, or the eight tasks chosen to cost Paka (M3-04) could be swapped away.
- Locks: Little; it applies only before the `protocol` tag. Each such swap uses one of the six reserve tasks that calibration also draws on.
- Not chosen: Keep it, Paka fails (The task stays counted and Paka scores never-passing runs. Strict, but it measures a known missing feature, and M3-17 cannot be met without a passing Paka reference solution.) · Add the feature (The language grows to fit the task. Reopens M2 decisions under time pressure and makes the reference larger.)
- Owner must still: Decide in person if more than two tasks cannot be expressed.

### M3-03.6 Between the `protocol` and `run` tags, what may be changed using what the dry run shows?

**Wording yes, rules no** (recommended). The reference text may be reworded, corrected or trimmed to fit 8,000 real tokens, using evidence from practice tasks only, and only where the tools already behave that way. No language rule, checker behaviour, counted task, test, counting rule or stop rule changes, except harness fixes, calibration swaps and a tool bug fix with a regression case.

- Why: Any language's documentation is improved by watching readers trip; the counted tasks stay unseen, so the result cannot be fitted to them.
- Locks: After `run`, nothing. Before it, only wording.
- Not chosen: Frozen at protocol (Nothing about Paka changes after the first tag. Cleanest, but a misleading sentence found on day one stays in every counted run, and a reference measured over 8,000 tokens could not be trimmed as M3-19 requires.) · Anything until run (Language rules may change too. That is language design steered by a run of a few practice tasks, and M2's conformance work would have to be redone.)
- Owner must still: Merge each amendment into `development` yourself, then tag `run`.

### M3-03 · settled without asking

- Who rules on what: Harness faults: the harness flags them from its own logs; the owner sees the list. Mismatches and reference gaps: the M3-21 agent proposes from the transcript and the reference text, a second agent re-rules a sample blind, the owner approves. Inexpressible tasks: the M3-16 author files, its reviewer confirms, the owner decides the swap. Saturation: applied mechanically in M3-19b. Amendments: the owner merges each.
- Mismatch or gap: It is a mismatch only when a small program behaves differently from what a sentence of the reference says. Everything else about the reference is a gap, and the run counts as it fell.
- What a harness fault is: Provider error, a usage limit reached mid-run, refused or empty reply, lost network, sandbox failure, harness crash, full disk. A wrong program, a slow program or a reply cut off at the reply limit is not one.
- Runs sent to review: Every never-passing run in any language, every run in which a tool crashed or printed an internal error, and every run the harness flagged.
- Wrong test or unclear statement found after the run tag: If the statement text proves an expected result wrong, the task is dropped in all three languages and counts toward the same limit of three.
- Rust or TypeScript toolchain defect: Treated like a Paka mismatch: the task is dropped in all three languages.
- Harness fault limit: A void run is rerun at most twice from the same commit, inside the approved token cap. If the same run faults a third time, the experiment pauses and the owner is told.
- Sandbox or counting fault: A `run` commit never changes. If a test on a fake model proves the sandbox or the counting wrong during counted runs: stop, fix the harness only, tag a new `run` commit, run all counted runs again and publish the old logs. The owner says yes first, because it takes the usage a second time.
- Model id changes or disappears: All counted runs use the one pinned id. If the provider withdraws it mid-run, the experiment stops; a new id means a new `run` tag and all counted runs again, with the owner's yes, because it takes the usage again.
- Defects found before the protocol tag: Reference and tool defects found while writing reference solutions are fixed through M3-16's defect tasks; a reference change there needs a matching conformance case and the M2 exit check passing.
- Amendments: Each change between the tags is its own pull request that the owner merges, and the report lists it. After the last one, every reference solution in all three languages and the M2 exit check pass again before the `run` tag.
- Publication: All raw logs are merged, void and dropped runs included. The report lists every ruling, every amendment and every difference between the two tags.
- No rule fits: The run stands as it fell and the case is written up for M3-23. No new rule is made after the `run` tag.

### M3-03 · what this changes in TASKS.md

- M3-10: The rulings file has three kinds of line (void run, dropped task, tagged run); a fourth dropped task turns the verdict to inconclusive; the tool also prints the metrics with nothing dropped and the table with tagged runs removed.
- M3-16: A task Paka cannot express at all is swapped and listed, not fixed; only bugs get the level 4 defect task; long or clumsy is not a defect.
- M3-18: The protocol states the swap rule, the Rust window (30% to 85%), the TypeScript ceiling (85%) and that a failed window goes to the owner.
- M3-19: Reference changes after the dry run are limited to wording and trimming backed by practice-task evidence, one pull request each.
- M3-19b: Applies the 6-of-6 rule in both directions, then the window, and refuses to proceed outside it. Its done-when and exit check 1 cannot be met as written if the window fails with the reserve used up; that case goes to the owner, not to a seventh swap.
- M3-20: Reruns a void run at most twice, inside the cap, and pauses on a third fault.
- M3-21: Reviews the fixed set of runs named here and outputs only the three kinds of ruling.
- M3-22: Also lists tasks under 'Paka could not express' and prints the not-the-verdict tables.

## M3-04 · Choose counted, reserve and practice tasks hard enough to separate languages

### M3-04.1 How many counted, reserve and practice tasks are there?

**Thirty tasks, 22 counted** (recommended). 22 counted, 6 reserve and 2 practice tasks, so the six test-writing batches and three reference-solution groups in TASKS.md stand. 110 counted runs per language, at the cost of 300 hand-worked cases.

- Why: It is the plan TASKS.md was already cut for, and 110 runs per language keep chance movement at about two thirds of the 10-point band. Twelve tasks would give about 9 points, and each task would carry 8% of the verdict.
- Locks: Batch counts of M3-11 and M3-14 to M3-16, and the budget of M3-02.
- Not chosen: Forty tasks, 30 counted (30 counted, 8 reserve, 2 practice. Chance movement of the first-try gap falls from about 7 to about 6 points, for a third more task writing and model calls.) · Eighteen tasks, 12 counted (12 counted, 4 reserve, 2 practice. Much less writing, but 60 runs per language moves the gap by about 9 points, nearly the whole band.)
- Look again when: If more than six tasks need replacing (M3-03), the reserve is too small and the owner decides.

### M3-04.2 Are the tasks written fresh for this experiment or taken from public exercise sets?

**Written fresh** (recommended). Every statement is new, with its own exact output format and rules, so no solution can be recalled. Costs the writing and hand working already planned in M3-11.

- Why: All three languages then start from the statement alone, which is the comparison the paper wants.
- Locks: The task texts, frozen with the protocol.
- Not chosen: Taken from public sets (Known problems with known tests. Less writing, but the known languages can pass by recall, it needs a download, and such sets are the saturated kind (GENESIS 382).)

### M3-04.3 What makes a task hard?

**Many exact rules** (recommended). Each task is ordinary work with a long, exact statement: tie-breaks, rounding, error cases and an output format, checked by hidden cases on the edges. Reference solutions run 100 to 300 lines; it costs careful statements (M3-11).

```
statement extract (task R1, build order):
- Print one name per line in dependency order.
- When several may come next, print the one first in ASCII order.
- On a cycle print `cycle: a -> b -> a`, starting at its smallest
  name, and exit with status 20.
```

- Why: Detail is hard without being recallable, it is where compilers and silent defaults separate languages, and it resembles the work agents really do.
- Locks: The style of all 30 statements and the weight on M3-11's review for anything a statement leaves to the language.
- Not chosen: Classic hard algorithms (Short statements that need a clever method. Models know these from training in the familiar languages, so they pass by recall or fail for reasons unrelated to the language.) · Large programs (Fewer tasks of 800 lines or more. Hard for sure, but each run is long and costly, whole-file edits become clumsy, and the slow interpreter and the 32,000-token reply limit get in the way.)
- Look again when: If calibration finds more than six tasks to swap, this way of making tasks hard has failed for this model and the list is redone before any counted run (M3-03).

### M3-04.4 Does the list deliberately include tasks where Paka is expected to pay?

**Yes, eight tagged** (recommended). 8 of the 22 counted tasks are tagged as expected to cost Paka: three file tools, and one each needing a custom sort order, a priority queue, a tree type that contains itself, in-place grid changes, and mixed Int and Float maths. The tags are published before the runs; Paka's headline numbers will be lower than on a neutral list.

```
enum Msg { Start(src: String), Loaded(text: String), Saved }

fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(src)   => [ReadFile(src, then: Loaded)]
    Loaded(text) => [WriteFile("out.txt", tidy(text), then: Saved)]
    Saved        => [Print("done"), Exit]
  }
}
```

- Why: A third of the list aimed at known costs, declared in advance, is the plainest answer to the charge that the tasks were chosen to flatter Paka.
- Locks: The make-up of the list and what the report may claim about balance.
- Not chosen: Neutral tasks only (Avoid anything that leans on a known Paka weakness or strength. Sounds fair, but it hides exactly the costs the decisions said the experiment should see.) · Spread by class only (Pick by class and let weak spots fall where they may. Nobody can then say afterwards whether the list was kind to Paka.)
- Look again when: When M2-05, M2-09, M2-12 and M2-14 are approved: recheck each tag against the real rules (for example whether a sort takes a function) before the protocol tag.

### M3-04.5 How are the 22 counted tasks spread over the seven task classes of M2-01?

**Even spread** (recommended). Four text tasks and three in each of the other six classes; one reserve task in each class except text. A swapped-out text task is therefore replaced from another class.

```
text processing 4 | parsing and interpreters 3 | state machines 3
grids and dynamic programming 3 | graphs by list index 3
integer and float reports 3 | file tools 3
```

- Why: With thin evidence on which class matters most, equal weight is the choice that needs no argument and is easy to check.
- Locks: Class counts in the frozen list; the swap rule's same-class preference.
- Not chosen: Weighted to everyday work (More text, report and file tasks, fewer graph and grid tasks. Closer to daily agent work, but these are the classes most likely to be passed first try by every language.)

### M3-04 · settled without asking

- Counted, text processing (4): T1 wrap and justify: wrap paragraphs to a given width, spread spaces left first, split over-long words. Stresses index arithmetic. Rust may fail on unsigned underflow and off-by-one widths; TypeScript on off-by-one spacing no type catches. T2 column table: quoted delimited rows to an aligned table with right-aligned numeric columns and totals. Stresses telling numbers from text. Rust: mixed-column number parsing; TypeScript: `Number("")` and `"1e3"` count as numbers. T3 natural sort (tagged): sort lines with digit runs as numbers, case-blind, stable. Stresses a custom sort order. TypeScript: default sort is by text; Rust: comparator over mixed chunks. T4 run-length codec: encode and decode with escapes, report the position of malformed input. Stresses escapes and error positions. Rust: position bookkeeping; TypeScript: undefined past the end reads as text.
- Counted, parsing and small interpreters (3): P1 calculator: integer expressions with precedence, variables across lines, truncating division, errors with column. Stresses a recursive parser. Rust: a type that contains itself needs boxing, error plumbing; TypeScript: `/` gives fractions. P2 stack machine: a small assembly language with labels, jumps, calls, a step limit and defined errors. Stresses a dispatch loop with many defined errors. Rust: index type conversions; TypeScript: stack underflow gives undefined, then NaN. P3 nested config (tagged): parse a brace-nested format with strings, escapes and lists, print sorted flat key paths, exact first error. Stresses a tree type that contains itself. Rust: ownership of the tree; TypeScript: union types under strict mode.
- Counted, state machines and simulations (3): S1 lift simulation: timed requests, a fixed scheduling rule, an event log. Stresses rule order over many steps. Fails on rule order and tie-breaks in any language; Rust also on changing state while walking it. S2 ledger with holds: open, deposit, hold, capture, release, transfer, with rejection rules, in whole cents. Stresses many rejection rules on shared state. Rust: two accounts changed at once; TypeScript: cents parsed as fractions. S3 comment stripper: remove comments from C-like text with strings, escapes, nested block comments and line joins. Stresses a character-by-character state machine. Rust: look-ahead over characters; TypeScript: silent reads past the end.
- Counted, grids and dynamic programming (3): G1 line diff: longest-common-subsequence diff of two files with a fixed tie-break and hunk format. Stresses walking back through a table. Rust: unsigned index minus one; TypeScript: rows that share one array. G2 falling sand (tagged): move grains on a grid until stable under fixed rules. Stresses in-place grid changes. Rust: reading and writing the grid at once; TypeScript: shared rows. G3 packing: best value under a weight limit, with the chosen items and tie-breaks (fewest items, then names in order). Stresses rebuilding the answer from the table. Fails on reconstruction and ties in any language.
- Counted, graph algorithms by list index (3): R1 build order: dependency file to an order with ASCII tie-break; on a cycle print it in a fixed form and exit with the error status. Stresses a graph kept as lists by index. Rust: name-to-index maps and borrowing; TypeScript: Map lookups that may be undefined. R2 cheapest route (tagged): weighted routes with ties broken by fewest hops, then path order; unreachable case. Stresses a priority order with ties. Rust: the built-in heap is largest-first and the tie order must be built into it; TypeScript: no built-in queue, so a hand-made one or a careful scan. R3 islands and bridges: connected groups and the links whose loss splits one; inputs are small, so removing each link and recounting is enough. Stresses graph search with state restored between tries. Rust: search over shared arrays; TypeScript: stale state between tries.
- Counted, integer and float reports (3): N1 invoice: quantities, cent prices, percentage discounts, tax in hundredths of a percent, round half up per line. Stresses exact money rounding. TypeScript: fractional arithmetic on money; Rust: rounding negative values. N2 group statistics (tagged): mean, median and standard deviation per group to two decimals, with rules for empty and single groups. Stresses mixed Int and Float. Rust: ordering fractions; TypeScript: the default sort orders numbers as text; Paka: explicit Int to Float steps. N3 calendar: days between dates 1900 to 2100, weekday, add working days skipping listed holidays. Stresses date arithmetic. TypeScript: the built-in date type has zero-based months and rolls bad days over silently; Rust: no date type, all by hand.
- Counted, file tools (3, all tagged): F1 include resolver: follow nested include lines, each file once, name the chain on a cycle, write the merged file. The number of reads is known only at run time. F2 split by key: one input file to one output file per key plus an index file; file names are worked out at run time. F3 three-way merge: base, ours and theirs to a merged file and a conflict report, with the error status on conflict. All three stress many chained file steps. Rust may fail on error handling across many file steps; TypeScript on missing-file errors thrown instead of reported; Paka pays for chaining messages by hand.
- Reserve, in swap order (6): V1 parsing: validator and pretty-printer for a small data format with exact error positions. Stresses position tracking through nested input. Rust: line and column bookkeeping, boxing; TypeScript: reads past the end give undefined. V2 state machines: car-park billing from entry and exit events. Stresses rule order and minute arithmetic. Rust: a map entry changed while it is read; TypeScript: division gives fractions. V3 grids: edit distance with the printed alignment and a tie-break. Stresses a table walk-back. Rust: unsigned minus one; TypeScript: shared rows. V4 graphs: longest chain of dependent jobs with slack per job. Stresses an ordering pass and two sweeps. Rust: index maps and borrowing; TypeScript: undefined Map lookups. V5 reports: loan repayment table in whole cents with a corrected last payment; the payment amount is an input, because working it out needs a power function M2-01 leaves out. Stresses integer money and rounding. TypeScript: fractions; Rust: integer division and casts. V6 file tools (tagged): pack several files into one text archive and unpack it. Stresses many file steps and exact framing. Rust: error plumbing; TypeScript: thrown missing-file errors. Each is written, tested and solved in three languages like a counted task.
- Practice (2): X1 text: word counts with the top N words and tie rules. Stresses sorting by two keys. Rust: two-key comparator; TypeScript: default text sort. X2 file tools: join two files with line numbers into a written file, missing-file error with the error status. Stresses the written-file check and the error path. Rust: error plumbing; TypeScript: a thrown missing-file error. They exist to shake down the harness; their results never enter the verdict.
- Tasks outside every class: None. Every task names one M2-01 class.
- Cases per task: 2 visible and 8 hidden, each with written hand working: 300 cases in all. Hidden cases sit on the edges the statement names.
- Limits on inputs: ASCII only; files under 200 lines; every number the statement defines, and every value the plain method works out on the way, fits in plus or minus 2^53 - 1 so no language is tripped by number width; float inputs are chosen so no result sits on a rounding tie; recursion in the plain solution stays under 200 deep and well under the call-depth limit M2-06 sets, checked in M3-16. No clock, randomness, network or standard input.
- Error status: A defined error prints its message on standard output, which is compared, and exits with one fixed status, the same in every task. The number is set in M3-11 once M2-15 is approved: 20 if a Paka program may choose its status freely. It must not be 1 or 2 (the paka tool's own), 3 (a trap), 101 (a Rust panic) or 1 again (a crashing Node program).
- Tags under swaps: A reserve task that replaces a counted one brings its own tag; V6 is tagged, V1 to V5 are not. The report gives the final number of tagged tasks.
- Statement neutrality: Statements describe arguments, files, standard output and exit status only, and name no language feature and no grant. M3-11's reviewer checks that nothing is left to a language's default behaviour.
- Outlines to full statements: The outlines above are the approved list. The full statements and cases are written in M3-11 and may sharpen rules but not change what a task stresses or its class.

### M3-04 · what this changes in TASKS.md

- M3-11: Stays at six batches of five tasks; each task has exactly 2 visible and 8 hidden cases and follows the 'many exact rules' style, the input limits and the one error status.
- M3-14: Stays at three groups of ten; the list of expected Rust failure causes per task gives the author what to watch.
- M3-15: Stays at three groups of ten; with all stated values inside 2^53 - 1, wide integers are no longer its hardest part, silent fractions and text-order sorting are.
- M3-16: Stays at three groups of ten; it must confirm or correct each of the eight weak-spot tags and V6's against the approved M2 rules, and check recursion depth against the interpreter's limit.
- M3-18: The protocol carries the 30 outlines, their classes, the swap order of the reserve list, the tags and the error status number.
- M2-14: Should know that tasks T3 and R2 depend on whether sorting takes a custom order and whether a priority queue is built in; either answer is fine, but the tags follow it.
- M2-15: Should know the task list needs three things from headless apps: an exit status the program chooses (not 0, 1, 2 or 3), file names worked out at run time (F2), and a number of reads and writes known only at run time (F1, F2).
- M2-06: Should know the tasks recurse up to about 200 deep; the interpreter's call-depth limit must sit well above that.

## M4-01 · Fix the generator benchmark method, test definitions, program size and winning margin

### M4-01.1 What does the stopwatch cover when a generator compiles one definition?

**Core form to first call** (recommended). Start: the checked core form (the small internal form the interpreter runs) of one definition is in memory. End: its machine code has returned the right value from a first call through its slot (the table entry callers jump through). Costs a small lowering per generator that must exist before anything is timed.

```
t0  core form of the definition in memory
    lower it to the generator's input
    compile
    make the code runnable, write the slot
    call through the slot, compare the result
t1
```

- Why: It is the generator's whole share of the dev path in line 302, so the number can be set against the 100 ms budget directly. Start-up is gated once as a separate figure (see defaults).
- Locks: The harness M4-01b builds and the figure the generator rule reads. Nothing in the language.
- Not chosen: Compile call only (Times only the generator's compile function on input built beforehand. Cheapest to measure, but hides building the input and making code runnable, which is where the two differ most.) · Fresh process each time (Every sample starts a process, compiles once and exits. It measures start-up, which a long-lived tool server pays once.)
- Look again when: M4-04 rules that every run compiles in a fresh process. Then the fresh-process figure is the one that matters.

### M4-01.2 How do the same three definitions reach each generator?

**Two lowerings from core form** (recommended). The three definitions are written in Paka, checked by the real front end, and lowered by a small throwaway translator per generator, covering only the node kinds these definitions use. Costs a few hundred lines of code that are thrown away after the choice.

```
three .paka definitions
  -> parser, checker, core form (M2, already built)
  -> lowering A -> Cranelift builder calls
  -> lowering B -> LLVM IR text
both: every check is a compare and a branch,
      a trap is a status return, calls go through the slot table
```

- Why: The core form exists by M4 (M2-23), so each generator gets the real shape and the same checks by construction. The translators are small because the definitions avoid strings, lists and records.
- Locks: Little. The translators live in the benchmark crate outside the workspace and are thrown away; M4-26 writes the real lowering.
- Not chosen: Hand-written input per generator (No lowering code. A person or agent writes each definition once per generator. A 200-line function written twice by hand is slow to write and nothing shows the two are the same work.) · Neutral instruction list (A third made-up form, written once per definition, with a translator to each generator. It avoids the core form but measures a shape Paka will never emit.)

### M4-01.3 What goes into the three test definitions, and how long are they?

**6, 40, 200 lines, numbers** (recommended). Three functions of about 6, 40 and 200 canonical lines using Int, Float, Bool, enums without fields, if, while, match, calls to other definitions through slots and calls to stand-in runtime functions. No strings, lists or records, because their memory layout is not decided until M4-05.

```
enum Mode { Up, Down, Hold }

fn step(x: Int, m: Mode; by: Int) -> Int {
  match m {
    Up => x + by
    Down => x - by
    Hold => x
  }
}
```

- Why: To a code generator a string or list operation is a call plus a check, and both are in the mix. Fixed sizes can be written down now, which is the point of the task.
- Locks: The three texts, their hand-written expected results, and the meaning of small, medium and large in M4-44 and the exit check.
- Not chosen: With strings and lists (The same sizes, but the bodies also index lists and build strings. More like real code, but the benchmark must invent layouts and runtime functions that M4-05 has not decided and may contradict.) · Sizes from M3 solutions (Small, medium and large are the median, 90th-percentile and largest definition in the M3-16 Paka reference solutions. Based on real Paka, but it fixes nothing today and experiment tasks are small one-file programs.)
- Look again when: The M3-16 reference solutions hold a definition longer than 200 lines, or M4-44b's profile shows code generation time is driven by something these bodies lack.

### M4-01.4 Is each generator fed and set the way Paka would really use it, or tuned to its best time?

**As Paka would use it** (recommended). Cranelift gets builder calls with optimization off and hands back code bytes, which the benchmark makes runnable itself, as M4-23 later will. LLVM 21 gets IR text, the form paka build already emits (M1-18), through its ORC JIT (LLVM's in-process compiler) with optimization off, one module per definition. LLVM pays for reading the text; in return only about fifteen C functions are declared by hand.

```
Cranelift: cranelift-codegen + cranelift-frontend only,
           opt_level=none, verifier off when timed,
           code bytes -> mmap, copy, mprotect (hand-declared libc)
LLVM 21:   IR text -> parse -> ORC LLJIT (llvm-c, libLLVM-21.so),
           code generation level None, no passes,
           one module per definition, looked up at once
both:      every other setting at its default; one context reused
```

- Why: If LLVM won, the plan would reuse the one text emitter for dev and release; that is its whole attraction, so that is the route to time. It is also the route with the fewest unsafe declarations.
- Locks: Little. The settings are listed flag by flag in the method document and may not change after the first timing.
- Not chosen: LLVM through builder calls (LLVM's input is built call by call, with no text. Faster for LLVM, but several times more unsafe declarations and a second emitter beside the text one.) · LLVM both ways (Both feeding routes are built and the better one counts. Fairest to LLVM, at double the LLVM work.)
- Look again when: Cranelift fails the gate and LLVM then misses it by less than its own measured parse time. Then M4-01b reruns LLVM once through builder calls before recording.
- Owner must still: Allow `unsafe` in the benchmark crate when M4-01b starts. The download is settled: you allowed the Cranelift crates on 2026-10-13, and M4-01b shows each crate by name, version, source and size before it fetches. Nothing to install for LLVM: libLLVM-21.so and the llvm-c-21 headers are already on the build machine.

### M4-01.5 What decides which generator wins?

**Budget gate, then default** (recommended). A generator passes if it gives right results and stays within the time limits in the defaults (20 ms warm on the large definition, 50 ms for a first compile in a new process). If one passes, it wins. If both pass, Cranelift is recorded. If neither passes, the faster on the large definition is recorded as provisional.

```
pass = right results
   and warm p95 (large) <= 20 ms
   and first compile in a new process <= 50 ms
one passes   -> that one
both pass    -> Cranelift
neither      -> lower warm p95 (large), marked provisional
```

- Why: Speed past the budget buys nothing (line 79), so the tie goes to the generator with a safe Rust interface that returns plain code bytes, which is what the installer task M4-23 assumes; LLVM brings a C++ JIT that owns its memory and can abort the tool server. The task plan already assumes Cranelift (M6 notes).
- Locks: The dev-mode architecture: M4-05, M4-06, M4-23, M4-26 and the lowering tasks M6-07 to M6-15 follow the recorded generator.
- Not chosen: Fastest wins outright (The lower 95th-percentile time on the large definition wins by any amount. Simplest to state, but lets a difference nobody can see choose the dev-mode architecture.) · Half again faster wins (LLVM must beat Cranelift by 1.5 times on the large definition to win; otherwise Cranelift. Keeps speed in play, but still needs a default and adds a second number to argue about.)
- Against: Making LLVM the tie-winner would give dev and release one lowering, shrink M6-07 to M6-15 and add no outside crate, which fits the owner's standard-library-first rule; its library is already on the machine. The price is hand-declared unsafe bindings with no Miri and a JIT inside the long-lived server.
- Look again when: Both fail the gate, or M4-44b later shows the generator taking more than a fifth of the edit-to-running time.
- Owner must still: M4-01b's record is the rule applied to the numbers; the owner signs that the rule was followed.

### M4-01.6 Is the LLVM side built and timed when Cranelift already passes the gate?

**Cranelift first, LLVM if needed** (recommended). Cranelift is built and timed first. If it passes the gate it is recorded and no LLVM code is written. If it fails, the LLVM side is built by the same method and the rule is applied to both. Costs a record with one generator's numbers in the likely case.

```
$ paka-bench generators
cranelift  small m/p95  medium m/p95  large m/p95  cold  PASS
llvm       not built: cranelift passed the gate
recorded   cranelift
```

- Why: Nothing is built before it is needed, and the unsafe code is written only in the case where its number can decide something. The method stays fixed for both, so adding LLVM later is the same work and no new decision.
- Locks: Little. The LLVM half of the method stays written down and can be run at any later time.
- Not chosen: Always measure both (Both sides are built and timed whatever Cranelift shows. Both numbers go on record, at the price of level-4 unsafe code written for a figure the rule ignores whenever Cranelift passes.)
- Against: The task as cut has M4-01b print both generators' timings, and a record with both numbers answers a later doubt without new work. It would also show what the one-lowering route costs in time.
- Look again when: Cranelift fails the gate (then LLVM is built at once), or M6 planning wants the LLVM JIT figure before M6-07 starts.

### M4-01.7 How big is the program on which M4-44 times edits?

**5,000 definitions** (recommended). About 100,000 lines at 20 lines a definition, the size of a large application. The size is an argument of the benchmark command, so other sizes cost one more run.

```
$ paka-bench edits --definitions 5000
```

- Why: It is the proposed figure, it is large enough to expose any step that grows with program size, and it is one argument to change.
- Locks: The number in exit check item 1.
- Not chosen: 1,000 definitions (Easier to pass and quicker to generate. Smaller than the programs dev mode is meant to stay fast on.) · 50,000 definitions (A stress size. No evidence that an agent-built program reaches it, and every benchmark run takes ten times longer.)
- Look again when: Real Paka programs after M5 are far larger, or M4-45 can only pass by a trick that works at exactly this size.

### M4-01.8 Which edits does M4-44 time, and which must pass 100 ms?

**Body edits gated, rest reported** (recommended). Body edits to the small, medium and large definition must pass. A signature edit with 10 direct users, an add and a delete are timed and printed but cannot fail the check.

```
gated:    body-small  body-medium  body-large
reported: signature-10-users  add  delete
```

- Why: It gates exactly what line 67 promises and still records the cost the paper calls proportional, for the price of three more loops in one command.
- Locks: The edit classes in M4-44's output and in the exit check.
- Not chosen: Body edits only (Nothing else is timed. Least work, but the cost of a signature edit stays unknown until it hurts.) · Gate signature edits too (A signature edit also gets a limit. The paper gives none, so the number would be invented.)

### M4-01 · settled without asking

- Full text of the three definitions: Written by the M4-01 agent to the recipe above, in the M4-47 layout, with hand-written expected results for fixed inputs, and checked by its reviewer. They are committed before the benchmark crate exists.
- Gate figures: Warm: 95th-percentile time on the large definition at most 20 ms, one fifth of the 100 ms budget; the rest is left for parsing, checking, the database, installing and starting the run. Cold: first compile of the large definition in a new process at most 50 ms, so M4-04 stays free to pick short-lived processes.
- Warm figure: samples and statistic: Per definition: 20 untimed compiles, then 200 timed ones, each of a fresh copy with one constant changed so nothing is cached. Median and 95th percentile are printed. The whole run is done three times in three processes and the worst 95th percentile counts; no further reruns.
- Cold figure: what it covers: From the generator's first call in a new process (building its settings or its JIT) to the first call of the large definition returning the right value. Parsing and checking the Paka source are outside it. One untimed process first, then 20 timed; the second slowest counts, so one stray stall cannot decide.
- Machine state and clock: The build machine, otherwise idle, one thread, Rust's monotonic clock. When both generators are timed they run in the same session, interleaved by definition.
- Correctness before timing: A timing counts only if all three compiled definitions return the hand-written results, including one input that must trap on overflow and one that must trap on division by zero. Every check is a compare and a branch that returns a trap status, as M4-26 will require; division by zero and the smallest Int divided by -1 are tested before the divide, so no instruction faults by itself. Each generator's own verifier must pass once, untimed.
- Provisional calling convention: For the benchmark only, thrown away when M4-05 decides. A compiled definition takes the slot table's address, a place for its result and its arguments, and returns a status: 0, or a trap number. An enum without fields is an integer tag. Calls to other definitions and to stand-in runtime functions load their address from the slot table.
- Versions: LLVM 21, the version of the installed clang and libLLVM. Cranelift at the newest release that builds with the pinned Rust toolchain 1.96.1, fixed in the benchmark crate's own lock file.
- Reported but not part of the rule: Run time of the compiled large definition on a fixed input, peak memory, number of outside crates and number of unsafe lines, for each generator measured. They are printed beside the timings so a later reader can see them.
- Both generators fail: The one with the lower warm time on the large definition is recorded as provisional. M4-45 inherits the gap, and the record says the 100 ms target is at risk.
- How M4-44 takes one sample: One sample is one edit request that also asks for a run (M4-03's shape). The clock starts when the benchmark writes the request and stops when the reply with the run result has been read. The app's first handler calls the edited definition, prints its result and exits, so the figure slightly overstates edit-to-first-handler and never understates it.
- Shape of the benchmark program: Written by a small fixed-shape generator inside the M4-44 command, not by M4-21's fuzz generator, so the same count always gives the same program. It is a directory with one file per definition (M4-48): the three test definitions, one app with its types, and filler functions of about 20 lines of the same numeric kind in a fixed call tree. Every definition is reachable from the app through calls.
- How each edit class edits: A body edit changes one constant, so the code really changes. The signature class makes a change that enters the interface hash and still checks at all 10 users; M4-44 picks it once M4-11 has fixed that hash. Add writes a new definition nobody uses; delete removes it.
- Samples per edit class in M4-44: 200 edits per class, median and 95th percentile, three runs, the worst 95th percentile counts for the exit check.
- Where the method and results live: The method is one document merged before the benchmark crate's first commit; the caller checks that order. M4-01b commits the printed timings and the recorded generator beside it.

### M4-01 · what this changes in TASKS.md

- M4-01b: Reworded: Cranelift is built and timed first, with one throwaway lowering from the core form; the LLVM side (an IR-text lowering and about fifteen hand-declared C functions) is built only if Cranelift fails the gate. Its done-when becomes: one command reprints the timings of each generator measured, and the record is the fixed rule applied to them.
- M4-44: Reworded: program size is a command argument with 5,000 as the gated value, six edit classes are printed and three are gated, timing is from request written to reply read. It gains its own small fixed-shape generator that writes a one-file-per-definition directory, so it still does not need M4-21.
- M4-46: Exit check item 1 reads 5,000 definitions as fixed, not proposed, and uses the worst of three runs.
- M4-06: Unchanged in size, but its condition is now mechanical: it runs if the rule records Cranelift and is dropped if it records LLVM.

## M4-03 · Propose the agent tool interface: transport, message shapes, versions, find queries

### M4-03.1 How do an agent and the tools talk?

**JSON lines on stdio** (recommended). `paka serve <program>` runs as long as the session. Each request is one line of JSON on its input and each reply one line on its output, with an `id` echoed back. Needs only the standard library; a harness that speaks another protocol needs a thin adapter.

```
$ paka serve wc/
> {"v":1,"id":1,"call":"find","name":"count_*"}
< {"v":1,"id":1,"ok":true,"found":[{"name":"count_words","kind":"fn","sig":"fn count_words(text: String) -> Int","version":"46b0f3eaf6de"}],"more":0}
```

- Why: It is the least to build and test with hand-written transcripts, which is all M4 needs. The message shapes do not depend on the channel, so an adapter or a socket can be added later without changing them.
- Locks: The envelope: `v`, `id`, `call`, `ok`, `error`. The channel itself is cheap to change.
- Not chosen: MCP on stdio (The same four calls offered through the Model Context Protocol, the tool protocol most agent harnesses already speak. No adapter, but a handshake, a tool listing and a result wrapper must be built, and Paka's promise is tied to an outside specification that keeps changing.) · One command per call (No server: `paka find`, `paka view` and so on each start, answer and exit. Simplest to use from a shell, but every call reloads the program and nothing can keep compiled code or a running app alive.)
- Look again when: The first real agent harness is connected. If it speaks only MCP, add an adapter that wraps these shapes. If two separate harnesses must edit one checkout, add a socket.

### M4-03.2 What is the base version an edit names?

**Hash of the canonical text** (recommended). The first 12 hex digits of the SHA-256 of the definition's canonical text, comments included. It can be recomputed from the files at any time. If text goes from A to B and back to A, an edit based on A is accepted again, which is harmless.

```
"version":"46b0f3eaf6de"
```

- Why: It needs no stored state, it is per definition as the task demands, and M4-07 already supplies the hash.
- Locks: The version string in every stored transcript and in any agent's memory of a session. It changes if the canonical layout changes.
- Not chosen: Counter per definition (A number that goes up with each stored edit. Short, but it lives only in the database and is lost when the database is deleted.) · One program revision (A single number for the whole program. Simplest, but any edit anywhere makes every other agent's base stale, against line 147.)

### M4-03.3 What does one edit call change, and does it run the program?

**One definition, optional run** (recommended). `base` is the state before and `text` the state after; null means absent, so the same call replaces, adds or deletes. An optional `run` field asks for a run right after the edit, in the same reply. The server remembers no run settings, so the agent must ask each time.

```
> {"v":1,"id":3,"call":"edit","name":"count_words","base":"46b0f3eaf6de","text":"fn count_words(text: String) -> Int {\n  len(split(text, \" \"))\n}","run":{"app":"wc","args":["notes.txt"]}}
< {"v":1,"id":3,"ok":true,"name":"count_words","version":"6c4c8968c9e4","text":"fn count_words(text: String) -> Int {\n  len(split(text, \" \"))\n}","errors":[],"broke":[],"installed":true,"run":{"stdout":"4 words\n","stderr":"","status":0,"trap":null,"stopped":null}}
```

- Why: One round trip gives edit plus result, nothing happens that the request did not ask for, and the server keeps no settings between calls.
- Locks: The fields of edit and of the run result. A signature change that breaks users needs no batch: the edit is stored and installed when the users check too (line 306).
- Not chosen: Edit never runs (Running is always a separate call. Cleanest split, but every edit-and-see costs two round trips and the 100 ms path spans two requests.) · Server reruns every time (After a first run call the server reruns with the same arguments after every accepted edit. Closest to the paper's wording, but file writes are repeated without being asked for and the server holds hidden state.)
- Against: The paper says a headless program is rebuilt and rerun after each edit (lines 74, 126), which is 'Server reruns every time' and M4-39's first wording. Here the rerun happens only when the request asks for it.
- Look again when: Agents are seen to need several definitions changed in one step that single edits cannot reach.

### M4-03.4 What does a find query by type shape mean?

**Takes and gives, exact** (recommended). `takes` lists types the parameters must include, in any order; `gives` is the result type. Each type is Paka type text and must match exactly; either field may be left out. No ranking and no wildcards, so a near match is not found.

```
> {"v":1,"id":4,"call":"find","takes":["String"],"gives":"Int"}
< {"v":1,"id":4,"ok":true,"found":[{"name":"count_words","kind":"fn","sig":"fn count_words(text: String) -> Int","version":"6c4c8968c9e4"}],"more":0}
```

- Why: It answers the two questions an agent has (what can I do with this type, what makes one) with a rule that fits in one sentence of the reference. Wildcards can be added later without breaking a query.
- Locks: The names `takes` and `gives` and the promise that results are exact.
- Not chosen: Signature with wildcards (The query is a signature pattern in which `_` stands for any type, matched in order. More exact about position, but the agent must guess the argument order and a pattern language must be specified and tested.) · Ranked loose match (Near matches are returned in order of closeness. Finds more, but the result is no longer exact and the ranking rule is one more thing to keep stable.)
- Look again when: M2-09 allows agent-written generic functions; then a rule for type parameters in a query is needed.

### M4-03.5 What gives way when a view does not fit its token budget?

**Drop signatures, never the body** (recommended). The body is always whole. Signatures of what it uses are added in the order the text first names them while the reply still fits; `more` counts the ones left out. If the body alone is over budget the call fails and says how many tokens it needs.

```
> {"v":1,"id":2,"call":"view","name":"update","budget":220}
< {"v":1,"id":2,"ok":true,"name":"update","kind":"fn","version":"970bec26c31d","text":"fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {\n  match m {\n    Start(path) => [ReadFile(path, then: Loaded)]\n    Loaded(text) => {\n      s.words = count_words(text)\n      [Print(\"{s.words} words\"), Exit]\n    }\n  }\n}","uses":[{"name":"Tally","kind":"type","sig":"type Tally { words: Int = 0 }","version":"bb18fead002b"},{"name":"Msg","kind":"enum","sig":"enum Msg { Start(path: String), Loaded(text: String) }","version":"b2cd465ef2e0"}],"more":1,"errors":[]}
```

- Why: The reply never exceeds its budget, as M4-18 requires, and the agent can get a missing signature with a find by name or a larger budget.
- Locks: The meaning of `budget` and of `more` in a view.
- Not chosen: Cut the body (The body is shortened to fit and marked as cut. Always answers, but the agent cannot safely replace a definition it has seen only in part.) · Return all, report size (The budget is advice: everything is returned with its token count. Never fails, but a view can blow the agent's context, against line 32.)

### M4-03 · settled without asking

- Interface version: Every request and reply carries `"v":1`. Adding an optional field or a new call does not change the number; removing a field or changing a meaning does. A request with an unknown number gets a `bad_version` error naming the numbers the server speaks.
- Unknown or malformed input: A field the call does not know is refused with `bad_request` naming it, so a misspelt field never passes silently (line 33). A line that is not a JSON object gets `bad_request` with `"id":null`.
- Which program: The server is started on one file or directory and serves that program only, so requests name no program. A second server on the same program is refused by a lock file held with the standard library's file lock, which the system frees when the process dies. If a file of the program does not parse, the server prints the errors and does not start.
- Files changed behind the server: The server assumes it alone writes the program's files. After git or a person changes them, restart it; the derived database is rebuilt from the text (line 275). Agents working in parallel use separate checkouts and merge through git.
- Order and framing: UTF-8, one JSON object per line, no line breaks inside an object. Requests are handled one at a time in arrival order and each reply echoes the request's `id`.
- Form of the schema: One table per call in the interface document: field, JSON type, required or not, meaning. No schema language and no generated code; M4-16's hand-written transcripts are the test.
- Failure shape: `{"v":1,"id":6,"ok":false,"error":{"kind":"stale_base","message":"count_words has moved"}}`. Kinds: bad_request, bad_version, unknown_call, not_found, stale_base, name_taken, parse, not_one_definition, name_mismatch, budget, no_app, not_runnable. A stale_base error also carries the current `version` and `text`, so the agent need not ask again; a parse error carries `errors` that point into the text as sent.
- When an edit is stored: Refused, and nothing stored, if the base is stale, the name already exists on an add, or the text does not parse as exactly one definition with the call's name. Otherwise it is stored even with type errors or broken users (lines 306, 321-323).
- Edit reply fields: `name`, `version` (null after a delete), `text` (the stored canonical text), `errors` (this definition's own), `broke` (the exact names of other definitions that no longer check because of this edit), `installed`, and `run` when a run was asked for. `installed` is true when the runnable version, the last one in which every definition checks, now includes this edit. If it is false no run happens and `run` is null.
- What a version hashes: The UTF-8 bytes of the canonical `text` exactly as a reply carries it: comments included, no line feed after the last line. The version strings on this page are real. The M4-03 agent checks them and any new ones with the system's sha256sum, not with Paka's own hash code (TASKS rule 5).
- Error objects: M2-04's JSON object with the same keys. Line, column and offsets count from the start of the `text` in the same reply, because the agent never sees the file, so the file key is null. For text that did not parse they count in the text as sent.
- Find query fields: `name` (exact, or with `*` standing for any run of characters), `users_of` (definitions that directly use the named definition, variant or built-in), `takes` and `gives`, `kind`, `failing` (true lists definitions that do not check now). Fields given together must all hold.
- Shape query details: Type text in a query is parsed and printed in canonical form before it is compared, so spacing does not matter. `inout` is not part of a type. A type listed twice in `takes` asks for at least two such parameters. Only the program's own fn and view definitions are matched: built-ins are generic and stand in the reference. Leaving out `gives` means any result.
- Find results: Sorted by name in byte order. Each entry has `name`, `kind`, `sig`, `version`. For a type or enum `sig` is its whole canonical text (line 136), so `version` is the hash of `sig`; for the rest it is the header line without the brace. A built-in has kind `builtin` and a null version. An optional `budget` (default 1,000 tokens) cuts the list; `more` counts what was left out and `after` (a name) continues from there.
- View details: `budget` defaults to 2,000 tokens. `uses` holds entries of the same four fields, in the order the definition's text first names them, signature first; built-ins are left out, since the reference travels with every task (line 325). `errors` holds the definition's current errors, so a definition named in `broke` can be fixed after one view. In the sample the reply is 563 bytes, 188 tokens; the third signature would make 223, over the budget of 220.
- Comments beside a signature: Not shown in version 1: the `sig` of a fn, view, app or example is the header only (line 315 asks for signatures), and its comments are read with a view of that definition. This is the ruling M2-02.8 left to the tools. A comment field can be added later without a version change.
- Token counting: M2-18's rule, applied to the whole reply line: bytes divided by 3, rounded up, until M3 records real tokens.
- Run call and result: `{"v":1,"id":7,"call":"run","app":"wc","args":["notes.txt"]}` runs the last version in which every definition checks; if there is none yet, the error is `not_runnable`. Reply: `{"v":1,"id":7,"ok":true,"run":{"stdout":"4 words\n","stderr":"","status":0,"trap":null,"stopped":null}}`. `app` is always required. `trap` carries the fields M2-06 fixes; exit statuses are those of M1-06 and M2-15, which also rules how `args` reach the first message.
- Run limits: Optional `limit_ms` (default 10,000) stops a run that takes too long: `stopped` is then "time" and `status` is null. Optional `max_bytes` (default 16,000) cuts each output stream and sets `stopped` to "output".
- Grants, directory and input of a run: The run uses the app's grants as written, with no approval step: the paper gates merge and release, not dev runs (lines 167, 187). It runs in the server's working directory with empty standard input. If M2-15 lets apps read input, a `stdin` field is added without a version change.
- Example exchanges: The samples on this page form one session in id order: find by name (1), view (2), edit with run (3), find by shape (4), a refused edit (6), run (7). Built-in names in them (`len`, `split`) are placeholders until M2-14. The M4-03 agent adds one exchange per error kind and per find field before M4-16 starts.
- One definition per edit: No batch call. Several edits are sent one after another; the last fully checked version keeps running in between (line 306).
- Look (M5): Not part of version 1. M5-09 is still to be decided; it should add its calls as new `call` values in this envelope, which needs no new version number.

### M4-03 · what this changes in TASKS.md

- M4-16: Reworded: the transport is JSON lines on standard input and output with a `v` check on every request and no handshake; unknown fields are refused; one server per program is kept by a standard-library file lock.
- M4-17: Reworded: the base version is the 12-digit hash of canonical text; null `base` adds, null `text` deletes; a stale base returns the current version and text; text that does not parse is refused with kind `parse`.
- M4-18: Grows slightly: name patterns with `*`, the `users_of`, `kind` and `failing` fields, paging with `after`, the `more` count and current errors in a view. It now needs M4-15, because view and find print canonical text and take the version from it.
- M4-19: Shrinks to an exact takes-and-gives match over the program's own fn and view definitions, with no wildcards or ranking; it may be re-graded to level 2.
- M4-39: Reworded: a run happens only when an edit carries a `run` field or a run call is made; the server keeps no run settings; time and output limits come from the request. 'A broken edit leaves the old result' becomes: an edit that is not installed returns `run` null, and a run call still runs the last fully checked version or answers `not_runnable` if there is none.
- M4-44: Reworded: one timed sample is one edit request with a `run` field.
- M5-09: Should shrink: the Look calls are added to this envelope and error shape instead of defining their own.
- M5-14: Reworded: views travel through find, view and edit with kind `view` and no new fields.
- M7-08: Reworded: items awaiting approval ride in edit replies as a new optional field of this envelope, which needs no version change. They are not errors, and M7-01 fixes their shape.

## M4-47 · Decide the canonical layout and how tools treat other layouts

### M4-47.1 Does the layout depend on line width, or only on the structure of the code?

**Structure only, no width** (recommended). Where a line breaks depends only on what the construct is, never on how long the line gets. Long lines stay long. The printer is a plain walk over the tree.

```
type Model { items: List<Item> = [], draft: String = "", filter: Filter = All, next_id: Int = 1 }
```

- Why: It is the smallest printer, and a one-word edit never reflows other lines, so the stored text changes only where the code did. Nobody reads the long lines (line 11), and it gives the fewest bytes per view (line 315).
- Locks: Every stored definition, every golden file and every base version: a later change of layout reprints all of them at once.
- Not chosen: Wrap at 100 columns (A list that does not fit in 100 columns is broken one item per line, as most formatters do. Familiar look, but the printer needs a fitting algorithm and a one-word edit can reflow many lines.) · Always one item per line (Every argument, field and list item gets its own line. No width rule either, but the most lines for every view an agent pays for.)
- Look again when: A tool or model in use mishandles very long lines, or the M3 logs show models misreading one-line type definitions.

### M4-47.2 Which constructs break across lines?

**Blocks broken, lists flat** (recommended). A block of statements, a match and an app block always break: one statement, arm or key per line, two spaces deeper, wherever the block stands. Everything in round or square brackets, and the fields of a type and the variants of an enum, stay on one line. A one-statement if takes three lines, and a comment on one field cannot stay beside it.

```
enum Job { Queued, Running(id: Int), Done }

fn is_idle(j: Job) -> Bool {
  match j {
    Queued | Done => true
    Running(_) => false
  }
}
```

- Why: Two rules cover the whole language, and they keep the paper's one-line type and enum shapes (lines 201-203). A type's whole definition is its signature (line 136), so flat types make find and view results one line each.
- Locks: The printer in M4-15, the view printer in M5-11, and the look of every sample in the reference.
- Not chosen: Members one per line (Type fields and enum variants also go one per line. Closer to Swift and Kotlin habit and a comment can sit above its field, but every type and enum takes more lines and the paper's one-line shapes change.) · Short blocks stay inline (A block with one statement stays on one line. Fewer lines, but a block then has two shapes and the rule for which one applies must be learnt and tested.)
- Against: A model leans on comments (line 55), and with flat types a note written on one field is lifted above the whole type, where it no longer says which field it means. One member per line would keep it in place.
- Look again when: Agents are seen to write notes on single fields of wide types. Then types and enums alone break one member per line.

### M4-47.3 Where does a comment sit in canonical text?

**Own line, above** (recommended). Every comment is a whole line directly above the definition, statement, match arm or app key it belongs to; a comment already on its own line stays where it is. A comment after code moves above that line, and a comment inside a flat list moves above the statement or definition that holds the list. Text and order are kept, but a note on one field or argument loses its place.

```
sent     sum += x // running total

stored   // running total
         sum += x
```

- Why: One place for a comment keeps one layout, and no comment text is lost. It also keeps flat lists flat.
- Locks: How the parser attaches comments and what M4-15's printer must carry.
- Not chosen: Trailing comments stay trailing (A comment after code stays at the end of its line. Position is kept exactly, but the printer carries two comment positions, and a comment inside a flat list would force that list to break.)
- Look again when: The same as for the point above: notes on single fields turn out to matter.

### M4-47.4 What does a tool do with text that parses but is in another layout?

**Accept, store canonical** (recommended). Edit takes any layout that parses, stores the canonical text and returns it. paka check judges meaning only and says nothing about layout. View and find always print canonical text, whatever is on disk. Costs that hand-made files may sit in git in another layout.

```
$ paka check old_style.paka
$ echo $?
0
```

- Why: Everything an agent sees and everything the tools write is canonical, which is what line 51 is for and what line 273 asks of the files, and no existing file or expectation has to change. Tightening later is one mechanical reprint, so this is the cheap side to start on.
- Locks: Little. Tool versions are taken from the canonical print and the database's hashes from the parsed definition, never from raw file bytes, so a file in another layout cannot confuse them.
- Not chosen: Check rejects other layouts (paka check fails on a file that is not canonical. One layout is then enforced on disk, but every stored test source must be reprinted and the line and column in its hand-written expectation redone by hand.) · Check rewrites files (paka check reprints any file it reads. Files are always canonical, but a command named check then changes sources under the person's hands.)
- Look again when: Files in mixed layouts cause real trouble, for example noisy merges between agents' branches. Then add a format command and a CI check.

### M4-47.5 Do tools quietly fix harmless wrong spellings: a stray semicolon, return on the last line, wrong letter case?

**Errors with a fix** (recommended). The tools never turn text the language rejects into accepted text. The three stay errors that carry their exact fix (line 322): text that still parses is stored with the error like any type error, text that does not parse is refused. Costs the agent a second edit each time.

```
count.paka:4:3: error[return-last]: return on the last line
  fix: replace `return total` with `total`
```

- Why: It needs nothing new and matches the paper's model of report plus fix (line 322). Loosening later breaks no program; tightening later would.
- Locks: Nothing new. M3-01 can record that these three cost a counted edit.
- Not chosen: Tools apply safe fixes (Edit applies fixes that cannot change meaning (semicolon, last-line return) and returns the corrected text. Saves round trips, but the tools then accept a second spelling, and whole-file runs such as the M3 experiment would need the same behaviour to be fair.) · Accept both spellings (The checker accepts the other spellings and the printer normalises them. No error ever, but there are then two spellings on input, against line 51.)
- Against: An edit replaces a whole definition (line 316), so one stray return on a 200-line function is paid for by sending all 200 lines again. Applying the fix would save exactly the dearest kind of round trip.
- Look again when: The M3 raw logs show these three errors taking more than about one in twenty of Paka's counted edits.

### M4-47.6 What happens to sources stored before the layout existed?

**Leave them as stored** (recommended). Nothing is reprinted. Old sources keep working because check accepts any layout. From now on hand-written samples, the language reference included, are written in the canonical layout, and M4-15 tests that the reference samples come out of the printer unchanged.

- Why: Nothing breaks and nothing is built. Test sources in assorted layouts are also free tests of the parser.
- Locks: Nothing.
- Not chosen: Reprint everything once (One pull request reprints every stored source. All files then look alike, but every expectation with a position is redone by hand, and the raw experiment logs must still be left alone.) · Reprint all but tests (Samples and reference solutions are reprinted, test sources are not. A middle road that needs a format command built just for it.)

### M4-47 · settled without asking

- What the printer may change: Only white space and where a comment sits. It never reorders, renames, adds or removes anything else, parentheses included: `(a * b) + c` stays as written.
- File basics: Two spaces per open block, no tabs, no spaces at line ends, lines end with a line feed, one line feed at the end of the file.
- Blank lines: Exactly one blank line between definitions and none inside a definition. A comment block sits directly above what it belongs to, with no blank line between.
- Sample: type and enum: One line each, items separated by a comma and a space, one space inside the braces, empty braces closed up.
type Counter { count: Int = 0 }
type Empty {}
enum Msg { Start(path: String), Loaded(text: String) }
- Sample: fn and example: Header on one line, body always broken, closing brace on its own line. An empty body is `{}` on the header line.
fn clamp(x: Int; lo: Int, hi: Int) -> Int {
  max(lo, min(x, hi))
}
example clamp_low {
  clamp(0 - 5, lo: 0, hi: 9) == 0
}
- Sample: app: One key per line, one space after the colon, no alignment, keys in the order written.
app wc {
  state: Tally
  start: Start
  update: update
  grants: [ReadFiles, Console]
}
- Sample: let, var, assignment, discard: One statement per line.
let bonus = 10
let n: Int = 0
var sum = 0
sum += x
_ = pop(&q)
- Sample: if, else, for, while: Bodies always broken; else stays on the line of the closing brace.
if x < 0 {
  return x
} else if x == 0 {
  hits += 1
} else {
  sum += x
}
for x in xs {
  sum += x
}
while i < n {
  i += 1
}
- Sample: match: One arm per line, one space each side of `=>` and of `|`, no alignment of arrows. An arm whose body is a block breaks like any block.
match m {
  Start(path) => [ReadFile(path, then: Loaded)]
  Loaded(text) => {
    s.words = count_words(text)
    [Print("{s.words} words"), Exit]
  }
}
- Sample: calls, literals, records, indexing: Always on one line; a comma is followed by one space; no space inside round or square brackets.
clamp(n, lo: 0, hi: 9)
[Print("done"), Exit]
Counter { count: 1 }
Counter {}
append(&g.next[a], b)
- A block inside an expression: A block breaks wherever it stands; the brackets or the `=` around it add no break of their own.
let y = if c {
  1
} else {
  2
}
- Spacing in signatures and types: `name: Type`, `-> Int`, `List<Int>`, `Map<String, Int>`, `s: inout Tally`. The label mark has no space before it and one after: `(x: Int; lo: Int, hi: Int)`.
- Operators: One space each side of a binary operator and of `=`, `->` and `=>`; none after `&` or another one-operand operator. Example: `a * b + c`.
- Strings, numbers, holes, names: Printed exactly as written: `"{s.count}"`, `0.0`, `??reset`. The printer never changes a name or a literal.
- return, break, continue: Each on its own line: `return 0`, `break`, `continue`.
- Alignment: None anywhere. The paper and DECISIONS samples that line up arrows and app keys mean the same and are simply printed with single spaces.
- Comment on its own line: Stays where it is, also as the last line of a block before the closing brace. The parser attaches every comment where the printer will put it, so printing and parsing again gives the same tree.
- Comment after the last definition: A comment below the last definition, at the end of an edit's text or of a file, joins the end of the comment block above that definition. Left below, it would pass to a definition added after it (M2-02.8). A tool that appends to a file with such a comment moves it first.
- Views (M5): A trailing block is a block: one child per line, as the paper writes it (lines 218-227). M5-11 confirms the detail when views are decided.
Col(gap: 8) {
  Text("{s.count}")
  ??reset
}
- Constructs decided later: Error-handling forms, ranges, function values, generics, wrapping arithmetic and migrate follow the same two rules: a block breaks, a list is flat. The task that decides each one adds its sample.
- Order of definitions in a file: Kept as stored. The printer never sorts or moves definitions.
- What an edit rewrites on disk: Only the edited definition and its comment block. Neighbours in the same file keep the layout they had.
- A format command: Not built. The printer is reached through edit, view and find. A command is added when a person or script first needs to reprint files.
- Text that does not parse: Has no canonical form. Edit refuses it with errors that point into the text as sent.

### M4-47 · what this changes in TASKS.md

- M4-15: Shrinks to a plain tree walk with no width fitting and no precedence logic, since parentheses are kept as written. It takes over attaching comments in the parser, in the place the printer puts them, so the round trip is exact. The paka check case is one outcome (accepted, silent), and it gains a test that the reference samples print unchanged.
- M2-18: Reworded: every sample in the language reference is written by hand in this layout from the start, and the reference says in one sentence that a comment is stored on its own line above what it describes.
- M3-01: Nothing to raise from M2-02's open note: the three harmless spellings stay errors with a fix and each costs a counted edit.
- M4-17: Reworded: edit accepts any layout that parses, stores canonical text for the edited definition only, and leaves the rest of the file alone.
- M4-11: Reworded: body and interface hashes are computed from the parsed definition, never from raw file bytes, so layout cannot change them and the task still does not need the printer.
- M5-11: Shrinks: views follow the existing block and flat-list rules; only the trailing-block sample is new.

## M4-48 · Decide a definition's full name and the file tools store it in

### M4-48.1 What is a definition's full name?

**The bare name** (recommended). The full name is the name as written in the source, for example `count_words` or `Tally`. This works because all definitions of a program share one pool and a duplicate is an error (DECISIONS M2-02). The stable id is this name; a rename makes a new id.

```
count_words
Tally
wc
dec_stops_at_zero
```

- Why: It is already unique, it is the shortest thing an agent can write, and a prefix can be added in front later without changing any name inside a program.
- Locks: Stable ids, base versions, cache keys and the name field of every tool call.
- Not chosen: Kind and name (The kind is part of the name. Every tool call gets longer and turning a fn into a view becomes a delete and an add, for no gain, since two kinds cannot share a name anyway.) · File and name (The file is part of the name. Moving a definition to another file renames it and breaks stored versions, although the unit is the definition, not the file (line 71).) · Program and name (A program prefix on every name, ready for packages. It designs half of an open question (line 358) before anything needs it.)
- Look again when: Packages are designed and one program must name a definition in another.

### M4-48.2 In a directory program, which file does a new definition land in?

**One file per definition** (recommended). A new definition goes into its own file named after it. Two agents adding different definitions never touch the same file, and an edit rewrites one small file. Costs many small files: 5,000 for the benchmark program.

```
wc/
  Tally.paka
  Msg.paka
  update.paka
  count_words.paka
  wc.paka
```

- Why: It keeps the paper's conflict rule true at the level of git with no choice left to the agent. The id does not depend on the file, so this is cheap to change later.
- Locks: Little: moving definitions between files later changes no id, version or hash.
- Not chosen: Caller names the file (The edit call says which file. The agent must make a grouping choice that means nothing, and two agents appending to one file conflict in git.) · One default file (Every new definition is appended to one file such as main.paka. Nothing to choose, but the file grows without limit and its end is a conflict point for every pair of agents.)
- Look again when: A platform whose file names ignore letter case is planned (line 132 says Linux first): `Msg.paka` and `msg.paka` would then collide. Also if start-up on thousands of small files proves slow in M4-44b.

### M4-48 · settled without asking

- Packages: Stay undecided through M7, as the M2 notes in TASKS.md already say. One program is one pool of bare names; nothing in the name form assumes how programs will depend on each other.
- Single-file programs: A program given as one file stays one file. A new definition is appended at its end after one blank line. No tool splits a file or turns it into a directory.
- Existing files with several definitions: Allowed and left alone. An edit rewrites the definition where it is; no tool moves a definition to another file.
- Delete: Removes the definition's text and its comments. A file left with no definition is deleted.
- Rename: There is no rename operation. It is an add under the new name and a delete of the old one; the users of the old name are reported as broken until they are edited.
- Position inside a file: A definition added to an existing file goes at the end. Order has no meaning and is never changed by a tool.
- File name: The definition's name exactly as written, plus `.paka`, directly in the program's directory. Names are ASCII letters, digits and underscore, so every name is a valid file name on Linux. An add whose file name would pass Linux's 255-byte limit is refused.
- A file of that name already exists: If `foo.paka` is there and holds other definitions, the new `foo` is appended to it. No tool renames or splits a file.
- Variants, fields and labels: Not definitions and have no id. A variant is found by its bare name, which is unique in the program, and the tools answer with the enum that holds it. Fields are shown as part of their type.
- Built-in names: Live in the same pool, have no file and cannot be edited. Find reports them with kind `builtin`.
- Examples and apps: Named and stored like any other definition: `dec_stops_at_zero.paka`, `wc.paka`.
- Kind in the id: Not part of it. A definition that changes kind under the same name keeps its id; its interface hash changes, so its users are rechecked.

### M4-48 · what this changes in TASKS.md

- M4-08: Shrinks: the stable id is the bare name alone, with no kind and no qualifier.
- M4-17: Reworded: an add in a directory program writes a new file named after the definition, a delete removes an emptied file, and text whose name differs from the call's name is refused.
- M4-14: Reworded: the rename edit kind is generated as an add followed by a delete.

## M5-01 · Decide what a view is and what its body may contain

### M5-01.1 How does a view write the children of an element?

**Child block after element** (recommended). An element that can hold children takes a brace block after its arguments. Each line in that block that gives an element adds one child, in order. Costs one new block form in the parser, checker and printer.

```
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
```

- Why: It is the paper's spelling and the shape models know from SwiftUI and Compose (line 436), and children read top to bottom with no brackets or commas. The must-use rule stays whole: in a child block an element line is used, as a child.
- Locks: The grammar and canonical layout of every view (M5-11), how views lower to the core form (M5-16), and every view in the reference. Moving to lists later rewrites every view.
- Not chosen: Children as a list (Children are an ordinary List argument. No new block form, but brackets and commas pile up, the paper's sample changes, and a loop over rows needs function values or a hand-built list.) · Plain function, no view (There is no `view` kind; a view is a `fn` that returns an element and children are a list. One definition kind fewer, but lines 118 and 265 are reversed and elements become ordinary values, which the third point turns down.)
- Look again when: M2-03 rules that `"{s.count}"` is not the text-building spelling; only the samples change, not this pick.

### M5-01.2 What may a child block contain besides element lines?

**let, if, for, match** (recommended). A child block holds element lines, view calls, holes, `let`, `if`/`else`, `for` and `match`; the bodies of those are child blocks too and add their elements in order. No `var`, assignment, `while`, `break`, `continue` or `return`. Costs one more rule in the reference.

```
view todo_view(s: Todos) -> View<Msg> {
  Col(gap: 8) {
    Input(s.draft, on_change: Draft, key: "new")
    Button("Add", on: Add)
    if len(s.items) == 0 { Text("Nothing to do") }
    for t in s.items {
      todo_row(t, key: t.id)
    }
  }
}
```

- Why: These four cover both samples, and anything heavier goes into an ordinary `fn` the view calls. Allowing more statements later breaks no program; taking them away later does.
- Locks: The statement set the parser accepts in a child block (M5-11), the checker rules (M5-13), and the key rules in M5-02, which rely on `for` being the only loop. A `match` in a child block still names every variant; an arm that adds nothing is written `Done => {}`.
- Not chosen: Any statement (A child block is an ordinary block; element lines add children and everything else is allowed. Nothing extra to state, but a view can hold counters and `while` loops, and the tree's shape can no longer be read from the block's shape.) · Element lines only (A child block holds only elements; conditions and repetition are special elements that take a function. Smallest block rule, but it needs function values that capture variables, which M2-09 has not decided, and two more built-in names.)
- Look again when: A real view needs a running total across loop rounds that a helper `fn` cannot give cleanly. If M2-14 names the list-length function otherwise, only the sample changes.

### M5-01.3 Is an element an ordinary value that any code can build, store and pass around?

**Only views, only in place** (recommended). Elements, child blocks and view calls may appear only in a view, as the view's result or as a line of a child block. A `fn` cannot build or return one, and an element cannot be put in a `let`, a list, a state field or an argument. Costs: a view cannot take ready-made children from its caller.

```
view badge(n: Int) -> View<Msg> {
  Text("{n} left")
}

// rejected: element built outside a view
// fn badge(n: Int) -> View<Msg> { Text("{n} left") }
```

- Why: Every element then has one visible place in the source, which is what keys, ids, holes and per-definition swapping stand on. Loosening later is free; tightening later breaks programs.
- Locks: `View<Msg>` appears only as a view's result type. The checker rejects elements elsewhere (M5-13). A child block is read only where an element may stand, which is what keeps `if cond {` unambiguous. A wrapper view that takes children (a card around any content) cannot be written yet.
- Not chosen: Elements are ordinary values (`View<Msg>` is a type like any other; functions return elements, lists hold them, views take them as arguments. Most freedom, but ids can no longer follow the source, and element-position holes need a second rule.)
- Look again when: Two or more sample apps need a wrapper view that takes children from its caller.

### M5-01.4 Does a view give exactly one root element, or any number of elements?

**Exactly one root** (recommended). A view body is zero or more `let` lines and then one element expression: an element, a view call, a hole, or an `if`/`else` or `match` whose every branch ends in one. The body is not a child block. Costs a wrapping `Col` or `Row` when a view wants to give two things.

```
view main_view(s: State) -> View<Msg> {
  match s.screen {
    Login => login_view(s)
    Home  => home_view(s)
  }
}
```

- Why: It reuses the decided last-expression rule: a view body is a block whose value is one element. One call, one element keeps keys and ids simple.
- Locks: The shape of every view body and the meaning of a key on a view call (M5-02). Going to many roots later changes how keys on calls work.
- Not chosen: Any number of elements (A view body is a child block and a call splices all its elements into the caller. No wrapper needed, but a call may add zero or many children, so a key on the call names no single element and the app's root needs its own rule.)
- Look again when: Sample apps keep needing a wrapping container only to give two siblings, and M5-05's layout makes that wrapper change what is drawn.

### M5-01.5 May a view call another view, and may the two have different message types?

**Plain call, same messages** (recommended). A view is called like a function, by its lower-case name, wherever an element may stand. The called view's result type must be the caller's `View<Msg>` exactly, so one app has one message enum. A view may call itself. Costs: no reusable part with its own private messages.

```
view todo_row(t: Todo) -> View<Msg> {
  Row(gap: 8) {
    Button(mark(t.done), on: Toggle(t.id))
    Text(t.title)
  }
}
```

- Why: It is checked from the signature alone, adds no new construct, and gives the agent small pieces to replace one at a time (line 114). Message mapping can be added later without breaking any program.
- Locks: One message enum per app; view names live in the one pool of lower-case names; a view call depends on the called view's signature in the who-uses-what index (M5-14).
- Not chosen: Call with message mapping (A called view may use another message enum, and the caller wraps its messages, as Elm does. Allows self-contained parts, but needs one more built-in and function values, which M2-09 has not decided.) · No view calls (Only the app's one view exists; every row is written inline. Nothing to check across views, but one definition holds the whole screen, so every edit swaps the whole screen and line 114's piece-by-piece growth is lost.)
- Look again when: M2-09 lands with function values and a sample app needs a part reused with its own messages.

### M5-01.6 How does a visual app get its first state and its first frame?

**Defaults, then optional start** (recommended). The first state is the state type built from its field defaults (`Counter {}`), so a visual app's state type is a record with a default on every field; the checker says which is missing. If the app has a `start:` line, that message is handled next. Then the view runs and the window opens with the first frame.

```
type Todos { draft: String = "", items: List<Todo> = [] }

app todo {
  state:  Todos
  update: update
  view:   todo_view
  grants: []
}
```

- Why: The paper's counter is already written this way (lines 201, 213) and it needs no new line or definition. M5-29's reset already expects the state to show its defaults, so launch and reset share one fresh value.
- Locks: The state type of a visual app is a record with a default on every field. The launch order: state, start message, view, window. Effects asked by the start handler finish after the window is open, so the view must draw the not-yet-loaded state. The comma between fields in the sample follows the M2 grammar and is not fixed here.
- Not chosen: An init function (The app block names a function that returns the first state. Fields need no defaults, but it adds a line to every app and a second way to say what a fresh value is.) · Start message always required (A visual app must have a `start:` line like a headless one, and the handler fills the state. One rule for both kinds of app, but it reverses M1-06.3 and the paper's counter gains a message that does nothing.)
- Look again when: M2-15 rules that a headless app's first state is made another way; both kinds of app should then follow one rule.

### M5-01.7 What happens when the person closes the window?

**Closing ends the app** (recommended). Closing the window ends the program with status 0; no message is sent and effects not yet performed are dropped. An `Exit` effect also ends it, as in a headless app. Costs: an app cannot save or ask 'are you sure' on close.

```
$ paka run counter.paka
(window closed by the person)
$ echo $?
0
```

- Why: Neither sample saves anything, and a window that always closes cannot be broken by app code. A `close:` line can be added later without changing any existing app.
- Locks: Little. The windowed run loop (M5-35) ends on window close with status 0. What happens to an effect caught half-way follows whatever M2-15 rules for `Exit`.
- Not chosen: Close sends a message (The app block names a message sent when the person asks to close; the app ends only when `update` returns `Exit`. Lets an app save first, but an app that forgets `Exit`, or traps in that handler, cannot be closed.)
- Look again when: The first app that writes files on its own needs to save before it ends.

### M5-01 · settled without asking

- What a view is: A `view` is a definition kind (line 118). It is pure: it reads its arguments, calls ordinary functions and other views, and gives one element. It cannot change state or ask for effects.
- View signature: Written in full as the paper has it: `view name(params) -> View<Msg>`. `View<...>` is a built-in name used only as a view's result type; the message enum is always written inside it, so a call is checked from the signature alone.
- View parameters: The view named in the app block takes exactly one parameter, the state, read-only. Other views take any read-only parameters by position or label under the M2-02 rule. No `inout` parameter and no `&` argument in a view.
- App wiring: The view named in the app block takes the app's state type and gives `View<M>`, where M is the message type of `update`. Checked from the three signatures alone.
- Spelling of element calls: Element name, parentheses always written (even when empty), then a child block if and only if the element takes children; a container writes its block even when empty (`Col() {}`), so there is one spelling (line 51). Which elements take children, and whether any argument may be left out, is M5-04's table; M2-02 gives user functions no default arguments.
- Where a child block is read: Only where an element may stand: after an element's arguments on a line of a child block or at the end of a view body. In a condition, an argument or a `let` value a `{` after a call never starts a child block, so `if len(xs) == 0 {` and `if m == Toggle(3) {` read as before.
- Events: An event argument such as `on:` takes a message value (`on: Dec`, `on: Toggle(t.id)`). An event that carries data takes a variant name, as `then: Loaded` does at line 248. No closures are assumed. Each element's events are M5-04's.
- Holes in a view: `??name` where an element may stand has type `View<Msg>` and draws a labelled placeholder (line 75). A hole anywhere else in a view, such as inside an argument, traps like any other hole. What status a drawn hole gives an example is M5-06's.
- Trap in a view: As line 160: the last frame that drew stays, whichever view trapped, and the trap is reported through the tools. If the very first frame traps, the window opens showing one error node with the trap message.
- Element names: Elements are Upper-case built-in names in the one pool of names, as M2-02.2 already decided, so a user type or variant may not be called `Text`, `Row` or `Button`. The names are taken from every program, headless ones included, once M5-12 adds the table; that can break an earlier program that used one.
- Shape of update in a visual app: Either form from the paper: no result when it needs nothing from outside, or `-> List<Effect<Msg>>` (line 263). Results of effects arrive as messages and cause a new frame.
- When the view runs: Once after launch, then after every message whose handler finished without a trap and after every code swap; never inside a handler, and swaps happen only between handlers and frames (line 305). The runtime compares the new tree with the old one; the rules are M5-02's.
- Launch details: If the start handler traps, the state stays at its defaults and the first frame is drawn from them. The trap is reported through the tools.
- Window title and size: The title is the app's name. The first size is M5-05's ruling.
- Recursion: A view may call itself (a tree of folders). Running out of stack is a trap in the view like any other.
- Both sample views: The counter view is the sample of point 1, unchanged from the paper. The to-do view is the sample of point 2 and its row view the sample of point 5; its types, messages and update are written out in M5-03.

### M5-01 · what this changes in TASKS.md

- M5-04: Grows: the paper's `list` element cannot be named `List` (taken by the list type), each element is marked as taking a child block or not, and the table must say whether element arguments may be left out although user functions have no defaults.
- M5-11: Reworded: the parser reads a child block only where an element may stand, accepts in it only element lines, view calls, holes, `let`, `if`, `for` and `match`, and reads a view body as `let` lines plus one element expression.
- M5-13: Grows by four checks: elements only inside views and only in element position, same message type on view calls, a visual app's state type is a record with a default on every field, and exactly one root per view.
- M5-12: Grows slightly: element names enter the built-in names table for all programs, headless ones included, and any earlier test case that uses one as its own name is renamed in a commit before the code, citing M2-02.2.
- M5-20: Reworded: the windowless loop follows the launch order (state from defaults, start message if any, first tree), ends on `Exit`, and holds one error node if the first view run traps.
- M5-35: Grows slightly: closing the window ends the program with status 0 and sends no message.
- M5-07: Shrinks slightly: an explicit reset can use the fresh value this task defines, the state type built from its field defaults.
- M5-23: Reworded: the to-do sample follows the shapes shown here: one message enum, a `todo_row` view called in a `for`, draft text in the state.

## M5-02 · Decide key rules: who needs one, scope, tree-wide ids, duplicates

### M5-02.1 Outside a loop, which elements must carry a key?

**Elements that hold state** (recommended). An element marked in the element table as holding its own state (the text input now, scrolling later) must be given `key:`, the last labelled argument. Any other element may carry one and need not. Checked from the element table alone.

```
Col(gap: 8) {
  Input(s.draft, on_change: Draft, key: "new")
  Button("Add", on: Add)
}
```

- Why: It is the paper's rule and it is what lets element state survive code edits that move lines. It costs a key only where losing one would lose something.
- Locks: The needs-key mark in the element table (M5-04, M5-12), the `key:` spelling in every program, and the checker rule (M5-13). Requiring more keys later breaks programs; requiring fewer does not.
- Not chosen: Every element (Every element and view call carries a key. Ids never depend on position, but each line costs tokens and the paper's counter sample (lines 218-227) is rejected.) · No key needed (Keys are optional everywhere and an element without one is known by its position. Fewest tokens, but a code edit that shifts positions silently moves or drops what the element holds, against line 123.)

### M5-02.2 Which elements made inside a `for` must carry a key?

**Every loop child keyed** (recommended). Every element or view call that a `for` body adds to the surrounding child list carries a key, including ones inside an `if` or `match` in that body; a hole is exempt. Checked from the view body alone. Costs a key on rows that hold no state, and a body that adds two things per round needs two different keys or a wrapping container.

```
for t in s.items {
  todo_row(t, key: t.id)
}
```

- Why: A view call is checked from its signature alone, so the checker cannot see whether a row holds a text field; keying every loop child is the one rule that is checkable at the loop and never lets state land on the wrong item. A row is usually one view call, so the cost is one argument per loop.
- Locks: Every `for` in every view. Ids of loop rows are their keys, so UI examples and screen readers see stable names for rows. Relaxing later is free.
- Not chosen: Position unless stateful (Loop children follow the same rule as elsewhere: only stateful elements need keys, the rest go by position. But a text field inside a row view keys itself the same in every row, so its state follows the row's position, not the item. A silent wrong result.) · Key on the loop (The loop names the key once and each round is its own scope, so the body may add several children. Fewer keys to write, but a loop round becomes a level in every id without being an element, which the tree comparison must carry.)
- Look again when: Views with two children per loop round turn out common enough that the wrapping container is a real cost.

### M5-02.3 Within what scope must a key be unique, and how does that give every element an id unique in the whole tree?

**Siblings; id is path** (recommended). A key must differ only from the keys of the same parent's other children. An element's tree-wide id is the list of steps from the root; a step is the element's key or, for an element without one, the number the next point gives. Costs: an element that moves to another parent gets a new id.

```
Col                      id: (root)
  Input key="new"        id: "new"
  Button "Add"           id: #1
  Row key=7              id: 7
    Button "done"        id: 7 / #0
    Text "buy milk"      id: 7 / #1
```

- Why: It is the paper's scope, it needs no hand-built key strings, and two agents writing two views cannot clash (line 147).
- Locks: How ids look to UI examples (M5-06), Look (M5-09) and AccessKit (M5-39), and the comparison in M5-17.
- Not chosen: Unique in whole tree (Every key is unique across the app, like an HTML id, and is the id. Simple to address, but each row's inner keys must be built by hand from the item (`"edit-{t.id}"`), and a clash between two far-apart views shows only when both are on screen.) · Unique within one view (A key must be unique inside the view definition that writes it; ids are view-call path plus key. Clashes are found per definition, but a view call becomes a level in the tree that is not an element.)
- Look again when: M5-06 finds that examples need to address an element without knowing its path.

### M5-02.4 How is a child without a key told apart from its siblings?

**Place in the source** (recommended). The step is the number of the line that wrote the child, counted from 0 among all lines of that child block that give an element, through `if`, `match` and `for` bodies. It cannot change while the code is the same. Costs: the compiler numbers the lines, every tree node carries the number, and a code edit that adds a line renumbers the lines after it.

```
Col(gap: 8) {
  if s.warn { Text("Check the name") }
  Row(gap: 8) { Input(s.name, on_change: Name, key: "f") }
  Row(gap: 8) { Input(s.mail, on_change: Mail, key: "f") }
}
// steps: Text #0, first Row #1, second Row #2, whatever s.warn is
```

- Why: Loop children all carry keys, so every other child comes from one line, and that line's number is a step that cannot shift while the program runs. Counting at run time lets a warning that appears above a field take the field's focus or hand it another field's cursor, with no error, which is what line 123 sets out to prevent.
- Locks: The step shown in every id (M5-06), a number on every tree node (M5-15, M5-16, M5-24, M6-18a) and the matching in M5-17. Changing the numbering later changes ids in every UI example.
- Not chosen: Count at run time (The step is the child's count among the parent's unkeyed children in this frame. Nothing for the compiler to do, but a child that appears or goes shifts every later one, in release builds too.)
- Against: The author counted at run time: the tree alone then gives every id, hand-built test trees need no numbers, and no back end can disagree on numbering.
- Look again when: Dev-mode edits that add a line, and so renumber its later neighbours, visibly drop cursor or focus often; the tools could then keep a stable number per line.

### M5-02.5 What does a key on a view call mean?

**Call key names the root** (recommended). Every view call accepts `key:` without the view declaring it; the key becomes the key of the view's root element. Inside a view the root carries no key of its own, and the root may not be an element that needs one (wrap it in a `Row` or `Col`). Both are checked in the view body alone.

```
view todo_row(t: Todo) -> View<Msg> {
  Row(gap: 8) {
    Button(mark(t.done), on: Toggle(t.id))
    Text(t.title)
  }
}
// caller: todo_row(t, key: t.id)
```

- Why: An element has at most one key, the loop rule stays checkable from the loop, and nothing but elements exists in the tree. Moving to the third option later would change ids but break no program.
- Locks: `key:` is accepted on every element and every view call, and no view may declare a parameter named `key` (functions still may). A view's root cannot be a bare text input.
- Not chosen: View passes key itself (A view call takes no key; the view takes an ordinary parameter and puts it on its root. Nothing new in the language, but the loop rule can no longer be checked where the loop is written, only when it runs.) · Call is its own level (A view call adds its own step to the id path, and the root may also carry a key. No rule about roots, but every element can then carry several steps of identity and the tree comparison must match on all of them.)
- Look again when: The wrapper around a stateful root shows up as a repeated annoyance in sample apps.

### M5-02.6 What kinds of value may a key be?

**Int or String** (recommended). A key is an Int or a String. `3` and `"3"` are different keys. Costs: an item identified by two values needs a key built as text.

```
Input(s.draft, on_change: Draft, key: "new")
todo_row(t, key: t.id)
```

- Why: It matches the map keys the tasks may rely on, covers fixed names and item numbers, and both have an obvious text form. Widening later breaks nothing.
- Locks: The text form of ids in M5-06 must tell an Int key, a String key and a place number apart.
- Not chosen: String only (Every key is a String. One type and one text form, but every loop over items with a number id writes a conversion.) · Any comparable value (Any value with equality may be a key, records included. Most freedom, but it leans on equality for user types, which M2-09 has not decided, and such keys have no obvious text form in an id.)
- Look again when: M2-09 and M2-14 allow record keys in Map; keys should then follow the same rule.

### M5-02.7 What is shown when two siblings carry the same key?

**One error node, in place** (recommended). All siblings sharing the key are replaced by one error node at the place of the first; it names the key and the count. Other siblings draw as usual and the app keeps running. It is not a trap, so no state is rolled back; the element state held under that key is dropped. The tools report it as an error.

```
Col
  Input key="new"
  Error "duplicate key 7: 2 children of Col"
  Row key=8
```

- Why: It is what the paper says, it points at the fault where it is, and the rest of the app stays usable while the agent fixes it. One error node per duplicated key keeps the sibling ids unique.
- Locks: An error node kind in the runtime tree (M5-15) and the duplicate cases in M5-17. Whether a UI example fails when its tree holds an error node is M5-06's.
- Not chosen: Replace all the siblings (The whole child list of that parent is replaced by one error node. Simplest comparison code, but one bad row hides the whole list and the input above it.) · Treat as view trap (A duplicate is a trap in the view, so the last good frame stays (line 160). No new node kind, but it reverses line 123 and the screen silently stops following the state.)

### M5-02.8 When is the state an element holds thrown away?

**Gone from tree, gone** (recommended). State is kept from one frame to the next only for an element that is matched: same step under a matched parent, and the same kind (the element's name). Anything not matched loses its state at once, with everything inside it; the same key appearing later starts fresh. This covers removal, a move to another parent, a change of kind (its own or of an element above it) and a duplicate.

- Why: One rule, no hidden store, and nothing stale can come back. State worth keeping across a disappearance belongs in the app's state, which is the paper's model (lines 118, 435).
- Locks: The comparison in M5-17 and the focus rule in M5-18. With M5-03's pick the text itself lives in the app, so what is lost is only cursor and selection.
- Not chosen: Kept while hidden (State is parked under its id when the element disappears and returns if the id comes back. A hidden panel keeps its cursor, but parked state grows without bound and can return after the data it belonged to has changed.)
- Look again when: Scrolling is built and people lose their place when a panel is hidden and shown. Also if M5-08 rules that elements sit on an existing toolkit that keeps element state its own way.

### M5-02 · settled without asking

- Spelling of a key: `key:` is a labelled argument, written last, on any element or view call. Its value is any Int or String expression. It is the one argument every element may leave out unless the table marks it as needed.
- Duplicates the checker can see: Two lines that stand directly in the same block with the same literal key are rejected at check time. Lines in different branches of an `if` or `match` may share a key, since only one branch runs. All other duplicates are found when the tree is built.
- Hole in a loop: A hole line in a `for` body needs no key, so a skeleton can be written first (line 121). Its placeholders are told apart by place number and round; they hold no state. The element that replaces the hole must carry a key.
- The root: The root element has the empty path as its id. It carries no key, so the app's whole view cannot be one bare text input.
- Focus: Focus is not element state; the runtime holds one id. Focus stays while that element is matched from frame to frame under the last point's rule and is dropped otherwise. Which elements can take focus is M5-04's.
- Code swap: After a swap the view runs again and the same matching applies. An edit that adds or removes a line renumbers the unkeyed lines after it in that block, so cursor or focus held in or under them can be lost or mixed up; the text never is, since the app holds it (M5-03). A key on the container holds it still across such edits.
- Hole becomes an element: A placeholder holds no state. The element that replaces it starts fresh.
- Reset and migrate: An explicit reset of app state drops all element state and focus; a migrate keeps them under the same rule as any frame. M5-07 owns the rest.
- Handler trap: A trap rolls back app state only (line 159). Element state is not written by handlers, so there is nothing to roll back; the view is not re-run and the tree stays.
- Two screens with the same key: If a `match` at the root swaps one screen for another, both roots have the same kind, and both hold an element of the same kind and key at the same path, it keeps its cursor and selection. Give the two different keys when that is not wanted.
- Nested loops: Children of an inner loop join the same sibling list as the outer loop's, so their keys must be unique across both. Wrapping each outer round in a keyed container gives the inner loop its own scope.
- Text form of ids: Left to M5-06. It must tell an Int key, a String key and a place number apart; the forms shown here are illustrations.

### M5-02 · what this changes in TASKS.md

- M5-04: Grows: the table marks which elements hold state and so need a key, and every element accepts `key:` as its last labelled argument.
- M5-06: Reworded: examples address an element by its path of keys and place numbers, and the tree text form must show keys, place numbers and the duplicate-key error node.
- M5-11: Grows slightly: `key:` is parsed on view calls as well as on elements, though no view declares it.
- M5-13: Grows by five checks: key present on stateful elements, key on every loop child except a hole, no key on a view's root and no key-needing root, no view parameter named `key`, and the same literal key twice directly in one block.
- M5-15: Grows slightly: each tree node carries its key or its place number, and the text form prints it.
- M5-16: Grows slightly: lowering numbers the element lines of every child block and puts the number on each unkeyed child.
- M5-17: Reworded: a child is matched to the old child with the same step (key or place number) and kind, under matched parents only, and all siblings sharing a key collapse into one error node whose state is dropped.
- M5-18: Reworded: focus is one id held by the runtime and is dropped when that element is no longer matched.
- M5-24: Grows slightly: compiled trees carry the place number, and M5-26's comparison shows any disagreement with the interpreter.
- M6-18a: Grows slightly: release trees carry the same place numbers as the interpreter's.
- M5-39: Grows slightly: AccessKit wants numbers, so the runtime keeps a table from path ids to numbers that stays stable while the id lives.
- M5-29: Reworded slightly: its done-when already restarts kept element state with the app state; focus is dropped too.

## M5-03 · Decide the text input model: text held by app or element

### M5-03.1 Who holds the text a person types: the app's state or the input element?

**App holds the text** (recommended). The text is a field of the app's state and the input shows whatever the view passes. Each edit is sent as a message and the app stores it; `add_item` appends the draft and clears it. The element keeps only cursor, selection and text still being composed. Costs one state field, one variant and one match arm per input.

```
type Todos { draft: String = "", items: List<Todo> = [] }
enum Msg { Draft(text: String), Add, Toggle(id: Int) }
fn update(s: inout Todos, m: Msg) {
  match m {
    Draft(text) => s.draft = text
    Add         => add_item(&s)
    Toggle(id)  => toggle(&s, id)
  }
}
// view: Input(s.draft, on_change: Draft, key: "new")
```

- Why: The screen stays a function of the state, a trap leaves text and list in step, and an example can read the draft from the state or the tree. It is the Elm model the paper builds on (line 435) and needs no new effect.
- Locks: The shape of every form in every Paka app, the input's signature (M5-04), what M5-19 builds and what UI examples observe (M5-06). Changing the holder later rewrites every app with an input. The record-field separator in the sample follows the M2 grammar, not fixed here.
- Not chosen: Element holds the text (The text lives in the element under its key. The app sees it only by asking, and clears it only by an effect. Less code per keystroke, but the Add button cannot reach the text, two new effects are needed, and a trap rolls back the list while the field keeps what was typed.) · Element holds, app mirrors (The element owns the text and also reports each change, so the app keeps a copy. The Add button works without asking, but there are two copies: clearing still needs an effect, and after a trap the copy and the field disagree.)
- Look again when: M5-08's trial shows composed text (see the preedit point) cannot be kept correct when the app holds the text, or rules that elements sit on an existing toolkit whose text field keeps its own text, or dev-mode handlers prove too slow to run per keystroke (line 77).

### M5-03.2 What does the input send the app when the person edits?

**The whole new text** (recommended). On each edit the input sends one message carrying the full text as it would be after the edit, through a variant named in `on_change:`. One arm in `update` stores it. Costs a copy of the text per keystroke, which is nothing for a one-line field.

```
enum Msg { Draft(text: String), Add, Toggle(id: Int) }

// view
Input(s.draft, on_change: Draft, key: "new")

// update, one arm
Draft(text) => s.draft = text
```

- Why: It is one line in `update`, has no positions to get wrong, and a variant name as the callback is the same device as `then: Loaded` (line 248). An edit-shaped event can be added to a later multi-line element without touching this one.
- Locks: `on_change:` takes a variant with exactly one String field. An input written once per loop row cannot yet say which row it belongs to, because that needs a variant with two fields filled in part; until then an editable list keeps one 'row being edited' in the state and shows one input for it.
- Not chosen: The edit itself (The input sends what happened (insert this text here, delete this range) and the app applies it. Cheap for long texts, but every app must write the text surgery and positions must use the unit M2-03 has not decided.) · Only on Enter (The input sends its text once, when Enter is pressed. Fewest messages, but the text must then live in the element between keystrokes, which is the model the first point turned down.)
- Look again when: M2-09 decides function values: if a function can capture a variable, `on_change:` should take any function from String to the message type so a row's input can carry its item id. Also when a multi-line editor is built.

### M5-03.3 What is shown when the app does not store the text the input sent, because the handler changed it, ignored it or trapped?

**App wins, cursor follows** (recommended). The element changes nothing itself: it works out the new text and where the cursor would go, and sends the message. If the handler traps, or the view hands back the text from before the key, nothing moves. If the view hands back any other text, that text is shown and the cursor goes where the key put it, pulled back to the end if the text is shorter.

```
before:          draft = "mil"   cursor 3
key "k"   ->     Draft("milk")
handler traps:   draft = "mil"   screen "mil"   cursor 3
stores as sent:  draft = "milk"  screen "milk"  cursor 4
stores "MILK":   draft = "MILK"  screen "MILK"  cursor 4
refuses:         draft = "mil"   screen "mil"   cursor 3
```

- Why: The screen never shows text the state does not hold, in every case including a trap, and the rule is one comparison with the text from before the key. A trapped or refusing handler loses the keystroke and nothing else; an app that upper-cases still gets the cursor after the new letter.
- Locks: The order of steps inside the input (M5-19) and the trap behaviour that M5-25b tests. UI examples can rely on: after a trap the tree, cursor included, is unchanged.
- Not chosen: Field shows typed text (The element shows its own text at once and the app's copy catches up. Typing never appears to fail, but after a trap or a refused edit the screen shows text the state does not hold, which line 159 exists to prevent.) · Mismatch is an error (The handler must store exactly the text sent; anything else puts an error node in place of the input. Strict, but it forbids honest uses such as upper-casing or a length limit.)
- Against: The author kept the cursor at its old place on any mismatch: it never moves the cursor after a refusal. But it leaves the cursor before the letter when the app upper-cases, so the next letters go in the wrong order.

### M5-03.4 Where does half-composed text live while a person types through an input method?

**Element only, until committed** (recommended). Preedit is held by the element and drawn inside the field at the cursor, marked as not final; the app gets no message for it. When the person commits, the element sends one `on_change` with the final text put in, under the same rule as a keystroke. If the app's text changes meanwhile the preedit stays, drawn at the cursor's new place; it ends on commit, on the IME's own cancel, or when the input loses focus or leaves the tree.

```
draft = "buy "                    (state)
person composes "gyuunyuu"        -> no message; tree: text "buy ", preedit "gyuunyuu"
person commits "牛乳"               -> Draft("buy 牛乳")
handler traps on that message     -> draft stays "buy ", preedit gone, cursor 4
```

- Why: It keeps M5-37's rule by construction, since preedit is never in a message. A commit behaves exactly like a keystroke, so the mismatch and trap rule covers it with no extra case.
- Locks: Preedit is a second piece of element state beside cursor and selection, kept and dropped under M5-02's rule. The tree text form (M5-06) has a separate preedit field.
- Not chosen: Preedit sent to app (Each preedit change goes to the app as an ordinary `on_change`. No separate element state, but half-typed text enters the state and the list, examples would see it, and the IME's own cancel cannot take it back.) · No inline preedit (The field shows nothing until commit; composing text appears only in the IME's own window, if it has one. Least to build, but on some setups the person types blind.)
- Against: The author had a change of the app's text cancel the preedit, so the field never shows composing text beside text it was not started on. That needs a way to make the IME drop a composition, which the stack may not offer.
- Look again when: M5-08's trial shows the chosen stack cannot deliver preedit events on GNOME Wayland; the third option is then the fallback.
- Owner must still: Rule in M5-08 whether IME support is in M5 at all. If not, M5-37 is dropped and this rule waits unbuilt.

### M5-03.5 Can the app read or set the cursor, the selection or the focus?

**No, element only** (recommended). Cursor, selection and focus belong to the runtime. The person moves them with keys and pointer; the app cannot read or set them, and they show in the tree for examples and the agent. Costs: a handler cannot put the cursor back into a field; whether clicking Add takes focus off the field depends on M5-04's ruling on which elements take focus.

- Why: Nothing in either sample or the M5 exit check needs it, and a focus effect can be added later without breaking any program.
- Locks: Little. The input takes no cursor or focus arguments in M5-04's table.
- Not chosen: Focus by effect (An effect lets a handler move focus to a key path. Lets Add hand focus back, but adds an effect, a grant question, and a rule for a key that is not on screen.) · All held by app (Cursor and selection are state fields and every arrow key is a message. Fully a function of state, but each input costs three fields and the app must redo text navigation the element already knows.)
- Look again when: The owner watches the to-do sample and has to click back into the field after every Add; the small fixes are a button that leaves focus where it is (M5-04) or the focus effect.

### M5-03.6 What is one step of the cursor: a code point or a whole character as a person sees it?

**One code point** (recommended). Arrow keys, Backspace and Delete move or remove one code point. Needs only the Rust standard library. Costs: Backspace can remove an accent and leave its letter, and the cursor can stop inside an emoji made of several code points.

```
text: "e" + combining accent + "x"   (shown as "éx")
cursor after the accent, Backspace -> "ex"
```

- Why: It needs no download and no approval, and the cursor is hidden from the app, so changing the step later touches one module and the few examples that press arrow keys over such characters.
- Locks: Little: M5-19's hand-written cases for non-ASCII text. The cursor position shown in the tree uses whatever unit M2-03 gives String positions.
- Not chosen: One grapheme cluster (A step is one character as a person sees it (a grapheme cluster). Correct for accents and emoji, but needs Unicode's segmentation table from a crate, which must be on M5-08's approved list or be downloaded with the owner's yes.)
- Look again when: M5-08's approved crate list already contains a segmentation table (text layout crates often ship one); then grapheme clusters cost nothing extra and should replace this. Also if M2-03 makes grapheme clusters the unit of String positions.

### M5-03 · settled without asking

- Both models written out for the to-do input: They are the first and second options of point 1; the third is the mixed form. Point 3's sample walks the picked model through a trap and point 4's through preedit.
- The other model under preedit and a trap: With the element holding the text, preedit and text sit together in the element, which is simple. But a trap in the Add handler rolls back the list and drops the clear effect, so the field keeps text the person believes was added; screen and state disagree.
- Shape of the input call: `Input(text, on_change: Variant, key: k)`: the text to show comes first and bare, as in `Text` and `Button`. The full argument list, including a hint text, is M5-04's.
- Enter key: The M5 input is one line; Enter adds no line break. Whether Enter sends a message (an `on_submit:` argument) is M5-04's; the exit check only needs typing and the Add button.
- When the view's text changes without typing: If the app sets the text itself (clearing after Add), the element shows it, pulls the cursor back to the end of the text if needed and clears the selection.
- Selection and plain cursor moves: The selection is held by the element as a start and an end. Arrow keys and pointer moves change cursor and selection at once and send nothing. Typing replaces the selected text, and its cursor and selection follow point 3's rule. A key that would not change the text sends no message.
- Focus at launch: Nothing has focus until the person, Look or an example clicks an input. Keys with no focused input are dropped.
- What examples can observe: The text, from the state and from the tree. Cursor, selection, focus and preedit, from the tree only. The exact form is M5-06's.
- Order of keystrokes: Keys, clicks and effect results wait in one queue. Each is turned into a message only when its turn comes, against the tree of that moment, and is handled to the end, view included, before the next; this is the same rule M5-27 needs for a click queued before a swap. Fast typing queues; it is not worked out against text the app has since replaced.
- Typing while the view is trapped: A trapped view keeps the last frame (line 160), so the tree shows old text while the state may hold newer text. A key typed then is worked out against the text last drawn. This lasts only until the view is fixed and is reported through the tools.
- Speed of typing in dev mode: Each keystroke runs one handler and one view. This rides on the paper's accepted dev-mode slowness (line 77); section 8 lists it under costs to measure (line 360).
- Left out: Clipboard, undo, multi-line text, password fields and a length limit are not built in M5, as the milestone notes say for clipboard. A length limit can be written in `update` already.

### M5-03 · what this changes in TASKS.md

- M5-04: Reworded: the input's row is `Input(text, on_change:, key:)` with no cursor or focus arguments, marked as needing a key; the table must rule on Enter, on a hint text, and on whether a click on a button takes focus from an input.
- M5-06: Grows slightly: the tree text form shows an input's text, cursor, selection and preedit as separate fields; the to-do example reads the draft from the tree; and since nothing has focus at start, the typing function names its target or the example clicks the input first.
- M5-18: Reworded slightly: an input event is turned into a message only when its turn in the queue comes, and keys with no focused input are dropped.
- M5-19: Shrinks and is reworded: the element stores no text, only cursor, selection and preedit; it sends the whole new text and moves the cursor only when the view hands back a text other than the one from before the key. The cursor steps by code point, so no crate table and no download.
- M5-25b: Grows by one case: a handler that traps on an input's message leaves text and cursor as they were.
- M5-37: Reworded: preedit stays in the element and is never in a message; a commit sends one `on_change`; a change of the app's text moves the preedit with the cursor and does not cancel it. Still dropped if M5-08 rules IME out.
- M5-23: Reworded: the to-do sample keeps `draft` in its state with a `Draft(text: String)` message, and Add appends and clears it in one handler.

## M5-04 · Decide the elements M5 builds and every argument of each

### M5-04.1 Which elements does M5 build?

**Five the samples use** (recommended). Text, Button, Input, Row, Col. Both samples and the whole M5 exit check are written with these; the to-do's done mark is a Button. Costs: two fewer than the task's count, no checkbox, no scrolling, no picture, and a long list runs off the window.

```
view todo_row(t: Todo) -> View<Msg> {
  Row(gap: 8) {
    Button(mark(t.done), on: Toggle(t.id))
    Text(t.title)
  }
}
```

- Why: Nothing in the exit check needs a sixth, and adding an element later breaks only a program that already used its name. Fewer names taken now means fewer headless programs broken by M5-12.
- Locks: The names Text, Button, Input, Row and Col are gone from the pool of user names for good; the pages before this one already write them. The reference and both samples use exactly these.
- Not chosen: Seven, with Checkbox and Spacer (Adds a Checkbox for the done mark and a Spacer that takes leftover room. Nearer the task's count, but two more names taken from every program and two more things to draw and test that no exit-check part asks for.) · The paper's seven (Adds the two others line 118 names: a list element and a canvas for free drawing. Nearest the paper's words, but the list cannot be called `List` (the list type), its real use is scrolling, which M5 leaves out, and a canvas needs a drawing vocabulary of its own that no sample asks for.)
- Look again when: The owner watches the to-do sample and the list runs off the window, or a third sample app needs a checkbox.

### M5-04.2 May a call leave out an element argument that has a default?

**Table defaults may be omitted** (recommended). The element table gives some labelled arguments a default, and only those may be left out; the ones written keep table order, `key:` last. In M5 the defaults are `gap:` (0) and `key:` (none), except that an Input's key must be written. The bare first argument, `on:` and `on_change:` have no default. Costs one rule that holds for elements and not for user functions.

```
Col() {
  Row(gap: 8) {
    Button("-", on: Dec)
    Button("+", on: Inc, key: "inc")
  }
}
```

- Why: It keeps the paper's samples and M5-01's `Col() {}` as written, and it is what lets later arguments (alignment, padding, a hint, a message for Enter) arrive without breaking a program.
- Locks: The checker's argument rule for elements (M5-13) and a default column in the table (M5-12). An argument written with its default value is accepted as written; whether the printer drops it is M4-47's.
- Not chosen: Every argument always written (No argument but `key:` may be left out. One rule fewer, but M5-01's `Col() {}` is rejected and any argument added to an element later breaks every program that uses it.)
- Look again when: M2-09 or a later decision gives user functions default arguments; elements should then follow that one rule.

### M5-04.3 Besides `gap:`, may a view set sizes, colours or fonts?

**No look arguments** (recommended). Each element has one fixed look and its size comes from M5-05's rules. `gap:` on Row and Col is the only layout argument. Costs: no heading text, no colour, no fixed width, no padding, no centred content.

```
Col(gap: 8) {
  Text("{s.count}")
  Button("+", on: Inc)
}
```

- Why: Neither sample needs one, and with the last point's rule any of them can be added later as an argument with a default, breaking nothing.
- Locks: Little. The fixed look listed under defaults is what M5-33's pixel tests are written against.
- Not chosen: A few look arguments (Text takes `size:` and `bold:`, containers take `pad:` and `align:`. Apps look better at once, but the alignment values need Upper-case names such as `Start` and `Center`, and `Start` is the variant name the M1 and M2 samples already use.) · A style record (Every element takes an optional `style:` value built from a built-in record. One argument for all look, but it adds a built-in type with many fields to the reference before any sample needs one.)
- Look again when: The owner finds the samples too plain to judge by eye, content touching the window's edge looks wrong, or a sample needs centred or padded content.

### M5-04.4 Which elements take keyboard focus, and does a click on a button take it from the input?

**Inputs only** (recommended). Only an Input takes focus, and only a click on an Input moves it. A click on a Button, a Text or empty space leaves focus where it is. Costs: buttons cannot be reached or pressed from the keyboard, and Tab does nothing.

```
  Input 'new' "mil" cursor=3 on_change=Draft focus
  Button #1 "Add" on=Add
(click on Add)
  Input 'new' "" cursor=0 on_change=Draft focus
  Button #1 "Add" on=Add
```

- Why: It is the small fix M5-03 named for the to-do, and it is the least to build: one kind of focusable element and one way to move focus.
- Locks: The focus rules M5-18 builds and what `focus` can stand on in the tree text. Adding focus to buttons later changes where focus sits after a click, so UI examples that read `focus` would change.
- Not chosen: Buttons take focus too (A click moves focus to the button, Tab walks inputs and buttons, Space or Enter presses the focused button. The usual desktop rule and needed for keyboard-only use, but the to-do loses its typing place on every Add and focus order becomes a rule to state and test.)
- Look again when: M5-08 rules screen-reader support into M5: such tools expect buttons to be reachable from the keyboard, so this point is then reopened.

### M5-04.5 What happens to the paper's other elements?

**Recorded as open** (recommended). The other elements are listed in plain words as not decided, with no milestone and no Paka names fixed. Line 118's 'about 20' is recorded as an open number. Costs: the paper's promise stays unmet until someone asks for an element.

- Why: Nothing is built before it is needed, and each element is cheap to add one at a time when a real app asks for it.
- Locks: Nothing.
- Not chosen: Placed in a milestone (A named milestone after M5 builds a fixed list. Gives a date, but fixes names and arguments for elements no sample has asked for.)
- Look again when: A sample app or the owner asks for one of them.
- Owner must still: When the paper is next revised, line 118 should say that 'about 20' is open and M5 builds five.

### M5-04 · settled without asking

- Text: `Text(text)`: one bare String, then optional `key:` (an Int or String, M5-02). No child block, no event, holds no state, takes no focus. Look: the default font at 14 units with lines 20 units high, black on the window's white, left-aligned, wrapped by M5-05's rule. Only a String is accepted; a number is turned into text first (line 47).
- Button: `Button(label, on: message)`: one bare String, then `on:` with a message value of the view's message type, then optional `key:`. A click sends the message. No child block, no state, no focus. Look: the label on one line, 12 units of space left and right and 6 above and below, fill #E6E6E6, border 1 unit #808080, corners rounded by 4.
- Input: `Input(text, on_change: Variant, key: k)`: the text to show, a variant with exactly one String field, and a key that must be written. It holds cursor, selection and preedit (M5-03) and takes focus. One line, 32 units high, text set in by 8 units, fill white, border 1 unit #808080, or 2 units #0060DF when focused, drawn inside the same box; cursor 1 unit wide, black; selection behind the text in #B4D5FE.
- Row and Col: `Row(gap: n) { ... }` and `Col(gap: n) { ... }`: `gap:` is an Int with default 0, then optional `key:`, then a child block that is always written. Row places children left to right, Col top to bottom. They draw nothing themselves, send no event, hold no state. A negative literal gap is a check-time error; a computed one is a trap in the view, of the failed-assertion kind (line 63) unless M2-06 names another.
- Marks in the table: Per element the table holds: takes a child block (Row, Col), holds state and so needs a key (Input only), takes focus (Input only), and the width class M5-05 uses (Button hugs its content; Text, Input, Row and Col fill). Per argument it holds the type and a default or none.
- Placeholder and error node: They are not elements and cannot be written in a view. A placeholder comes from `??name` and draws a dashed #808080 border around the name, sized like a Button with that label. An error node draws its message as wrapped text on #FFE0E0 with a 1-unit #C00000 border and 6 units of space inside. A click on either does nothing.
- What a click is: A press and a release of the main pointer button on the same element. The message is sent on release. Other pointer buttons, double clicks and hover do nothing in M5.
- Hover and pressed look: None in M5: a Button looks the same whether pointed at or pressed. This keeps Button free of state, so it needs no key.
- Enter key: Enter in a focused Input changes nothing and sends nothing; the Input's one event is `on_change:`. The exit check asks only for typing and the Add button. An optional `on_submit:` message can be added later as an argument with a default, breaking no program; that is the fix if the owner presses Enter in the to-do sample and expects a new row.
- Tab key: Dropped in M5. Focus moves only by a click on an Input.
- Line breaks in a label or an input's text: A Button label and an Input's text are drawn on one line. A line break in them is drawn as a space; the text itself is not changed.
- Hint text, disabled, password, length limit: Not in M5. The Input has no hint text and the Button no disabled form; each can be added later as an argument with a default.
- The paper's `list` and `canvas` elements: Not built. A `for` in a Col gives the rows. If a list element is built later it needs another name, since `List` is the list type.
- Elements not decided: Listed in plain words, with no Paka names fixed: checkbox, choice of one (radio), drop-down, slider, multi-line text editor, scrolling container, long list that builds only visible rows, grid or table, stack of overlapping layers, spacer, divider line, picture, icon, canvas for free drawing, progress bar, tabs, dialog, menu, tooltip, link.
- Element names: The paper's spellings, already used by the pages before this one: `Text`, `Button`, `Input`, `Row`, `Col`. `Row`, `Input` and `Text` are likely type names in headless programs, so M5-12's renaming of earlier test cases is expected work.

### M5-04 · what this changes in TASKS.md

- M5-12: Reworded: the table has five elements and, per argument, a type and a default or none; per element the child-block, needs-key, takes-focus and width-class marks.
- M5-13: Grows slightly: a labelled element argument with a table default may be left out, written ones must follow table order with `key:` last, and a negative literal gap is rejected.
- M5-16: Grows slightly: building a Row or Col with a negative computed gap is a trap in the view.
- M5-18: Reworded: only an Input takes focus, only a click on an Input moves it, a click on a Button leaves it, and Tab and Enter are dropped.
- M5-19: Shrinks slightly: no Enter message and no hint text.
- M5-23: Reworded: the to-do sample uses only the five elements and its done mark is a Button.
- M5-33: Reworded: it draws the fixed look given here, with no hover or pressed form, and its pixel tests use these colours.
- M5-36: Reworded slightly: a click is press and release on the same element; other buttons, double clicks and hover are ignored.
- M5-39: Grows if it exists: a screen-reader ruling in M5-08 reopens the focus point, since buttons must then be reachable from the keyboard.
- M5-05: Shrinks: no size, padding or alignment arguments to give rules for; only `gap:` and the per-kind width class.
- M5-41: Shrinks slightly: five element rows in the reference.

## M5-05 · Decide the layout rules and units

### M5-05.1 What unit are sizes written and computed in?

**Whole units, scaled** (recommended). Every size and position is an Int number of units. One unit is one pixel on a screen with scale factor 1; the window multiplies by the screen's scale factor only when drawing. Measured text is rounded up to whole units. Costs: nothing can be placed at half a unit.

```
Row(gap: 8) at scale 1: buttons 8 pixels apart
Row(gap: 8) at scale 2: buttons 16 pixels apart
layout rectangles: the same numbers in both
```

- Why: It keeps the paper's `gap: 8`, makes rectangles exact integers that M5-31 can compute by hand, and gives the same layout at every scale factor.
- Locks: The type of every size argument now and later, the rectangles in M5-31's tests, and hit testing (finding the element under a point) in M5-36. Moving to Float later rewrites every view.
- Not chosen: Fractional units (Sizes are Float. Text widths need no rounding, but `gap: 8` must be written `gap: 8.0`, and layout results depend on float rounding that tests must pin to the last bit.) · Physical pixels (Sizes are device pixels. Nothing to scale, but the same app is half the size on a high-density screen and every view must know its screen.)
- Look again when: Rounded-up text widths make text look visibly loose at the font M5-08 picks.

### M5-05.2 What decides the width and height of an element?

**Width down, height up** (recommended). A parent hands each child a width; the child answers with its height. A Button hugs: it takes the width of its label plus its inner space. Text, Input, Row and Col fill: they take the width handed to them. In a Row, hugging children take their width first and the filling children share the rest equally. One pass, no arguments. Costs: two Texts in a Row split it in half whatever their length.

```
Row 480 wide, gap 8, test measure 8 per character:
  Button "x"       x=0    w=32   (8 + 12 + 12)
  Text "buy milk"  x=40   w=384  (480 - 32 - 48 - 8 - 8)
  Button "del"     x=432  w=48   (24 + 12 + 12)
```

- Why: It needs no argument and no extra element, gives a to-do row its right-hand button for free, and every rectangle can be worked out by hand in one pass. Size arguments can be added later with defaults that mean exactly this.
- Locks: The width class of each element kind in the table, the algorithm in M5-31, and how every existing app looks. Later arguments must default to this behaviour or apps change shape.
- Not chosen: Per-child size arguments (As in CSS flexbox: every element takes optional `width:` and `grow:`, and leftover room goes to children by their grow share. Most control, but two more arguments on every element and a harder rule to compute by hand.) · Natural size plus Spacer (Every element takes its natural size and a Spacer element takes the leftover. Simple to state, but it needs a sixth element, and text beside a button needs a second rule for when to wrap.)
- Look again when: A sample needs two Texts side by side with unequal widths, or a button pushed right with no text beside it.

### M5-05.3 How are children lined up across a Row or Col?

**Left; centred in rows** (recommended). No argument. In a Col each child starts at the left edge. In a Row each child is centred top to bottom in the row's height, an odd leftover unit going below. Text is left-aligned. Costs: nothing can be centred or right-aligned across a Col.

```
Row height 32 (its tallest child, a Button):
  Button "x"       y=0  h=32
  Text "buy milk"  y=6  h=20   ((32 - 20) / 2)
so the label and the button text sit on one line
```

- Why: It makes the commonest row, a button beside a label, look right with no argument and takes no names. An argument whose default is this rule can be added later.
- Locks: The rectangles in M5-31 and the look of every app. If an alignment argument comes later its value names must avoid `Start`.
- Not chosen: Left and top (As above, but Row children start at the top. One rule fewer to state, but text beside a button sits 6 units higher than the button's label.) · An align argument (Row and Col take `align:` with built-in values. Full control, but the values take common names such as `Start`, `Center` and `End` from every program, which breaks the light program's `Start`.)
- Look again when: The owner wants the counter centred in its window, or a sample needs right-aligned content.

### M5-05.4 How does Text wrap?

**Always wraps to its width** (recommended). Text is broken into lines so that each fits the width it was handed; a line break in the string starts a new line. Lines are filled word by word; where a line may break is the text library's rule (line 131), which for plain Latin text means at spaces, and the spaces at a break are not drawn. A word wider than the line is broken where it stops fitting, with at least one character per line. Height is lines times 20 units; an empty text is one line.

```
Text "buy milk and bread", width 100, 8 per character:
  12 characters fit (96 <= 100)
  line 1: "buy milk and"
  line 2: "bread"
  size: 100 wide, 40 high
```

- Why: Nothing typed is ever hidden, and it needs no argument. A `wrap:` argument defaulting to true can be added later.
- Locks: Text heights in M5-31's tests and the line-breaking call M5-32 makes. In the window, break places come from the text library; the hand-computed tests use the test measure, which breaks at spaces only.
- Not chosen: Never wraps (Text is one line and what does not fit is cut off. Height is always one line, but long titles lose their end and there is no way to see it without scrolling.) · A wrap argument (Text takes `wrap:` true or false. Both behaviours, at the cost of an argument no sample needs.)
- Look again when: M5-08's text library breaks ordinary Latin text at places other than spaces often enough that the hand-computed tests and the window disagree.

### M5-05.5 What happens to content that does not fit?

**Sticks out, window cuts** (recommended). Nothing shrinks and no container clips. A child that does not fit keeps its size and runs past its parent's right or bottom edge; only the window edge cuts it off. Later elements draw over earlier ones. Costs: rows below the window cannot be reached, and overlapping can look wrong.

```
Row 100 wide, gap 8:
  Button "Cancel"    x=0   w=72
  Button "Save all"  x=80  w=88   (ends at 168, 68 past the row)
the Row is still 100 wide; a 100-wide window shows the first 20 units of "Save all"
```

- Why: It is the least to build and to compute by hand, and the real answer to content that does not fit is scrolling, which is not in M5.
- Locks: Little: drawing order and hit testing in M5-31 and M5-33. A scrolling container added later brings its own clip.
- Not chosen: Each container clips (A Row or Col cuts off what leaves its box. Tidier, but it adds a clip per container to drawing and hit testing and hides content the same way.) · Shrink to fit (Children are made narrower until they fit. Nothing is hidden at first, but labels get cut, and the rule for who shrinks by how much is the hard part of flexbox.)
- Look again when: The owner adds enough to-do rows to pass the window's bottom edge; that is the case for building scrolling.

### M5-05 · settled without asking

- Root and window: The root element is handed the window's width and sits at 0,0, with no margin: content starts at the window's edge. The window opens 480 by 360 units, can be resized, and is white where the tree does not reach. A resize lays the same tree out again; the view does not run.
- Worked example: the counter at 480 wide: With the test measure (8 units per character, lines 20 high): Text "0" at 0,0 size 480x20; Row at 0,28 size 480x32; Button "-" at 0,28 size 32x32; Button "+" at 40,28 size 32x32; placeholder `reset` at 0,68 size 64x32. The Col is 480 wide and 100 high: 20 + 8 + 32 + 8 + 32.
- Gap: The gap is put between each two children that are present, never before the first or after the last. A child left out by an `if` takes no gap. Placeholders and error nodes count as children. Example: a Col with gap 8 and three one-line Texts puts them at y = 0, 28 and 56 and is 76 high; with the middle one left out, at y = 0 and 28 and 48 high.
- Sharing leftover width: Filling children of a Row share the leftover equally; when it does not divide, the earlier children get one unit more. Example: Row 101 wide, gap 8, a Text and an Input: leftover 93, so Text 47 wide at x=0 and Input 46 wide at x=55.
- Smallest width of a filling child: A filling child in a Row is never narrower than 40 units; if the leftover gives less, it takes 40 and the row overflows to the right. Example: Row 100 wide, gap 8, Button "Save all" (88 wide) and a Text: leftover 100 - 88 - 8 = 4, so the Text takes 40 at x=96 and ends at 136, 36 past the row.
- Heights: Text: lines times 20, at least one line. Button and Input: 32. Row: its tallest child, or 0 when empty. Col: its children's heights plus the gaps, or 0 when empty.
- Size of a placeholder and an error node: A placeholder hugs: its name's width plus 24, and 32 high. An error node fills: its message wraps inside 6 units of space on every side.
- Text in an Input that is too long: The Input keeps its size and cuts its text at its own border, the one clip in M5. The text is shifted left just enough that the cursor is inside; the shift is worked out from the cursor each frame and is not kept state.
- Text measuring: Width comes from the text library with the default font at 14 units, measured at scale factor 1 so the screen does not change the layout; each measured width is rounded up to a whole unit and the line height is set to 20. One tree gives one result for one font; which font is M5-08's. M5-31's tests replace this with the test measure of 8 units per character.
- Finding the element under a point: The point belongs to the last-drawn element whose rectangle holds it: children are drawn after parents and later siblings after earlier ones. An element that sticks out of its parent can still be hit there. Row and Col are never hit themselves, so a point in none of their children hits nothing.
- Scale factor: Layout is always in units. The window multiplies by the screen's scale factor when drawing and divides pointer positions by it, rounding down, before hit testing.
- Layout and UI examples: Rectangles are not part of the tree text and UI examples never see them (milestone note: examples need no window or font).

### M5-05 · what this changes in TASKS.md

- M5-31: Reworded: one pass with widths handed down and heights handed up, the hug and fill classes, equal sharing with the spare unit to earlier children, the 40-unit floor, centring in rows, no clipping, last-drawn hit testing that also finds an element sticking out of its parent, all in whole units with the 8-by-20 test measure that breaks at spaces and keeps one character per line at least.
- M5-32: Grows slightly: text is measured at scale factor 1, measured widths are rounded up to whole units, the line height is fixed at 20 units and the font size at 14.
- M5-33: Reworded slightly: the only clip is an Input's own text; units are multiplied by the scale factor at drawing.
- M5-35: Grows slightly: the window opens at 480 by 360 units, and a resize lays the tree out again without running the view.
- M5-36: Reworded slightly: pointer positions are divided by the scale factor before hit testing.
- M5-12: Grows slightly: the table carries each element's width class.
- M5-15: Unchanged in size: tree nodes carry no rectangles; layout results live beside the tree.

## M5-06 · Decide UI example functions, the tree text form and hole status

### M5-06.1 How does an example get a running app and act on it?

**Ui value, changed in place** (recommended). `ui_start(app)` gives a `Ui`: a running app with no window. Action functions take it as `inout`; read functions take it read-only. The example ends in a Bool, like a function example. Costs one built-in type name and a `ui_` prefix on every call.

```
example inc_once {
  var ui = ui_start(counter)
  ui_click(&ui, "#1/#1")
  ui_text(ui, "#0") == "1"
}
```

- Why: It reuses the example form and the `inout` call the paper already shows, hides nothing, and drives the same windowless loop a person's click drives.
- Locks: The name `Ui` and the `ui_` names, taken from every program. Every UI example ever written, human-owned ones included.
- Not chosen: Example names its app (The example header names the app and the functions act on it without an argument. Shorter, but the functions change something no argument names, against line 165, and it adds a second example form.) · No actions, call update (An example calls `update` itself and turns the view's result into text. Nothing new but one function, but clicks, focus and typing are never exercised, line 183 asks for actions on the tree, and a view would have to be callable outside a view, which M5-01 turned down.)
- Look again when: M2-05 spells `inout` arguments other than `&ui`, or M2-06 gives example bodies another shape; only the spelling changes. Also when examples need many actions only to reach a starting state; a start that takes a state value can then be added.

### M5-06.2 How does an example say which element it means?

**Path as a String** (recommended). A path is the steps from the root joined by `/`: an Int key as its digits, a String key in single quotes, a place number as `#n`. The root is the empty string. A badly formed literal path is a check-time error; an element that is not there is found when the example runs. Costs: a place number changes when a line is added above it, so an element an example clicks is best given a key.

```
ui_click(&ui, "'new'")             // String key "new"
ui_click(&ui, "#1")                // second element line of the root
let title = ui_text(ui, "7/#1")    // row with Int key 7, its second line
let odd = ui_has(ui, "'3'")        // String key "3", not Int key 3
let t = ui_text(ui, "{id}/#1")     // built from an Int value
```

- Why: It is M5-02's id written down, the same text the tree form and Look show, so the agent copies a path from the tree it just read. It adds no names and no second way to address.
- Locks: The id text form shared by examples, the tree text, Look (M5-09) and any accessibility bridge. Changing it later rewrites every UI example.
- Not chosen: Find by kind and label (Functions find an element by what it shows, such as the button labelled Add. Survives edits that move lines, but two rows with the same label need a second way to pick one, and every kind needs its own function.) · Typed list of steps (A path is a list of built-in step values. Checked by type, not by parsing, but three more Upper-case names leave the pool and every call is longer.)
- Against: The author wrote a String key bare when it is shaped like a name (`new`) and in quotes only otherwise, with one spelling accepted: fewer characters in the common case. But a path built from a String value then has no single template that always works, and `Hole footer` reads like a node with the key footer.
- Look again when: Human-owned examples keep breaking because place numbers shift under harmless view edits; a find-by-label function can then be added beside paths.

### M5-06.3 What is the text form of the tree?

**One line per node, indented** (recommended). Each node is one line, two spaces deeper than its parent: kind, step (none for the root), the shown text in double quotes, then the `name=value` fields listed per kind under defaults, then `focus` if it has focus. A placeholder is `Hole`, an error node `Error`. Costs: a format of Paka's own to specify.

```
Col gap=8
  Input 'new' "mil" cursor=3 on_change=Draft focus
  Button #1 "Add" on=Add
  Row 7 gap=8
    Button #0 "[ ]" on=Toggle(7)
    Text #1 "buy milk"
  Error 8 "duplicate key 8: 2 children of Col"
  Hole #4 footer
```

- Why: It is the cheapest in tokens, easy to write by hand before the code exists (caller rule 5), and each line starts with what the agent needs to build a path.
- Locks: Every hand-written tree in the test suite, what `ui_tree` returns, and what Look's tree call returns. Any change to the form rewrites all of them.
- Not chosen: Looks like view code (The tree prints as nested element calls. Familiar to the agent, but it reads as source that could be pasted back, and ids and input state have no place in that spelling.) · JSON (The tree prints as nested JSON objects. Any tool can parse it, but it costs several times the tokens (line 41 makes the same point about syntax trees) and is painful to write by hand in tests.)
- Look again when: M2-03 fixes string escapes and M2-09 with M2-14 fix how a value prints as text; quoted texts and message values in the tree follow those rules.

### M5-06.4 How much does the tree text show?

**All the view passed** (recommended). Every argument the view gave, messages included, plus what the element holds (cursor, selection, preedit) and focus. An argument at its default and an empty selection or preedit are not printed. Costs: a message with a large payload makes a long line.

```
Row 7 gap=8
  Button #0 "[x]" on=Toggle(7)
  Input 'edit' "milk" cursor=4 sel=1..4 preedit="gyuu" on_change=Rename focus
```

- Why: The tree is the agent's eyes (line 124) and the thing two back ends are compared on, so a wrong message must show. Leaving defaults out keeps it short.
- Locks: The fields of every node line. Message values print in the value text form M2-09 and M2-14 will fix.
- Not chosen: What is shown only (Kinds, steps, shown text and input state; no messages and no gaps. Shorter, but a button wired to the wrong message looks right in the tree and in the back-end comparison.) · With rectangles (Also each element's position and size. Lets the agent see layout, but then every tree needs a window width and a font, which the milestone notes rule out for UI examples.)
- Look again when: Trees of real apps make Look's reply too long because of message payloads; payloads can then be cut to a length.

### M5-06.5 What can an example read back?

**Tree, one text, presence** (recommended). Three read functions: `ui_tree(ui)` gives the whole tree text, `ui_text(ui, path)` the text one Text, Button or Input shows, `ui_has(ui, path)` whether an element is there. The app's state is not readable through a `Ui`; state is checked by function examples on `update`.

```
let cleared = ui_text(ui, "'new'") == ""
let gone = ui_has(ui, "7") == false
let same = ui_tree(ui) == expected
```

- Why: Two narrow readers cover what the to-do and counter examples assert, and the whole tree stays available for the rest. More readers can be added without breaking an example.
- Locks: Little. M5-03 said examples can observe the draft from the state or the tree; under this pick a UI example reads the tree, and the state is checked by a function example.
- Not chosen: Whole tree only (Only `ui_tree`. One function, but every example is a long text comparison that fails on any unrelated change to the view.) · Nodes as values (The tree is returned as a built-in record an example can walk. Most power, but it adds a built-in tree type and its fields to the reference.)
- Look again when: Examples need cursor, focus or a button's message often enough that comparing whole trees becomes the habit.

### M5-06.6 Which UI examples are reported as unfinished because of a hole?

**Only if it touches one** (recommended). A placeholder on screen does not by itself change a status. The example stops as unfinished if it clicks, reads or tests a path that is a placeholder, if `ui_tree` is called while the tree holds one, or if any hole is reached while a handler or view runs. Costs: an example can pass on a screen that is partly placeholders.

```
example inc_with_hole_on_screen {       // pass
  var ui = ui_start(counter)
  ui_click(&ui, "#1/#1")
  ui_text(ui, "#0") == "1"
}
// ui_click(&ui, "#2")  -> unfinished: #2 is Hole reset
// ui_tree(ui) == "..." -> unfinished: the tree holds Hole reset
```

- Why: It is the function-example rule carried over: a function example passes while other functions still hold holes, as long as it reaches none. Finished pieces get passing evidence while the rest is still skeleton.
- Locks: The statuses M5-22 computes and the hand-written expectations in M5-22 and M5-23. Release builds have no holes, so human-owned examples run there are not affected.
- Not chosen: Any placeholder in any frame (If a placeholder is in the tree at any moment of the example, it is unfinished. Strictest evidence, but no UI example can pass until the whole screen is finished, which defeats growing the app piece by piece, and M5-22's three counter examples could not report pass or fail.) · Placeholders never matter (A drawn placeholder is an ordinary node; clicking it does nothing and reading it gives its name. Simplest, but an example can pass by asserting on a placeholder, and line 185 is not met.)
- Look again when: M2-06 defines 'reaches a hole' in a way that does not fit; this rule should then follow it.

### M5-06.7 What is an example's status when the app traps or shows an error node while it runs?

**Trap ends the example** (recommended). If a handler or the view traps during a UI example, the example stops there with the status M2-06 gives a trapped example; a trap that comes from reaching a hole gives unfinished instead (line 185). If a frame holds an error node, it stops with status fail, naming the node. Costs: an example cannot show that an app recovers from a trap; the toolchain's own tests do that.

```
example add_twice {
  var ui = ui_start(todo)
  ui_click(&ui, "#1")     // handler traps here
  ui_has(ui, "1")
}
// status: trapped, at the ui_click line, with the handler's trap report
```

- Why: A pass then means no trap and no error node happened, which is what a person who never reads the code would assume it means.
- Locks: The status rules in M5-22. Loosening later would turn some trapped examples into passes; tightening the other way would break passing ones.
- Not chosen: App carries on (The example sees what a person would: the state rolled back, the last frame kept. An example can assert recovery, but a passing example may have trapped on the way without saying so.)
- Look again when: M2-06 gives function examples a different rule for traps inside called code, or names the statuses otherwise.

### M5-06.8 Where does typed text in an example go?

**To the focused input** (recommended). `ui_type(&ui, text)` sends the text one code point at a time to the input that has focus, as a keyboard would. The example clicks the input first. With no focused input the call is a failed-assertion trap that says so. Costs one extra line per example.

```
ui_click(&ui, "'new'")
ui_type(&ui, "milk")
```

- Why: Typing follows focus exactly as it does for a person and for Look, so focus handling is tested by every typing example. Typing into nothing is always a mistake in an example, and the trap names it at the line that made it.
- Locks: Little: the signature of `ui_type` and every example that types. Turning the trap into a silent drop later breaks no passing example.
- Not chosen: Typing names its target (`ui_type` takes a path, gives that input focus and types. One line, but it moves focus as a side step, and the click-to-focus path is never exercised by examples.)
- Against: The author dropped the keys when nothing has focus, as happens live, so one rule covered person, Look and example. But `ui_click` on a Text already traps although live it does nothing, and a silent drop surfaces lines later as a path that finds nothing.

### M5-06 · settled without asking

- The function list: `ui_start(app) -> Ui`; `ui_click(ui: inout Ui, path: String)`; `ui_type(ui: inout Ui, text: String)`; `ui_tree(ui: Ui) -> String`; `ui_text(ui: Ui, path: String) -> String`; `ui_has(ui: Ui, path: String) -> Bool`. All arguments are bare. They may be called only in an example body.
- ui_start: Takes the bare name of a visual app. It follows M5-01's launch order with no window: state from defaults, the `start:` message if there is one, first tree. Naming a headless app is a check-time error.
- The to-do example, written out: `example add_one_item {` / `var ui = ui_start(todo)` / `ui_click(&ui, "'new'")` / `ui_type(&ui, "milk")` / `ui_click(&ui, "#1")` / `ui_text(ui, "'new'") == "" && ui_text(ui, "1/#1") == "milk"` / `}`. It assumes M5-01's to-do view and that the first item gets id 1. The `&&` spelling waits on M2-03.
- The tree after that example: Line by line: `Col gap=8`; `  Input 'new' "" cursor=0 on_change=Draft focus`; `  Button #1 "Add" on=Add`; `  Row 1 gap=8`; `    Button #0 "[ ]" on=Toggle(1)`; `    Text #1 "milk"`. The 'Nothing to do' Text was line #2 and is gone; focus stayed on the input (M5-04).
- Fields per kind: Text: the text. Button: the label, `on=`. Input: the text, `cursor=`, `sel=a..b` when something is selected, `preedit=` when composing, `on_change=`. Row and Col: `gap=` when not 0. Hole: its name. Error: its message. `focus` comes last on the focused line. Keys are shown as the step, not as a field. A shown text is printed with line breaks and quotes escaped by M2-03's rule, so one node is always one line.
- Steps in the tree and in paths: Int key: its digits, with `-` in front when negative. String key: in single quotes, with `'` and `\` inside it escaped by `\`. Place number: `#n`. A placeholder made in a loop: `#n.r`, where r counts the placeholders that line has already added to this child list in this frame, from 0, so nested loops need no second number. An error node for a duplicate key carries that key as its step. The root has no step.
- Hole and Error as words: `Hole` and `Error` appear only in the tree text. They are not names a program can write, so they take nothing from the pool of names.
- When an action returns: Each action is handled to the end before the function returns: the message, the handler, the view, the comparison with the old tree. This is M5-03's queue rule with a queue of one.
- ui_click details: On a Button it sends the button's message. On an Input it gives focus and puts the cursor at the end of the text, since there are no pixels. On a placeholder the example is unfinished. On any other kind, or on a path that finds nothing, it is a trap of the failed-assertion kind that names the path.
- ui_type details: Each code point is one key, handled to the end before the next. If focus is lost part-way, the next key is the failed-assertion trap. A line break or tab in the text is the Enter or Tab key, which M5-04 says do nothing.
- ui_text and ui_has details: `ui_text` gives the text the app passed; on a Row, Col or error node, or on a path that finds nothing, it is a failed-assertion trap. `ui_has` never traps on a well-formed path; a badly formed path built at run time is a trap. Either one on a placeholder makes the example unfinished.
- Which status wins: The example stops at the first of these and takes its status: a hole touched or reached gives unfinished; any other trap gives M2-06's trapped status; an error node in a frame gives fail. Otherwise the last line's Bool decides pass or fail.
- Effects in a UI example: Not performed: an example cannot touch files, console or clock (line 165). An effect the grants allow is dropped, so no answering message arrives. One they do not allow is handled as it is live, which M1-06.1 makes a trap for now and M2-15 settles. An `Exit` ends the app; reads still see the last tree, and any later action on that `Ui` is a failed-assertion trap.
- Other keys: No function for Backspace, arrows or Enter in M5. How the Input edits is the toolchain's own test matter (M5-19); an app sees only `on_change` messages. No function composes text, so preedit never shows in a UI example in M5.
- A Ui as a value: An ordinary owned value under M2-05's rules. It cannot be stored in app state or compared; the checker rejects `Ui` outside example bodies.
- Launch traps: `ui_start` on an app whose start handler or first view run traps ends the example under the trap point's rule.
- Who owns an example and caching: Not ruled here: agent-written or human-owned is M7's, and M5 runs every UI example each time (M5-22's note).

### M5-06 · what this changes in TASKS.md

- M5-15: Reworded: the printer follows the one-line-per-node form, with steps (digits, single-quoted String keys, `#n`, `#n.r`), fields per kind, defaults left out, escaped texts, and `Hole` and `Error` lines.
- M5-21: Reworded: it checks the six `ui_` functions and the `Ui` type, allows them only in example bodies, and rejects a badly formed literal path; its 'wrong element references must fail at check time' can only mean the path's form, since whether an element exists depends on state.
- M5-22: Reworded: the hole status is 'unfinished only if the example touches a placeholder or a hole is reached'; any other trap in a handler, a view or a `ui_` call ends the example as trapped; an error node ends it as fail; allowed effects are dropped.
- M5-23: Reworded: the to-do UI example is the one written here; elements an example clicks are best given keys.
- M5-12: Grows slightly: `Ui` and the six `ui_` names enter the built-in names table for every program.
- M5-16: Grows slightly: trees carry message values and non-default arguments so the printer can show them, using the value text form of M2-09 and M2-14.
- M5-09: Shrinks slightly: Look's tree call returns this text form and its click can take the same path form; the rest is still M5-09's.
- M5-26: Unchanged in size: comparing tree text now also catches a wrong message or gap.
- M5-34: Reworded slightly: a Look click on an Input by path puts the cursor at the end, as `ui_click` does.
- M5-41: Grows slightly: the reference gains `Ui`, the six functions, the path form and the tree text form.

## M5-07 · Decide how migrate and explicit reset are written and applied

### M5-07.1 Where are migrate and reset written: in the source, or in the edit that needs them?

**Carried by the edit** (recommended). The Edit call has one optional state field holding either `reset` or the text of one migrate. It belongs to the change set waiting to be installed, is used once at the swap and is then thrown away. It is never written to the source files. Costs: the tool interface gains a field, and no record of the migrate stays in git.

```
edit request (field names are M4-03's to fix):
  replace: type Counter { count: Int = 0, step: Int = 1 }
  base:    <version of Counter>
  state:   migrate(old: Counter) -> Counter {
             Counter { count: old.count }
           }
```

- Why: It is what line 122 says, the source stays a description of the program and not of one session, and the old type never has to exist in the source because the toolchain still holds the running version.
- Locks: The Edit call's shape (M4-03, M4-17) and where M5-30 finds the migrate. `migrate` stays a reserved word that is not a definition kind. The comma between fields in the samples follows the M2 grammar and is not fixed here.
- Not chosen: A definition in the source (`migrate` is a seventh definition kind stored beside the type. Visible in git, but it describes a type that is gone, means nothing to a program started fresh, and someone must delete it after it has run.) · Reset only, no migrate (A changed state type always restarts the state. Least to build, but line 122 and part 3 of the M5 exit check both ask for a migrate that keeps the count.)
- Look again when: State is one day kept between runs on disk; a migrate must then be stored with the program.

### M5-07.2 What counts as a change to a stored type?

**Any change but defaults** (recommended). The stored types are the state type and every user type reached from it through fields, list items, map keys and values, enum payloads, shared values and pool items. Compared with the running version, the state has changed if the app names another state type or any stored type's definition differs in anything but default values, comments and layout. Costs: renaming a type, reordering fields or adding a variant needs a migrate or a reset though the data would have survived.

```
type Todo { id: Int, title: String, done: Bool = false }
type Todos { draft: String = "", items: List<Todo> = [] }

add `note: String = ""` to Todo      -> changed (reached through the list)
change Todo's `done` default to true -> not changed
add a variant to Msg                 -> not changed (not stored)
rename Todo to Item                  -> changed
```

- Why: It errs toward refusing, which costs one reset, and never toward running old data under a new shape. Letting more edits through later breaks nothing.
- Locks: A stored-shape comparison of its own in the derived database (M5-28). The cost of adding a variant to an enum kept in the state: a migrate that matches every old variant, or a reset.
- Not chosen: Only layout changes (Changed only if the data's shape in memory differs; renames and reordering that the runtime could map by name are let through. Fewer refusals, but the mapping is new machinery, and a wrong mapping corrupts live state silently.) · Use the interface hash (Changed whenever the state type's interface hash (line 285) changes. No new hash, but that hash includes default values, so a changed default is refused, and it must first be widened to reach types held in lists.)
- Look again when: Resets forced by renames or added variants become a routine nuisance. Also when M2-09 decides function values: if one can be stored in state, any code change could be a state change. M2-10 fixes what 'shared value' and 'pool' mean here; M2-01 left pools unbuilt.

### M5-07.3 Does one migrate turn the whole old state into the new, or is there one per changed type?

**One for the whole state** (recommended). A migrate takes the old state value and gives the new state value. Fields whose types did not change are carried over as they are; changed items are rebuilt by a loop the agent writes. One function, nothing for the runtime to walk. Costs a few lines of copying when the change is deep, and a changed type that contains itself cannot be converted.

```
migrate(old: Todos) -> Todos {
  var items: List<Todo> = []
  for t in old.items {
    append(&items, Todo { id: t.id, title: t.title, done: t.done, note: "" })
  }
  Todos { draft: old.draft, items: items }
}
```

- Why: It is an ordinary function run once, with no walking machinery to get silently wrong. Per-type migrates can be added later as a shorter spelling of the same thing.
- Locks: What M5-30 runs. An old value of a changed type cannot be passed to a helper function, since no signature can name the old type, so the conversion is written inside the migrate with loops. A changed type that contains itself (a tree) cannot be converted that way; such an edit takes a reset.
- Not chosen: One per changed type (A migrate converts one value of one changed type, and the runtime finds every such value in the state and applies it. Shorter to write, but the runtime must walk lists, maps, payloads and pools, and several migrates must be sent and checked together.)
- Look again when: A real app holds a tree-shaped type in its state and loses it to a reset; per-type migrates, or helpers carried by the edit, are then the fix. Also if M2-05 asks for a written copy when a value is taken out of a read-only one, or M2-14 spells list building other than `append(&items, ...)`; only the sample changes.

### M5-07.4 How does a migrate refer to the old types?

**Parameter old, result new** (recommended). In `migrate(old: T) -> U`, the parameter's type is read in the running version and the result's type in the new one. Inside, anything read out of `old` has its old type and a pattern on it uses the old variants; anything built, and any type name written in the body, is new. A type that did not change, itself or in anything it reaches, is the same type on both sides. No new name or keyword.

```
migrate(old: Counter) -> Counter {
  Counter { count: old.count }
}
```

- Why: The old type is taken from the running version, which the toolchain already holds, so nothing is restated and nothing can be mis-copied. A migrate has exactly one parameter, so position leaves no doubt.
- Locks: The migrate's grammar and its checking against two versions of the types at once (M5-30). An old value of a changed type can only be taken apart: fields read, matched on, looped over.
- Not chosen: An Old marker (The old type is written `Old<Counter>`. Explicit, but it takes a built-in name from every program and is written in exactly one place, where position already says it.) · Old types restated (The edit also carries the old definitions under new names and the migrate uses them. Everything is spelled out, but the agent must copy the old types exactly, and a wrong copy reads live state under the wrong shape.)
- Look again when: Agents regularly misread which side is old in first tries; the explicit marker is then worth its name.

### M5-07.5 When is the choice demanded, and what happens to an edit without one?

**Refused at the edit** (recommended). While a visual app is running, an edit that would change a stored type, compared with the running version, is refused, with the source untouched, unless it carries a choice or the waiting change set already has one. The refusal names the changed types and how each differs. A later choice replaces an earlier one, and a call may carry a choice alone.

```
edit: type Counter { count: Int = 0, step: Int = 1 }
refused: stored type Counter changed (field step added);
         send the edit with state: reset or state: migrate
source: unchanged     running app: still answers clicks
```

- Why: It is the paper's wording and the exit check's, and the agent learns at once, at the edit that causes it, that the state is at stake.
- Locks: The check in the Edit path (M5-28) and one more refusal reason in the tool interface. With no visual app running there is no state to lose, so no choice is asked; the swap-time rule under defaults covers a change stored that way.
- Not chosen: Stored, held until chosen (The edit is stored like any breaking edit and simply not installed until a choice arrives. No refusal to handle, but the source moves ahead of the app without the agent having said what happens to the state, and exit-check part 3 words it as a refusal.)
- Look again when: M4-03's interface makes a call that carries only a state choice awkward; the choice could then ride on a resend of the type edit.

### M5-07.6 What happens when a migrate fails to check, or traps when it runs?

**Old version keeps running** (recommended). The migrate is checked with the change set; an error in it blocks the install like an error in any definition. At the swap it runs on the old state, which it can only read. If it traps, nothing is installed: old code and old state carry on, the change set stays stored and waiting, and the trap is reported through the tools. It is not tried again by itself; the agent sends a corrected migrate or a reset.

```
edit: adds `top: String = ""` to Todos, with
migrate(old: Todos) -> Todos {
  Todos { draft: old.draft, items: old.items, top: old.items[0].title }
}
running state: items is empty
result: migrate trapped (out of bounds); not installed;
        the old to-do still shows 'Nothing to do' and answers a click
```

- Why: It is line 122's rule and the trap rule of line 159 applied to the swap: a trap leaves everything as it was.
- Locks: The order of steps in M5-30: compile everything, run the migrate, and only then swap code and state together. The source can be ahead of the running app until a good migrate or a reset arrives.
- Not chosen: Fall back to reset (If the migrate traps, the new code is installed with fresh state. The app always moves forward, but a bug in a migrate silently throws away the state the agent asked to keep.) · Stop the app (A trapping migrate ends the running app with an error. Nothing half-done, but the person watching loses the window over a mistake in a one-off function.)

### M5-07.7 What exactly does a reset do?

**Launch again, same window** (recommended). At the swap the state becomes the state type built from its field defaults, the same `start:` message as at launch is handled again if the app has one, element state and focus are dropped, and the view runs. The window stays open. A reset may be sent with any edit, or alone, whether or not a type changed. Costs: effects asked by the start handler run again.

```
edit request:
  replace: type Counter { count: Int = 0, step: Int = 1 }
  state:   reset
result: installed; state reset
tree: Text #0 "0" (was "3")
```

- Why: One meaning of 'fresh' for launch and for reset (M5-01), and an app that loads its data at start still works after a reset.
- Locks: What M5-29 applies. Messages still queued for the old version are dropped on a reset, and so are answers to effects asked before it, whenever they arrive.
- Not chosen: Defaults only (The state goes back to its defaults and the start message is not sent again. No repeated effects, but an app that loads its data in the start handler stays empty until it is closed and run again.)
- Look again when: A start handler with a costly or visible effect makes repeated resets a nuisance.

### M5-07 · settled without asking

- What 'old' is: The version that is running: the last one installed. It is not the base version an edit names, and not an earlier change set that never installed.
- Shape of a migrate: `migrate(name: OldState) -> NewState { body }`: no name of its own, exactly one read-only parameter, the result written in full. It is pure like any function, may call the new version's functions with new or unchanged values, and its body ends in the new state value.
- What can be done with an old value: If its type is unchanged it is an ordinary value. If its type changed, its fields can be read, it can be matched (the pattern names the old variants, every one of them), and a list of them can be looped over or indexed. It cannot be passed to a function, built-in ones included, compared or printed. Loosening this later breaks nothing.
- Exit-check sample: Starting each time from `Counter` without `step`: adding `step: Int = 1` bare is refused and the old counter answers a click; with `state: reset` it installs and shows 0; with `migrate(old: Counter) -> Counter { Counter { count: old.count } }` it installs and the count on screen stays.
- After a migrate succeeds: New code and new state are swapped in together between handlers and frames (line 305), the migrate is thrown away, and the view runs. The start message is not sent again. Element state and focus are kept by M5-02's matching, as on any frame.
- A hole in a migrate: Allowed by the checker; reaching it is a trap, so the old version keeps running.
- A migrate that does not finish: It is stopped the way a stuck handler is, which is M4-04's ruling, and then counts as a migrate that trapped.
- A migrate when nothing stored changed: Accepted: old and new types are then the same, and the function runs once on the state. Nothing is built to forbid it.
- No visual app running, and headless apps: The edit-time rule applies only while a visual app is running in dev mode. A headless program is rebuilt and rerun (line 126), and an edit made with nothing running has no state to keep; a state field sent then is ignored and the result says so.
- A change that reaches the swap with no choice: This happens when the change was stored before the app was launched, or reached the files outside the tools. The swap then waits: nothing is installed, the old version keeps running, and the edit result says a state choice is missing. A call that carries only a choice releases it.
- The app ends while a choice waits: The waiting migrate or reset is dropped with the app. The next launch starts fresh from the newest version in which every definition checks.
- Pointing the app at another state type: Editing the app block's `state:` line is a change of the state type and falls under the same rule. Adding a first `view:` to a headless app is not: there was no kept state.
- What the edit result says: One of: state kept, state migrated, state reset, refused for a missing choice (with the changed types and how each differs), not installed for a missing choice, or migrate trapped (with the trap report). A waiting migrate that a later edit made wrong is listed among what that edit broke. The agent never has to guess what happened to the state.
- Changes to the message type: The message enum is not stored unless the state holds a message, so adding a message needs no choice. A click or effect answer waiting from before the swap is turned into a message only when its turn comes, against the new code (M5-03's queue rule); if its variant is gone or has other fields, it is dropped and reported.
- Both back ends: The interpreter and dev mode follow the same rules; M5-30's done-when already asks for both. Release mode has no swap, so no migrate and no reset.
- The toolchain never migrates by itself: Every stored-type change needs a written migrate or a reset, as line 122 asks, even one that only adds a field with a default. Record defaults keep that case to one line: fields not named take their defaults. A choice that copies matching fields can be added later without breaking anything, if most migrates turn out to be plain copies.
- Stale state the checker cannot see: An edit to `update` or a view that changes what a field means, with no type change, keeps the state (line 114). The agent can send a reset with it. This is the rest of the gap line 437 describes, and it stays open.

### M5-07 · what this changes in TASKS.md

- M5-28: Reworded and grows: it compares the stored types of the running and the new version (reached through fields, lists, maps, payloads, shared values and pools; defaults, comments and layout ignored), refuses at the Edit call unless a choice is carried or waiting, only while a visual app runs, holds back a swap that changes a stored type with no choice, and adds the state field to the Edit call. Its pool case applies only if pools exist by then (M2-01 left them unbuilt).
- M5-29: Reworded: a reset is a relaunch in the same window (defaults, start message, element state and focus dropped, view) and may come with any edit or alone.
- M5-30: Grows, and needs splitting under caller rule 2: no task parses and checks a migrate, so a new task must parse the migrate text and check it against two versions of the types; M5-30 keeps it with the waiting change set, runs it before the swap and leaves the change set waiting if it traps.
- M4-03: Grows slightly, or is amended in M5: the edit schema gains one optional state field, a call that carries only that field, one more refusal reason and the result values listed here.
- M4-17: Unchanged in M4; its Edit path gains the stored-type check in M5-28.
- M5-27: Grows slightly: an input or effect answer queued before a swap is rebuilt against the new code or dropped and reported; on a reset the queue is emptied and later answers to old effects are dropped.
- M5-14: Grows slightly: the who-uses-what index must give the set of types reached from the state type, through built-in containers too.
- M5-41: Reworded slightly: the reference describes migrate as text carried by an edit, with the one sample, and not as a definition kind.
- M5-42: Reworded slightly: part 3 is met by the Counter edits listed under defaults; since the accepted reset installs the change, the script restores the old type or adds a second field before the migrate part.

## M7-01 · Decide how the owner writes and approves examples and grant changes

### M7-01.1 What makes an approval something an agent cannot produce?

**Signature from owner's key** (recommended). `paka approve` signs the approval file by running the system's `ssh-keygen -Y sign` with a key only the owner can unlock, and the gate checks it with `ssh-keygen -Y verify`. Costs: the owner keeps one key and types its passphrase for each approval, and OpenSSH must exist where the gate runs; no crate, no download.

```
$ paka approve examples/wc
...
Approve this item? yes / no: yes
Enter passphrase for /home/owner/.config/paka/approval_key:
approvals: serial 8, signed
```

- Why: It is the only option with a secret the agent does not hold. Merging many pull requests cannot double as a considered yes to each grant, and the M7 exit check asks for an approve command. It needs no dependency: the M7-04 note bans hand-written signature code, and the toolchain already runs clang as a system tool.
- Locks: The signature format (OpenSSH, namespace `paka-approval`), M7-04's trust anchor, and the owner's habit of unlocking a key. Swapping the signer later means signing one file again, so the lock is light.
- Not chosen: Owner's click on GitHub (The gate prints the pending items and the owner approves with a review or a button on GitHub, which the workflow reads back. No key, but there is no local check and an agent holding the owner's token can click too.) · Signature by a crate (The same signature made inside paka by an outside cryptography crate. Costs a download, vendoring, a trusted-list entry and a key format of our own.) · Merge is the approval (The approval file is unsigned and the owner's merge counts as approval. Nothing to build, but a pull request can change the file together with the code, and the owner, who does not read code, merges it.)
- Look again when: The passphrase per approval proves too heavy, or OpenSSH is missing where the gate must run. A hardware key with a touch button is the stronger form and needs no design change.
- Owner must still: Create a key used for nothing else, with a passphrase, outside `~/.ssh`: `ssh-keygen -t ed25519 -f ~/.config/paka/approval_key`. Never add it to a key agent (GNOME's offers the keys it finds in `~/.ssh`), never let the desktop remember the passphrase, never give it to an agent. Commit the public half as `approvals.pub` yourself before the gate becomes required, and check by eye that it is yours. OpenSSH 10.2 is at `/usr/bin/ssh-keygen` on the build machine (checked). Confirm it on the CI image (not checked here; no install expected).

### M7-01.2 Where do approvals live, and what exactly is signed?

**One signed file, repository root** (recommended). `approvals` holds a serial number and the approved text of every item, keyed by program path; `approvals.sig` signs the whole file and `approvals.pub` is the owner's public key. Cost: two pull requests that both carry approvals conflict, and the second is signed again after it is brought up to date.

```
serial 8
grants examples/wc wc [ReadFiles, Console]
grants examples/counter counter []
example examples/counter dec_stops_at_zero {
  var s = Counter {}
  update(&s, Dec)
  s.count == 0
}
file .github/workflows/gate.yml 31
  (the file's 31 lines follow, indented)
```

- Why: One file, one signature and one serial give one rule: a changed store must verify and carry a higher serial than the target branch's. That stops forgery and stops putting back an old signed store. Holding the approved text, not hashes, lets `paka approve` show old against new without git. The same file is the list of gated programs, so no second list can drift.
- Locks: Three record kinds (an app's grant list, an example, an owned file) and the store's text form (M7-04). The paths `approvals`, `approvals.sig`, `approvals.pub`. That a program is gated once the store has a record for its path, and that moving a program needs an approval.
- Not chosen: One signed file per program (The same file beside each program's source. Fewer conflicts, but a signed file can be copied to another program unless it names its path, and the key and the owned files still need a home at the root.) · One signed record per item (Each item has its own small signed file, so nothing conflicts. But any record the owner ever signed stays valid, so an agent can put back a grant list the owner later withdrew.)
- Look again when: Store conflicts between parallel pull requests become common (pillar 4 expects parallel agents, line 147). Or M4-48 gives programs a name better than a path, or a rule for finding every program in a tree, which would let the merge gate cover brand-new programs too.

### M7-01.3 What stops a pull request from weakening the gate itself?

**Gate runs from target branch** (recommended). The gate job uses the `pull_request_target` trigger, so GitHub takes the workflow file from the branch being merged into (`development` for agent work, `main` for the owner's merge). It builds paka from that branch and runs it, with a read-only token, on the pull request as merged, using that branch's key and store as the old state. The check is required on both branches. Cost: a pull request that changes the toolchain and also a gated program's use of that change must be split in two.

```
# .github/workflows/gate.yml
# on: pull_request_target    permissions: contents: read
# base   = checkout of the target branch (development or main)
# merged = checkout of the pull request merged into it
base/target/release/paka gate merged --base base
```

- Why: It is the only option where a green check means the gate already on the target branch judged the change. It is safe only because this job runs nothing from the pull request except Paka source in the interpreter: code there has no authority of its own (lines 163-166), examples perform no effects, the interpreter has limits (M2-06) and the job has a time limit.
- Locks: The CI trigger and the two-checkout layout in M7-10. The rule that no step of this job compiles C, runs a script or runs compiled code from the pull request. Every file under `.github/workflows/` is an owned file. The toolchain's own Rust source stays outside the store; its reviewers and the owner's merge guard it.
- Not chosen: Gate runs from pull request (The normal trigger; workflow and tool come from the pull request. Least to set up, but a pull request can rewrite the job or the gate's code and still show green.) · Owner checks paths by eye (The normal trigger, and the gate prints a warning when protected paths are touched. It rests on the owner reading every warning, which is the review the paper removes.)
- Look again when: The two-pull-request split slows toolchain work after M7. A change to the gate's Rust code slips through review; then the gate's source directory becomes an owned path. M7-10's planted test shows GitHub lets a second job named `gate` satisfy the required check. A merge from `development` into `main` carries toolchain changes together with gated programs that need them, which main's own paka cannot read; then merges into `main` are cut smaller, toolchain first.
- Owner must still: Mark the check `gate` as required, for administrators too and with no bypass: on `development` once M7-10's workflow is merged there, on `main` after the M7 merge. Before the gate counts as proof, agents work under a GitHub account of their own with write access only, which cannot merge into `main`, change settings or write check results. Approve each workflow file as an owned file in the first approval. Always run `paka approve` from a paka built from the branch the pull request targets, never from the pull request's own build.

### M7-01.4 How does an example become human-owned?

**Owner adopts by name** (recommended). The owner says in words what should hold, the agent writes an ordinary `example`, and the owner runs `paka approve` with its name, reads it and signs. No new syntax; the cost is that the owner reads a few lines of Paka per example.

```
$ paka examples examples/counter
dec_stops_at_zero   agent-written   pass
$ paka approve examples/counter --adopt dec_stops_at_zero
```

- Why: It needs no syntax, keeps `example` as the one evidence form (line 183), and the label exists only in the signed store. An example may be adopted before it passes, so the owner can fix the goal first.
- Locks: Little: ownership is a store record, so any other way of creating the record can be added later. M7-05's label is 'a record exists for this name'.
- Not chosen: Marker in the source (A keyword such as `owned example` marks it and the gate demands an approval for every marked one. The label then starts with the agent, against the exit check, and an agent can drop the marker to escape the gate.) · Cases in a data file (The owner writes inputs and expected outputs in a plain table and the toolchain runs them. No Paka to read, but it is a second evidence mechanism beside `example` and it cannot express a sequence of UI actions (line 183).)
- Look again when: The owner finds example bodies unreadable in practice; then a plain rendering or data-file cases are added on top. Also if M2-06 or M5-06 change what an example body is; this page follows GENESIS 3.6 until they are approved.

### M7-01.5 What does the approval of an example cover: its own text, or also the code it calls?

**The example's own text** (recommended). The store holds the example's canonical text, comments included; editing, renaming or deleting it needs approval, editing anything it calls does not. Cost: an example that leans on an agent-written helper is only as good as that helper, so the approve screen lists what it uses as not covered.

```
example dec_stops_at_zero {
  var s = Counter {}
  update(&s, Dec)
  s.count == 0
}
// covered: these five lines
// not covered: update, Counter
```

- Why: The example states what must hold; the code it calls is the code under test and must stay free to change. The remaining gap is made visible at approval time instead of being paid for on every edit.
- Locks: What M7-04 stores and compares, and the gate's cases in M7-07. Widening the cover later only adds pending approvals; it breaks nothing.
- Not chosen: Text plus what it calls (Also covers the text of every definition the example names. Closes the helper trick, but the function under test is one of them, so every change to it needs the owner.) · Everything it depends on (Covers the closure (everything reached, directly or through other definitions). Nothing can change unseen, and the approval is void after almost every edit.)
- Look again when: Human-owned examples are found resting on helpers that were later changed to make them pass.

### M7-01.6 When an agent changes a grant list or a human-owned example, when is it stopped?

**At merge and release** (recommended). The edit is stored and runs in dev mode; the edit result and the gate list each pending item, and merge and release are refused until the owner approves. Cost: in dev mode on the owner's machine a program can use a grant the owner has not seen yet.

```
{"pending": [
  {"kind": "grants", "program": "examples/wc", "app": "wc",
   "approved": ["Console"], "found": ["Console", "ReadFiles"]},
  {"kind": "example", "program": "examples/counter",
   "name": "dec_stops_at_zero", "change": "edited"}],
 "blocks": ["merge", "release"],
 "next": "The owner runs: paka approve. Or revert the change."}
```

- Why: It is what lines 167 and 187 say, and it keeps approval logic out of the run loops; the Edit tool only compares against the store to report. An agent that can run commands on the dev machine is not bounded by Paka grants there anyway.
- Locks: The pending-item shape that M7-08 prints and agents learn, and that the run loops never read the store. Tightening to run time later changes what dev-mode programs do.
- Not chosen: At the edit (The Edit tool refuses the change until it is approved. Tightest, but the agent stops dead on every grant change and the owner must be present during the work.) · At run time too (The edit is stored, but every run uses the approved grant list, so a new grant's effects are refused as traps until approved. Protects the dev machine, but the agent cannot try what it built and the runtime must read the store.)
- Look again when: Agents are run on machines where a dev-mode grant matters (untrusted input, no other sandbox); then 'At run time too' is the next step.

### M7-01.7 What exactly does the owner see when asked to approve?

**One item, old and new** (recommended). Items come one at a time, each with its own yes or no, and every word comes from the toolchain: a grant change shows the approved list, the new list and a fixed one-line meaning per grant; an example change shows the whole new body with changed lines marked, its status and the names it uses. Cost: many items mean many answers.

```
[1/2] examples/wc, app wc, grants
  approved: [Console]      now: [Console, ReadFiles]
  + ReadFiles: may read any file you can read
Approve this item? yes / no: yes
[2/2] examples/counter, example dec_stops_at_zero, edited, now: pass
    var s = Counter {}
    update(&s, Dec)
  - s.count == 1
  + s.count == 0
  uses: update, Counter (not covered by this approval)
```

- Why: Each approval is then a statement about one thing the owner actually saw, in text no agent wrote. Approvals are rare (grants and acceptance examples), so one answer per item is cheap.
- Locks: The hand-written expected texts of M7-06 and the fixed one-line meaning each grant must carry in the built-in table.
- Not chosen: One list, one yes (All pending items are listed and one answer approves them all. Fastest, but one dangerous item hides among harmless ones.) · With the agent's reason (As the first option, plus a sentence from the agent on why it made the change. Helpful context, but it is the one part of the screen an agent can use to mislead.)
- Look again when: M2-15 gives grants arguments (for example a path), or M5-06 gives UI examples a form that needs another rendering.

### M7-01 · settled without asking

- Where labels live: Only in the signed store. An example is human-owned when the store has a record for its program and name, otherwise agent-written. Source text carries no label.
- Finding the store: The store is the file `approvals` in the nearest directory at or above the program. Record paths are relative to that directory. The example listing, the Edit result, approve, gate and release all use this one rule. `paka build` never reads the store.
- Which programs are gated: A program is gated once the store holds a record for its path; the first approval of its grant list, even an empty one, puts it there. Test and conformance programs have no records and are not gated. A program with no records can be merged, but `paka release` refuses it.
- Grant comparison: Every app in a gated program needs a grants record, also for `grants: []`; a new app without one is a pending item. The app's list and the approved list are compared as sets, so order does not matter. An added grant and a removed grant are both pending items.
- What counts as a change to an example: Any difference in canonical text, comments included (layout per M4-47). A rename or deletion of a human-owned example is a pending item; the owner approves the removal or the agent puts it back.
- Owned files: A `file` record holds a path and the file's text, so the approve screen can show a line diff without git. The gate refuses when the file differs or is missing, and when a file under `.github/workflows/` has no record. Used for the workflow files and for `TRUSTED` (M7-02). C files named by a `foreign` declaration are held as SHA-256 (M4-07's hash) inside the grants record.
- The key file: The gate refuses any pull request that changes `approvals.pub`. To add, change or replace the key, the owner lifts the branch rule by hand once, commits the key and a store signed with it, and puts the rule back. The first key goes in this way, before the check is required. A lost key is handled the same way.
- Serial rule: The serial is a counter in the store's first line. The gate verifies the merged tree's store on every run. When that store differs from the target branch's, its serial must be higher; this stops an old signed store being put back. `paka approve` starts from the working tree's store, refuses if it does not verify, and adds one.
- Verifying the signature: The gate runs `ssh-keygen -Y verify` against the target branch's `approvals.pub` with one fixed signer name and the namespace `paka-approval` (a label inside the signature that stops reuse of signatures made for other things). It never uses the mode that checks a signature without checking whose it is.
- Signing: `paka approve` runs `/usr/bin/ssh-keygen` by that fixed path with an empty environment, so no key agent and no helper program takes part. ssh-keygen asks for the passphrase on the terminal itself; paka never sees it. The key path defaults to `~/.config/paka/approval_key`; a `--key` flag names another.
- Which paka the owner approves with: One built from the branch the pull request targets, in a checkout no agent works in, as the gate does. The pull request's own build is unmerged code and could show one thing and sign another. A pull request whose items need new toolchain code to be read is split, toolchain first.
- Who commits the signed files: The owner checks out the pull request's branch and runs the target branch's `paka approve` on it; the command rewrites `approvals` and `approvals.sig`. Either the owner or the agent commits and pushes them, since the signature, not the committer, is the proof. One pull request carries the change and its approval.
- The approve command and the terminal: `paka approve` reads the yes or no from a terminal only and exits with usage status 2 otherwise. It has no flag that answers for the owner. This stops accidents; the passphrase is what stops an agent.
- What the screen escapes: Every character outside printable ASCII in an example, a declaration or a file is shown in escaped form. A comment or a string cannot then hide lines, move the cursor or pass one letter off as another.
- What a refused agent sees at the gate: `paka gate` exits 1 and lists each pending or failing item in the same shape as the edit result, as text (one line per item in M1-03's line form) or as JSON. The last line says what the agent can do: revert the change or ask the owner to run `paka approve`.
- What the gate checks: The store's signature and serial, every record against the source, every workflow file against its record, and that every human-owned example of every gated program passes in the interpreter (the reference). A failing, trapped or unfinished one refuses. Agent-written examples never refuse. The CI job has a time limit, so an endless example fails the check.
- Stale checks: The check judges the pull request merged into its target branch as that branch was when the job ran. Two pull requests that each pass can still break a human-owned example together. `paka release` runs the gate again, so nothing ships on a stale pass. GitHub's 'branch must be up to date' setting closes it at the cost of a rebase per pull request; it is left off.
- Approving an example that fails: Allowed. Approval says the example is the right goal; passing is checked separately by the gate, so the pull request cannot merge until the example passes.
- Giving up ownership: `paka approve <program> --drop <name>` removes the record after the same screen and signature. The example then reads as agent-written.
- Local runs: On the owner's machine the gate and release use `approvals.pub` from the working tree and have no main to compare the serial with. This catches honest mistakes early; it is no proof against a program that rewrites the working tree. The check that cannot be bypassed is the required one on GitHub, which is why a release counts only from a clean checkout of main.
- What this does not stop: A program with the owner's rights that records the passphrase as it is typed or swaps the tools on the owner's machine; a hardware key closes the first. A change to the gate's own Rust code, which is guarded by review and the owner's merge, not by the store. The signature stops an agent that is wrong, misled or steered by hostile input.

### M7-01 · what this changes in TASKS.md

- M7-04: No cryptography crate and no download: it calls the system ssh-keygen by a fixed path with an empty environment. It stores approved text, a serial and three record kinds; file records hold text. Its tests add: a store signed by another key is refused, and an older signed store put back is refused.
- M7-05: The label is defined as 'a store record exists for this program and name'. The store is found by the one walk-up rule.
- M7-06: Grows. Per-item screen with fixed grant meanings, whole example bodies with marked lines, escaped non-ASCII, `--adopt` and `--drop`, terminal-only answers, signing through ssh-keygen and the serial bump. The owner's walk-through uses a paka built from `development`, since M7 has not reached `main` yet.
- M7-07: Grows and one case is narrowed. The command is `paka gate <tree> --base <tree>`; the store lists the gated programs. 'A new app whose grant list was never approved' means a new app in a gated program; a program with no records at all is refused by release, not by the merge gate. Adds the serial rule, owned-file records, the workflow-directory rule and the refusal of any key-file change. Runs human-owned examples in the interpreter.
- M7-08: Reworded. The agreed shape is the pending list above; an edit is stored, not refused, and the result names merge and release as blocked. The Edit tool reads the store to compare, without checking the signature.
- M7-10: Regraded from level 1 to level 4: a mistake in this job is silent (rule 7). It uses pull_request_target, checks out the target branch (`development` or `main`) and the merged pull request side by side, builds paka from the target branch, uses a read-only token and a time limit, and runs no pull-request code. Its test pull requests come after the workflow is on `development` and the check is required there. A third test pull request adds a second workflow with a passing job named `gate` beside an unapproved grant change and must still be blocked.
- M7-11: The owner first approves each sample's grant list, also the empty ones, since every app in a gated program needs a record.
- M7-13: The approval of a foreign grant also covers the declaration text and the hashes of its named files (see M7-02), inside the grants record.
- M7-16: The owner's approval of the trusted list is an owned-file record in the same store, given through `paka approve`.

## M7-02 · Decide the foreign-code model: declaration, named grant, process boundary, trusted list

### M7-02.1 Where is a C library declared?

**A seventh definition kind** (recommended). `foreign Name { ... }` in the program's own source names what to link or compile, the header, and each callable C function; its name is the grant. Costs one keyword reserved from M2 on, one section of the 8,000-token reference, and one more kind for every tool (the paper lists six, line 265).

```
foreign Sum {
  source: "sum.c"
  header: "sum.h"
  Checksum(text: String) -> Int = "sum_bytes"
}
// in the app block:
//   grants: [ReadFiles, Console, Sum]
```

- Why: Source stays the one place that says what a program is (line 273), and the tools, hashes and approval screen get the declaration for free. The declaration is short, so the owner can be shown all of it.
- Locks: The keyword `foreign`, reserved now because DECISIONS.md reserves later keywords everywhere. A seventh definition kind in parser, printer, database and reference. That the declaration's name is the grant name, Upper-case and unique in the program's one pool of names.
- Not chosen: A manifest beside the program (A separate non-Paka file lists libraries and functions. No new syntax, but it is a second file format, the Find, View and Edit tools do not reach it, and the checker must read outside the source.) · Read from the C header (The toolchain parses the header and offers every function it finds. Nothing to write, but it needs a C parser, exposes far more than was asked for, and the owner cannot be shown a short list.)
- Look again when: M4-48 or a later package decision (line 358) lets one program use another's definitions; a foreign declaration would then be shared, not repeated.

### M7-02.2 How does Paka code call a C function?

**One effect per C function** (recommended). Each line in the declaration defines a typed effect with an Upper-case name and the C symbol it stands for; it is used like `ReadFile`, with its arguments and then `then:` naming the message that carries the answer. Cost: effect names are bare and share the program's one pool of names (M2-02.2).

```
fn update(s: inout Total, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path)  => [ReadFile(path, then: Loaded)]
    Loaded(text) => [Checksum(text, then: Summed)]
    Summed(n)    => [Print("{n}"), Exit]
  }
}
// success path only, as in line 248; failures take M2-15's form
```

- Why: The checker sees argument and result types from the declaration alone, so a wrong call is an exact error at once (line 33). It adds no new idea: a foreign call reads like every other effect.
- Locks: That foreign calls never appear inside pure functions, the declaration's function-line form, and the effect names agents learn per library.
- Not chosen: One generic call effect (A single built-in effect takes the library, the C name as text and a list of arguments. Nothing new per function, but arguments are checked only when the call runs and a misspelt name is a run-time failure.) · Looks like a function call (`sum_bytes(text)` written inside a function. Shortest, but it either breaks the no-hidden-effects rule or needs the step-by-step notation the paper leaves open (line 355).)
- Look again when: M2-15 or a sequential-effects decision changes how built-in effects such as ReadFile are written, or how their failures arrive; foreign effects follow the same form.

### M7-02.3 What may cross the boundary between Paka and C?

**Four plain types, copied** (recommended). Arguments are Int, Float, Bool and String; results are Int, Float, Bool or nothing; each maps to exactly one C type and is copied, and no pointer, handle, struct or callback crosses. A C function of another shape needs a few lines of wrapper C beside the program, under the same grant.

```
// Paka:  Checksum(text: String) -> Int = "sum_bytes"
// The C function must be exactly:
int64_t sum_bytes(const char *text, int64_t text_len);
// Int = int64_t   Float = double   Bool = bool
// String = pointer + length, read-only, valid during the call,
//          with a zero byte after the end
```

- Why: It is the least unchecked code that still checks every call's types, and the M7 exit check needs only a test library. Wider shapes can be added later without breaking a declaration.
- Locks: The type table of the wire format and the generated glue (M7-14), and that library state lives on the C side, never in Paka values.
- Not chosen: One byte buffer each way (Every foreign function takes one block of bytes and returns one. The smallest glue, but every library needs a hand-written wrapper and the checker sees no types.) · Structs, pointers and handles (Records map to C structs, results may be pointers kept as opaque handles, and out-parameters are supported. Most libraries fit directly, but the glue is large, a stale handle is a silent error on the C side, and the reference grows.)
- Look again when: The first real library needs text or bytes back; String results and List arguments are the first additions. Also if M2-03 or M2-14 add a byte type.

### M7-02.4 Where does the C code run, and for how long?

**One host per library, kept** (recommended). The first call to a declared library starts one helper process (the host) for it; the host stays up until the app ends and answers calls one at a time, in order, so the library can keep state between calls. Cost: C-side state is not rolled back or migrated, and a hung call needs a time limit.

```
app starts          no host
first Checksum      host for Sum is started, answers
later Checksum      the same host answers
app ends            host is killed
```

- Why: It keeps the paper's default boundary (line 174) in all three modes and lets a small C wrapper hold what cannot cross. No in-process route is built until a measured need exists.
- Locks: The pipe protocol and host lifetime that M7-14 and M7-15 build, and that C-side state outlives handlers and code swaps.
- Not chosen: A new process per call (Each call starts a process and ends it. Nothing can go stale, but a library that must open something and use it later cannot work, and every call pays for a process start.) · In-process on request (A declaration may ask to be loaded into the app's own process for speed. One bad write in C then silently breaks every Paka guarantee (line 446 describes this loss in other languages).)
- Look again when: A measured workload shows the pipe round trip is too slow, a visual app needs a slow C call without freezing its window, or M2-15's concurrency ruling lets effects run in parallel.

### M7-02.5 What does the owner's approval of a foreign grant cover?

**Declaration and named files** (recommended). When an app lists a foreign grant, its approval record also holds the full text of that `foreign` declaration and the SHA-256 of every file it names; a system library is covered by name only. Cost: any edit to the declaration or its C files needs the owner.

```
[1/1] examples/sum, app sum, grants
  approved: [ReadFiles, Console]
  now:      [ReadFiles, Console, Sum]
  + Sum: runs C code with all your rights, in a separate process
      source sum.c (41 lines)   header sum.h
      Checksum(text: String) -> Int = "sum_bytes"
Approve this item? yes / no:
```

- Why: Everything an agent can change through a pull request is covered, because the host is built only from the named files. What only the machine's administrator can change is trusted like the operating system.
- Locks: What M7-04 stores for a foreign grant and what M7-06 shows. Pinning system libraries later only adds pending items.
- Not chosen: Grant name only (The store holds just the app's grant list. Least to build, but the declaration and the C files can change after approval with no pending item.) · Library bytes too (Also pins the SHA-256 of each system library file. Strongest, but every system update makes the gate refuse and the hash differs between machines.)
- Look again when: Releases must run on machines the owner does not control; then the library bytes need pinning or the library must be built from source.

### M7-02.6 What holds the host's unchecked code?

**Generated C, pinned clang** (recommended). The toolchain writes one small C file per declaration that includes the header and calls each function with real C types, plus one fixed C file for the pipe loop; clang builds them with warnings as errors, and one generated line per function fails the build unless the header's type is exactly the declared one. Cost: a foreign program needs clang wherever it is built or run from source, and this is the one place the project owns C text.

```
#include "sum.h"
// fails to build unless the header's type is exactly this
static int64_t (*const f0)(const char *, int64_t) = sum_bytes;
static void call_0(paka_req *r, paka_rep *p) {
  paka_str text = paka_get_str(r);
  paka_put_int(p, f0(text.ptr, text.len));
}
```

- Why: The C compiler already knows the calling rules and checks each call against the header, so the unchecked part is a few fixed lines read once. clang has been required since M1-19 and is pinned by M6-03; no dependency is added.
- Locks: The fixed C file's small interface, the clang flags, and that a host is built and cached per declaration, keyed by the declaration, its files and the compiler version. Interpreter and dev build it at the first call; release builds it at release time and ships it (M7-03).
- Not chosen: Rust crate with unsafe (A paka-host crate loads the library at run time and calls through hand-written unsafe stubs, one per argument shape. No clang at run time, but nothing checks the declaration against the header and the unsafe code is ours.) · An outside FFI crate (The host uses a library such as libffi to call any signature. Flexible, but it is a download, a vendored dependency and one more trusted-list entry, and it still checks nothing against the header.)
- Look again when: Programs with foreign code must run in the interpreter or in dev mode on machines without clang.

### M7-02.7 Where is the list of trusted libraries kept, and how does it change?

**One owned text file** (recommended). `TRUSTED` at the repository root, one line per library with name, version, licence, where it runs and purpose; it is the same approved-crates list M4-06 and M5-10 already check, made an owned file. Cost: every added or updated library, toolchain crates included, needs the owner's signature.

```
# name      version    licence                         runs in    purpose
cranelift   <version>  Apache-2.0 WITH LLVM-exception  toolchain  dev-mode code
vello_cpu   <version>  Apache-2.0 OR MIT               programs   drawing
parley      <version>  Apache-2.0 OR MIT               programs   text layout
libc        system     -                               programs   C runtime
# rows follow M4-01b and M5-08, both still open
```

- Why: Trusting a library is the owner's decision, like a grant, and the approval store already has the record kind for it. One list serves the vendoring check, the M7-16 test and the notices file, so no second list can drift.
- Locks: The file's name and line form, and that adding or updating a vendored or linked library needs the owner's signature.
- Not chosen: A table in toolchain source (The list is a Rust constant. One place less, but a pull request can add a line together with the crate it adds and nothing tells the owner.) · Derived from the vendor directory (Whatever is vendored is trusted and the list is generated. Nothing to keep by hand, and nothing for the owner to approve: adding a crate trusts it silently.)
- Look again when: Library updates become frequent enough that signing each version bump is a burden; the version column could then leave the owned part.
- Owner must still: Approve the first `TRUSTED` file through `paka approve` when M7-16 lands. Downloads of the libraries themselves stay with M4-06 and M5-10 and still need your approval there.

### M7-02 · settled without asking

- How failures surface: As a message, never as a trap. The answer carries the result or a failure, in whatever form M2-15 and M2-12 give a failed file read; both are still open. Failure kinds: host could not start, host died, timed out, bad reply. A C function's own error code is an ordinary Int result.
- A call without the grant: Same rule as every effect (M1-06.1, provisional until M2-15): refused when it runs, with one line naming the effect and the missing grant, and no host is built or started. The checker does not try to prove which app can reach which effect; that is the spreading of permissions through signatures the paper rejected (line 344). If M2-15 turns ungranted built-in effects into a check-time error, foreign effects follow.
- Before approval: In dev mode and the interpreter a newly listed foreign grant works before the owner has seen it (M7-01, 'At merge and release'), so agent-written C is compiled and run with the owner's rights. This adds nothing an agent with a shell cannot already do. The merge gate never compiles C, and release refuses until the grant is approved.
- Time limit: Each call has a fixed limit of 10 seconds. On expiry the host is killed, the call answers 'timed out', and the next call starts a fresh host. No per-library setting exists until a real library needs one.
- Waiting for an answer: The run loop waits for each answer, as it does for a file read (pending M2-15). A slow C call therefore freezes a visual app's window until it answers or times out. Because the answer is a message either way, moving the wait off the loop later changes no program.
- Host build, start and end: In the interpreter and dev mode the host is built and started at the first call, not at app start, and the built host is cached. A released program carries its hosts beside the binary and never builds one. The host is killed when the app ends. If it dies, the call in flight answers 'host died' and the next call starts a new one; C-side state is lost and the program learns it only through that failure.
- How the host is built: From copies of the named files in an empty directory. An `#include` of a nearby file the declaration does not name fails to build, so nothing outside the approved hashes gets in.
- Wire format: The host's standard input and output are the pipe. The host first moves them aside, then points its standard output at standard error and its standard input at nothing, so a library that prints or reads cannot corrupt the protocol. Frames are length-prefixed: a function number, then Int as 8 bytes, Float as its 8 IEEE bytes, Bool as 1 byte, String as length plus bytes. Exact layout is M7-14's.
- Declaration lines: `library:` names a system library to link; `source:` names C files to compile into the host; `header:` names the one header to include. At least one of the first two is required. `source:` and `header:` name files beside the program (in its directory, or next to a one-file program). Every name is a plain file name with no path and no leading dash. A system header is reached by including it from the named header.
- Function lines: Each gives the effect name, typed parameters, the result and the C symbol. Every foreign effect also takes the labelled `then:` argument after its own, as `ReadFile` does. An effect with no result still names a message, which arrives when the call has returned.
- Code swaps in dev mode: The host survives a swap of Paka code. An edit to the `foreign` declaration or its files rebuilds and restarts that host; calls in flight answer 'host died'.
- Examples and foreign code: Running an example never starts a host. Function examples only inspect the effects `update` returns. Whether UI examples perform effects at all is M5-06's ruling; if they do, foreign effects are still not performed.
- What is claimed: Nothing in section 3.5 is claimed for the C side. It has the user's full operating-system rights and may differ between runs. A foreign result is outside input, like the contents of a file.
- Trusted libraries and grants: Libraries on the trusted list run inside the program's process and need no grant. Every `foreign` declaration runs in its own process and needs its grant. There is no way to mark user-declared C code as trusted.
- Callbacks: None. C code cannot call into Paka or send messages on its own; it only answers calls.

### M7-02 · what this changes in TASKS.md

- M2-07: `foreign` joins the reserved words now, since DECISIONS.md reserves keywords kept for later milestones everywhere.
- M2-18: The reference lists `foreign` as reserved from M2. It gains a short `foreign` section in M7-13's pull request and must still pass its size test.
- M4-06: Its approved-crates list is written as the file `TRUSTED` in the line form above, so M7-16 makes that same file owned and no second list ever exists.
- M5-10: Adds its crates and their licences to the same `TRUSTED` file; its check reads that file.
- M7-13: Reworded and grown. It adds the `foreign` definition kind with its three line kinds and plain-file-name rule. 'Builds only with the grant' becomes 'the call runs only under the grant; without it the call is refused as every ungranted effect is'. The grants record holds the declaration text and file hashes.
- M7-14: Reworded. The unchecked code is one fixed C file plus generated C per declaration, emitted by Rust and built by the pinned clang with warnings as errors and one exact-type line per function, from copies of the named files in an empty directory. The Rust side only starts a process and moves bytes, so its `unsafe` flag likely drops while the level stays.
- M7-15: Grows slightly. Lazy start, one host per declaration, the fixed time limit, restart after death and the four failure kinds are now fixed; a released program finds its hosts beside the binary. It stays level 3 only while foreign effects use M4-36's decoder unchanged, as its note says.
- M7-16: Reworded. The trusted list is the existing approved-crates file `TRUSTED`, now an owned file with system libraries added, approved through the store. The notices file is generated from it and the vendor directory.
- M7-04: Grows. A grants record can carry a foreign declaration's text and the SHA-256 of its named files.
- M7-06: Grows. The grant screen for a foreign grant shows the declaration's lines and named files, and shows a changed C file as changed.
- M7-12: A release of a program with foreign code also builds and ships one host executable per declaration (see M7-03).

## M7-03 · Propose how human-owned examples run against the shipped release binary

### M7-03.1 How are human-owned examples run against the release binary?

**Compiled in, one switch** (recommended). `paka release` compiles the human-owned examples, function and UI, into the binary; started with the single reserved argument `--paka-examples`, the binary runs them with no window and no effects, prints each status and exits. Costs: examples add size, one argument is taken from every released program, and example bodies must be lowered for release.

```
$ ./wc --paka-examples
count_words_empty      pass
count_words_two_lines  pass
2 human-owned examples, 2 pass
$ echo $?
0
```

- Why: It is the only mechanism where function examples and UI examples both run in the very file that ships, with one way of running them. The evidence also travels with the binary: anyone holding it can run the switch again.
- Locks: The reserved argument, that human-owned examples are part of every released binary, and release lowering for example bodies and the UI example functions.
- Not chosen: Driven from outside (UI examples are replayed against the binary through the Look protocol; function examples cannot reach the binary and run only in interpreter and dev. Nothing extra in headless binaries, but half the evidence never touches release code and release binaries must ship a Look port.) · A twin test binary (A second binary is linked from the same compiled code plus the examples. The shipped file stays clean, but the tested file is not the shipped file, which is what line 187 rules out.)
- Look again when: M5-06 defines UI example functions that cannot run inside the app's own process, or M2-06 changes what an example body is; this page follows GENESIS 3.6 until both are approved. Also if binary size from examples becomes a measured complaint.

### M7-03.2 Which command ships?

**A new `paka release`** (recommended). `paka release <program> -o <dir>` runs the gate, builds, runs the examples in the binary and only then leaves the artefact. `paka build` stays as it is and is never gated; the cost is one more command.

```
$ paka release examples/wc -o dist/wc
gate: approvals serial 8, signature good, nothing pending
build: sha256 41c7...
examples in binary: 2 of 2 pass
released: dist/wc/wc  sha256 41c7...
```

- Why: Release is build plus checks, in one process with no step an agent can slip between, and nothing that uses `paka build` today changes.
- Locks: The command name, and that 'released' means 'has a release record written by this command'. The M6 note that `paka build` has no second spelling is reworded.
- Not chosen: `paka build` gated by default (`paka build` refuses without approvals unless a flag switches the gate off. No new command, but every existing test, fuzz and benchmark call must add the flag, and M7-12 requires them to pass unchanged.) · `paka build` plus a stamp (`paka build` is ungated and a separate `paka stamp` checks a finished binary afterwards. The binary can change between the two steps, so the proof has a gap by design.)
- Owner must still: Run `paka release` yourself, from a clean checkout of main. No release job in CI is built yet.

### M7-03.3 Which builds carry examples inside the binary?

**Release and test builds only** (recommended). `paka release` compiles in the human-owned examples named by the store it has just verified; `paka build` compiles in none and never reads the store, and its test flag `--all-examples` adds every example of the program. Cost: for a gated program, a plain build and a release are two different compiles.

```
$ paka build examples/wc -o wc                  # no examples, no switch
$ paka build examples/wc -o wc --all-examples   # tests and debugging
$ paka release examples/wc -o dist/wc           # human-owned examples
```

- Why: `paka build` stays a function of the source alone, with no hidden tie to the approval store. The other option buys nothing for evidence: conformance, fuzz and benchmark programs have no approvals, so they are built without examples either way, and three-mode example cases need `--all-examples` in both. Identical bytes between two builds are promised nowhere, and nothing here relies on them, because release never builds twice (next point).
- Locks: The test flag's name, that a plain build has no switch, and that examples are a compile unit of their own which calls the program's functions out of line.
- Not chosen: Every build of the program (`paka build` reads the store without checking its signature and compiles in the examples it names, so build and release take one path. Cost: build output depends on a file outside the source, and an approval changes the binaries that unrelated tests build.)
- Against: The author picked 'Every build of the program': with one build path, the everyday `paka build` of a gated program gives what release gives, so an agent can reproduce a release failure with the usual command and nothing differs between what is tried and what ships.
- Look again when: A failure shows up in a released binary that a `--all-examples` build does not reproduce.

### M7-03.4 How is the tested binary shown to be the shipped one?

**One build, hash, run, rename** (recommended). Release builds once into a temporary file inside the output directory, takes its SHA-256, runs it with the switch, takes the hash again, renames the same file into place and writes the text record `RELEASE` beside it. It never builds twice; a changed hash at any step refuses and leaves no artefact.

```
program    examples/wc
binary     wc  sha256:41c7...
compiler   paka <version>, clang 21
approvals  serial 8  sha256:77aa...
example    count_words_empty      pass
example    count_words_two_lines  pass
```

- Why: A rename does not copy or rebuild, so the file that ran the examples is the file that ships, and two hashes show nothing changed in between. It needs only M4-07's SHA-256.
- Locks: The record's file name `RELEASE` and its fields, which M7-11 and M7-12 test against.
- Not chosen: Build twice and compare (Test one build, ship a second, and require equal hashes. It needs builds that are identical to the last byte every time, which nothing in M6 promises.) · Signed release record (As the first option, and the owner's key signs the record. A stranger could then verify a release, but the owner types a passphrase per release and no one has asked for that yet.)
- Look again when: Releases are handed to people who must verify them without trusting the builder; then the record is signed with the approval key.

### M7-03 · settled without asking

- Function examples in the binary: Each example calls the program's compiled functions by symbol, out of line, so it runs real compiled code and the optimizer cannot fold the example away. Keeping those functions callable can change how the optimizer treats them. A copy the optimizer inlined into other callers is not what the example runs; three-mode conformance and fuzzing cover that code. The release record claims no more.
- UI examples in the binary: They run inside the binary against the real runtime with no window, as M6-18 already does. The app's own update and view code is called as in normal use, and the example's actions go to the element tree (GENESIS 3.3, pending M5-06). No Look port is needed in release.
- Effects during the switch: None are performed and no grant is used; no foreign host is started. This follows what M2-06 and M5-06 rule for examples in the other modes.
- Which examples: `paka release` compiles in exactly the human-owned examples the verified store names for the program. `--all-examples` compiles in every example and involves no label; `paka release` never passes it. A plain `paka build` compiles in none.
- The switch: `--paka-examples` must be the only argument and is taken before the app sees its arguments. It prints one line per example in the same shape as `paka examples` and exits 0 when all pass, 3 otherwise. `paka release` trusts the lines, not the status alone: every store-named example must appear once with `pass`. The form `--paka-examples=trees` also prints the tree text after each UI action, for M5-26's comparison.
- A trap inside an example: It is reported as `trapped` and the remaining examples still run. How the runner survives the trap belongs to the split-out lowering task.
- What release checks, in order: The gate for this program: signature, records, human-owned examples in the interpreter; with no main at hand the serial is recorded, not compared. That the program has records and at least one human-owned example; a release with no owner evidence is refused. The build, which refuses a hole (line 308). The examples in the binary. Any refusal leaves no artefact and exits 1.
- The artefact: One directory with the binary, one host executable per `foreign` declaration, the third-party notices file and the record `RELEASE` with the SHA-256 of every file. Hosts are built from the same file copies whose hashes the gate checked. Examples perform no effects, so the record lists hosts as built, not exercised. Hosts are found beside the binary at run time.
- Where release runs: On the owner's machine, from a clean checkout of main. No release job in CI is built yet. The record states the store's serial and hash so a release can be matched to a state of main. A release from any other tree proves nothing, because the local gate trusts the key in the working tree (M7-01).
- The merge gate and release binaries: The pull-request gate never builds or runs a release binary; binary examples run only inside `paka release`. This keeps compiled code from a pull request out of the gate job (M7-01).
- Conformance after M7-11: Example-status cases are compared in release by building with `--all-examples` and reading the switch's output. This ends the 'interpreter and dev only' exception in M6-06 and the M6 exit check.
- Caching: Results of examples run in the binary are not taken from the cache at release; they are always run. M7-09's cache keeps serving the interpreter and dev modes.

### M7-03 · what this changes in TASKS.md

- M7-11: Split as its note foresees. Lowering example bodies and the UI example functions for release as a compile unit of their own, with a runner that reports a trapped example and carries on, becomes its own level 4 task. M7-11 keeps the switch, the report and the comparison. The store is keyed by path, so the hole-free counter from M6-18a gets its own approval records.
- M7-12: Reworded; re-grade it, since the hash-and-rename sequence is where a silent mistake would live. The release command is the new `paka release`: temporary file, two hashes, rename and the `RELEASE` record. It also refuses a program with no records or no human-owned example. `paka build` stays ungated and does not read the store.
- M6 notes: The note '`paka build` is the release command since M1; there is no second spelling' is reworded: `paka build` is the optimizing build, `paka release` is the command that ships.
- M6-06: Its 'example-status cases in interpreter and dev only, until M7-11' exception, repeated in the M6 exit check, ends through the `--all-examples` test flag.
- M6-04: No change to what `paka build` produces. It gains only the test flag `--all-examples`, added by the split-out lowering task.
- M5-26: Its command gains release as a third back end by building with `--all-examples` and reading `--paka-examples=trees`, as M7-11's done-when needs.
- M7-16: The notices file is written by `paka release` into the artefact directory and listed in the release record.
- M7-15: Release output includes the host executables beside the binary, and the runtime looks for them there.
- M7-07: The gate must be callable for one program from inside `paka release`, without a base tree, as well as from CI.
