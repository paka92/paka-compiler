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
| M1-14 | How does a compiled function hand back its result and say that it trapped? | Status return, result by pointer |
| M1-14 | How are the effect list, an effect and a string laid out in memory? | Three 8-byte words |
| M1-14 | How does the runtime learn which app it is running? | Plain arguments to one entry |
| M1-14 | What happens when a program is linked with a runtime library built for a different layout? | Version in the entry name |
| M1-14 | Who makes a closed pipe an ordinary write error instead of death by signal? | Runtime ignores SIGPIPE at entry |
| M1-14 | What does the user see when the runtime itself fails? | Own status 4 |
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
| M2-03 | How wide is Int, and is there more than one integer type? | One Int, 64-bit signed |
| M2-03 | What is one position in a String? | Code points, one-character Strings |
| M2-03 | What do Int division and remainder give for negative numbers? | Truncate toward zero |
| M2-03 | How are wrapping arithmetic and bit operations written? | Named functions |
| M2-03 | How is a range of integers written? | Only a..<b |
| M2-03 | How does a Float become text? | Fixed digits only |
| M2-05 | What does it mean to assign a value to another name or store it somewhere? | Copy in meaning |
| M2-05 | What happens when a call passes the same data as inout and as a read-only argument? | Refuse, with a fix |
| M2-05 | When do two places overlap? | Fields apart, items not |
| M2-05 | Are the names bound by a match arm or a for loop values or places, and may the body change what is matched or walked? | Values; walked list frozen |
| M2-05 | Is a slice of a list or String its own kind of thing? | A slice is a value |
| M2-05 | How does a type contain itself? | Allowed, stored indirectly |
| M2-06 | Which parts of a trap report belong to the program's meaning? | Kind and position |
| M2-06 | When more than one trap is possible, which one is reported? | First in evaluation order |
| M2-06 | Which trap kinds exist? | Eight plain kinds |
| M2-06 | How does a hole get its type? | From position, else error |
| M2-06 | What is the body of an example, and when does it pass? | Block ending in Bool |
| M2-09 | How do ==, <, +, max and len work on several types when there is no overloading? | One closed built-in table |
| M2-09 | Do records, enums and collections have == without the agent writing it? | Every type, by structure |
| M2-09 | What can be a function value? | Named functions and variants |
| M2-09 | Can two function values be compared with ==? | Equal when same name |
| M2-09 | Which types have an order (< <= > >=, min, max)? | Int, Float, String only |
| M2-09 | Does + join two Strings? | Yes, + and += |
| M2-09 | Can a record, enum or collection be turned into text automatically? | No automatic text |
| M2-10 | Is there a shared immutable type in M2? | No shared type yet |
| M2-10 | What is recorded about pools and handles, and how is a graph written until then? | Not decided; list indexes |
| M2-12 | How does a function say it can fail or have no answer? | A value in the result |
| M2-12 | Which built-in types carry failure and absence? | Option and Result |
| M2-12 | Where is the line between a trap and a returned failure? | Bugs trap, outside data returns |
| M2-12 | Is there a short form to pass a failure up to the caller? | Postfix ? |
| M2-12 | Is there a form that stops the program where a value is expected? | unreachable() |
| M2-14 | In what order is a Map walked, and which types may be keys? | Key order, Int or String |
| M2-14 | How is a Map written, read and changed? | Literal, index and get |
| M2-14 | How does code walk a Map or a String when there are no tuples? | Lists from keys and chars |
| M2-14 | Do the built-ins that change a List change it in place or give back a new List? | In place, with & |
| M2-14 | How is a List sorted in an order the program chooses, and what is the result for any function it passes? | before function, fixed merge sort |
| M2-14 | How large is the built-in vocabulary? | About sixty rows, loops otherwise |
| M2-14 | Do the text built-ins follow Unicode rules or ASCII rules for case and white space? | ASCII rules only |
| M2-15 | How does a failed file read or write reach the program? | A second message, or: |
| M2-15 | What does a file failure carry? | One of six fixed texts |
| M2-15 | In what order are a handler's effects performed and their answers handled? | List order, answers queued |
| M2-15 | How do the command-line arguments reach the first message? | One String field per argument |
| M2-15 | Do the owner's two M1-06 rules on refused effects and failed Print stand, and are they trap kinds? | Both kept, as effect stops |
| M2-15 | How does a program end, and how does it choose its exit status? | Exit and ExitWith(status) |
| M2-15 | What happens when no message is left and the program has not performed Exit? | An error, status 3 |
| M2-15 | Can a headless program read standard input? | None in M2 |
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
| M6-01 | What may count as proof that a promise to the optimizer is true? | Emitter alone, shape checked |
| M6-01 | Who may remove a bounds or overflow check? | Only the optimizer |
| M6-01 | How large is the first table? | Small core, grown by measurement |
| M6-01 | What is the tail-call rule? | Never written, never promised |
| M6-01 | What does the IR say at a place the checker proved cannot be reached? | Return internal-error status |
| M6-01 | What may the IR promise about runtime functions written in Rust? | Only: does not unwind |
| M6-02 | Which optimization level and target CPU does paka build use, and do the Rust and C++ builds use the same? | Baseline x86-64, -O3 |
| M6-02 | How must the Rust and C++ versions of a benchmark be written? | Idiomatic safe code, tuned once |
| M6-02 | Which programs make up the set? | Eleven small own programs |
| M6-02 | What are the integer kernels compared against, given that Rust and C++ do not check overflow by default? | Both checked and unchecked Rust |
| M6-02 | Are measurement-only builds that leave out one family of checks accepted? | Accepted, after a clean run |
| M6-02 | What is timed, and how? | Whole process, timed outside |
| M6-02 | How does the published table state that Rust is optimized by LLVM 22 and Paka and C++ by LLVM 21? | State it; C++ is control |
| M6-02 | Which measured ratio is called 'parity'? | Parity means within 10% |
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

## M1-14 · Decide how compiled code and the runtime share data and calls

### M1-14.1 How does a compiled function hand back its result and say that it trapped?

**Status return, result by pointer** (recommended). Every compiled function returns a 32-bit status (0 = finished, anything else = stopped early) and writes its result through a pointer the caller passes last. Costs one extra argument and a store per call.

```
; fn update(m: Msg) -> List<Effect<Msg>>
define internal i32 @p_update(i64 %m, ptr %out) {
  store { ptr, i64 } { ptr @fx, i64 2 }, ptr %out
  ret i32 0
}
```

- Why: Every piece of the contract is written in the IR and in the Rust function type, with nothing left to a compiler's hidden record-passing rule. It is the fixed 'status return' taken literally, and the status travels in a register.
- Locks: The shape of every compiled function until M4-05 rules again: status in the return value, result through the last pointer. M1-16, M1-17 and M1-18 are written to it. In M1 compiled code only ever returns 0; the numbers of the trap kinds and where the position goes are M4-05's.
- Not chosen: One returned record (The function returns status and result together as one record, with no extra argument. But LLVM hands such a record back in registers while Rust's C convention returns a 24-byte record through a hidden pointer, and the IR text shows neither, so the two sides disagree silently.) · Status kept in the runtime (The function returns its result; a trap sets a flag in the runtime that the caller reads after each call. Costs a call or a memory read after every call, and a forgotten read loses the trap.)
- Look again when: M4-01b or M6 measurements show the out-pointer store costing real time on small functions; small results could then come back in registers.

### M1-14.2 How are the effect list, an effect and a string laid out in memory?

**Three 8-byte words** (recommended). A list is {pointer, count}, an effect is three 8-byte words (tag, pointer, length; Print = tag 0 with its text, Exit = tag 1 with zeros) and a string is pointer plus UTF-8 byte count with no ending zero. Costs 24 bytes per effect even for Exit.

```
@s0 = private constant [18 x i8] c"\6C\65\74\20\74\68\65\72\65\20\62\65\20\6C\69\67\68\74"
@fx = private constant [2 x { i64, ptr, i64 }] [
  { i64, ptr, i64 } { i64 0, ptr @s0, i64 18 },   ; Print(text)
  { i64, ptr, i64 } { i64 1, ptr null, i64 0 } ]  ; Exit
```

- Why: Every field is 8 bytes at an 8-byte boundary, so there is no padding to disagree on and the Rust side is one plain three-field record. All M1 data is constant, so no heap and no ownership rule is needed.
- Locks: Tag numbers (Print 0, Exit 1, the variant's position, the same rule as for messages) and the three-word record in M1-16's decoder and M1-18's template. Little beyond M1: M4-05 and M4-22 lay out every type afresh and say whether this stays.
- Not chosen: Packed tag and union (A one-byte tag followed by the variant's own fields, as Rust lays out an enum. Smaller, but padding and alignment must match on both sides, which is exactly the silent mismatch this task guards against.) · Runtime calls build the list (Compiled code calls a runtime function per effect and shares no layout at all. Effects stop being data, the runtime needs a growing buffer (a heap) in M1, and M1-16's decoding task has nothing to decode.)
- Look again when: M2-15 adds effects that need more than one text or a follow-up message (ReadFile with then:); the record then grows or M4-22's enum layout replaces it.

### M1-14.3 How does the runtime learn which app it is running?

**Plain arguments to one entry** (recommended). main calls one runtime function with five plain arguments (app name as pointer and length, update function, first message number, grant bits) and returns the status it gives back. No record to lay out; costs a longer call when apps gain more parts.

```
@name = private constant [5 x i8] c"\6C\69\67\68\74"
declare i32 @paka_rt1_run(ptr, i64, ptr, i64, i64)
define i32 @main() {
  %st = call i32 @paka_rt1_run(ptr @name, i64 5, ptr @p_update, i64 0, i64 1)
  ret i32 %st
}
```

- Why: It shares no extra layout and the entry is an ordinary function a test can call with a fake update function. Five arguments all travel in registers.
- Locks: The entry's name and argument order in M1-17 and in the emitted main. Cheap to change: one call in a text template.
- Not chosen: Constant app record (main passes a pointer to one constant record holding the same fields. Grows without changing the call, but adds a fourth layout that unsafe code must read.) · Fixed symbol names (The runtime looks for symbols with agreed names, such as paka_app_update. No arguments at all, but the library then has undefined symbols, so its own unit tests cannot link without a fake program.)
- Look again when: The app needs more than six arguments (state, command-line arguments in M2-15 or M4-05); a record is then the simpler form.

### M1-14.4 What happens when a program is linked with a runtime library built for a different layout?

**Version in the entry name** (recommended). The entry is called paka_rt1_run and any change to a layout or to the entry renames it to paka_rt2_run, so a stale library fails at link time with a missing symbol. Costs nothing at run time; someone must remember to bump the number.

```
$ paka build light.paka -o light
light.paka: error[<code from M1-06's spec>]: clang failed: undefined reference to `paka_rt2_run'
$ echo $?
1
```

- Why: The linker does the check, so there is no code to write and nothing to run. It turns the silent failure into a loud one before the program exists.
- Locks: The name paka_rt1_run, used by M1-17, M1-18 and docs/abi/light.ll. The rule to bump the number is written in the ABI document.
- Not chosen: Version word checked at start (main passes a version number and the runtime compares it first. A clearer message, but one more argument and a check that runs in every program.) · No guard (Trust that compiler and library are always built together. Nothing to build, and a stale library corrupts memory without a word.)

### M1-14.5 Who makes a closed pipe an ordinary write error instead of death by signal?

**Runtime ignores SIGPIPE at entry** (recommended). paka_rt1_run first sets the broken-pipe signal to 'ignore' with one hand-declared libc call kept in paka-rt's one unsafe module, so the write fails with an ordinary error. Costs one more unsafe call in M1-16.

```
(standard output is a pipe whose reader has gone)
light: effect Print failed: broken pipe
$ echo $?
3
(the line's exact wording is M1-06's spec's)
```

- Why: The rule about writes belongs with the code that writes, and the emitted program stays a template that knows nothing about signals. Setting a signal to 'ignore' installs no handler.
- Locks: Little. The call sits behind one safe function in M1-16's module, so M1-17 keeps its level.
- Not chosen: Emitted main ignores it (The IR's main calls libc's signal itself before calling the runtime. No Rust unsafe, but runtime policy then lives in every emitted program and the IR declares a libc function.)
- Look again when: M4-04 runs programs inside the tool server's process; the server then owns signal settings and the entry must not change them.

### M1-14.6 What does the user see when the runtime itself fails?

**Own status 4** (recommended). The entry catches every panic, prints the line 'app: internal error: reason' on standard error and returns 4; the same line and status are used when the runtime meets data it cannot decode, such as an unknown effect tag. Costs one more status number to document.

```
$ ./light
(Rust's own panic message, naming file and line)
light: internal error: runtime panicked
$ echo $?
4
```

- Why: It keeps 'the program stopped' apart from 'the toolchain broke', which tells the agent whether to fix its code or report a bug. Status 1 and 2 belong to the paka tool and 3 to traps.
- Locks: Status 4 for compiled programs. M2-15 must keep 3 and 4 out of the statuses a program may choose for itself, if Exit ever carries one.
- Not chosen: Report as a trap (A runtime panic exits with status 3 like any trap. One number fewer, but a compiler or runtime bug then looks like the program's own mistake, and the trap line M2-06 fixed needs a kind and a position that a runtime fault does not have.) · Abort the process (Build the runtime to abort on panic. Least code, but the process dies by signal, which M1-20 counts as a mismatch and M1-01 rules out.)

### M1-14 · settled without asking

- The IR owns main: docs/abi/light.ll defines 'i32 @main()' with no parameters; it calls paka_rt1_run and returns its result. Command-line arguments are not delivered in M1. The runtime never calls process exit itself, so the C library shuts down normally.
- Triple and module header: target triple = "x86_64-pc-linux-gnu". No datalayout line: clang fills it in from the triple. If clang 21.1.8 prints anything for that, the document adds the exact line clang itself writes.
- Calling convention: The C convention on both sides, with no other convention named in the IR. The Rust type of update is an extern "C" function taking (u64, pointer to a {pointer, u64} record) and returning i32. The entry takes it as an optional function pointer, so a null one is refused and never called.
- Messages: A message is the variant's position in its enum, counted from 0 in declared order, as a 64-bit number. M1 variants carry no payload and there is no state argument and no context pointer; M4-05 adds them.
- Grants: One 64-bit word of bits passed to the entry: bit 0 is Console, all others are zero. The runtime checks the bit before performing each effect and gives M1-06.1's line and status 3 when it is missing.
- Who owns the memory: Every pointer compiled code hands over points at constants that live for the whole process. The runtime only reads through them and never frees. The result record is a slot on the runtime's stack, set before the call to a value the decoder refuses (null pointer, count 1), so an update that returns 0 without storing is an internal error and not an empty list. paka-rt has no allocator.
- What the runtime refuses to read: An unknown tag, a null pointer with a non-zero length or count, a list pointer that is not 8-byte aligned, a count or length too large for the address space, a non-zero word in an unused field, or a null update pointer ends the run with the internal-error line and status 4. The runtime never reads past the count it was given, and it treats text as plain bytes: it never tells Rust that they are valid UTF-8.
- Status values: The process exits 0 after Exit, 3 for a refused or failed effect, 4 for an internal error. A non-zero status from update is an internal error in M1 (status 4): no M1 program can trap, and the trap line M2-06 fixed ('app: trap kind at file:line:col') needs a kind and a position that only M4-05 gives numbers to. M1-17 tests it with a fake update.
- A list that ends without Exit: The runtime does what M1-06's spec says for a program that stops without Exit. The document copies that status and adds no rule of its own.
- No println, and failed writes: All output goes through one runtime function that writes the bytes and a newline to standard output and returns the error instead of panicking. A failed write gives M1-06.2's line on standard error and status 3; a failure to write that line itself is ignored.
- Flushing: Standard output is flushed after every Print, so a failure is seen at the effect that caused it and later effects are dropped. Costs one system call per Print; M2-15 may rule otherwise for both back ends together.
- Exact error lines: The document copies the standard-error lines from M1-06's spec word for word. The interpreter and paka-rt each print them, and M1-20's comparison catches any drift. Only the internal-error line is the document's own.
- No link-time optimization across the two compilers: paka-rt is built to machine code by rustc with no lto and no linker-plugin-lto; clang gets no -flto. rustc's LLVM 22 and clang's LLVM 21 cannot read each other's intermediate files, and keeping them apart keeps Rust's optimizer promises out of Paka code.
- Link command: clang -O2 light.ll libpaka_rt.a, then the system libraries that 'rustc --print native-static-libs' names for paka-rt, written out in the document (on the build machine: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc); the system linker, no -fuse-ld.
- IR text rules: Opaque 'ptr' pointers; no attributes and no metadata; user functions are 'internal' and named p_ plus their Paka name, so none can clash with main or libc; constants are 'private constant' with every string byte hex-escaped; the result is stored as one whole record, so no getelementptr is needed. None of the banned words appears.
- Panics: The body of paka_rt1_run runs inside catch_unwind, so nothing unwinds into compiled code. paka-rt keeps Rust's default unwinding panics, as M1-01 already requires. No panic hook is installed: Rust's own message, with file and line, stays on standard error and the internal-error line follows it.
- Threads: Everything runs on the thread that entered main. The runtime starts no thread in M1.
- The stand-in runtime: One Rust file built with 'rustc --crate-type staticlib', shown in the pull request log with the link command and the printed line. It is not added to the workspace; M1-16 and M1-17 replace it with paka-rt.
- Heap ownership: None, as the note says: all M1 data is constant. Who frees what is M4-05's.

### M1-14 · what this changes in TASKS.md

- M1-06: Its code list needs one code for 'clang or the linker failed' beside 'missing clang', since a stale runtime library is reported under it; its exact standard-error lines and its rule for a program that stops without Exit are copied into the ABI document.
- M1-16: Decodes three shapes made only of 8-byte words (string, effect, list header), refuses the cases the document lists, takes no allocator through the interface, and holds both the exported entry and the one safe wrapper that sets the broken-pipe signal to ignore.
- M1-17: Its entry paka_rt1_run takes five plain arguments and is exported from M1-16's module, as its done-when says; the loop catches panics and returns 4, treats a non-zero status from update as an internal error, and prints the spec's lines; its fake update is an ordinary extern "C" function.
- M1-18: Emits internal functions named p_ plus the Paka name, stores the result as one whole record with no getelementptr, and writes no datalayout line.
- M1-19: Uses the document's link command with rustc's listed system libraries and never passes an LTO flag; a stale library shows up as the linker's undefined-reference error, reported as one diagnostic under the spec's code for a failed clang run, with status 1.
- M1-20: Status 4 never appears in an expectation, so an internal error is always a mismatch.
- M1-13: The interpreter flushes after every Print and prints the same standard-error lines as the ABI document lists, since M1-20 compares standard error.
- M4-05: Its kept-or-changed list has these rules to go through: status return with out pointer, three-word effects, plain-argument entry, version in the entry name, SIGPIPE ignored at entry, status 4, non-zero status as internal error, flush per Print, no cross-compiler LTO; it adds state, a context pointer and trap kind numbers.
- M2-15: If Exit gains a status, it must keep 3 and 4 out of a program's reach, and it rules whether flush-per-Print stays.

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
- Malformed input and missing files: Every counted task defines at least one error condition with an exact message and the fixed error status; missing-file errors are tested in the file-tools class and the file practice task only. The mechanism is M2-12's and M2-15's.
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
- Records and data transformation: Not a class of its own; covered by text processing and the reports class.
- Full record: docs/decisions/M2-01.md holds the seven classes with their types, effects, data shapes and example tasks, what is out, and the fairness risks the M3 report must state.

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
- Unused names: An unused local or parameter is a warning, not an error, so it never costs an edit. A `var` that is never reassigned is accepted without a message. `_` ignores a field in a pattern. The warning is printed by `paka check` only, and only when there is no error; `paka run` and `paka build` print none.
- One pool of names: All definitions in a program share one pool; with case enforced, Upper and lower names cannot clash. A duplicate definition is an error naming both places. Fields, labels, variant fields and the keys inside `app { }` belong to their owner and clash with nothing, so `update: update` (line 231) stays legal. All definitions are top level, order has no meaning, and a definition may be used before it appears.
- Identifiers and keywords: ASCII letters, digits and underscore. Only the first letter's case is enforced; the reference recommends snake_case and CamelCase. Keywords are reserved everywhere, including ones kept for later milestones (view, migrate). `_` alone is the discard target and the wildcard.
- Label details: The mark is `;` between bare and labelled parameters in these samples; the written grammar fixes the spelling. Bare parameters come first, labels are written in declared order. No default values for function parameters yet (adding them later breaks nothing); type fields keep theirs (line 201). Variant fields follow the function rule, so `Start(path)` builds and matches by position. Records are built with braces and field names (`Counter {}`, line 213). Trailing blocks such as `Col(gap: 8) { }` wait for M5-01.
- Comment details: `//` only, no block comments. A comment after the last definition of a file belongs to that definition. Comments never change meaning. Whether comment text enters a definition's hashes (lines 284-286) is left to M4-11.
- Which files make one program: The single file given to the command (as in M1), or every `.paka` file directly in the directory given. File boundaries and order carry no meaning (line 71). Packages stay undecided (line 358 and the M2 notes in TASKS.md). Declarations are read in file-name byte order, then source order.
- M1-06 rules kept or changed: Under the picks the light program parses and means the same, so no M1 run case changes; only the six planned reject cases stop holding (comment, hole, example, match, integer literal, labelled argument). Picking 'Always return' or 'Always with the enum' would change the light program itself. M1-06 is not written yet, so the line-by-line kept-or-changed list is made against it when it lands (M2-02 needs M1-06).
- Corrections to the proposals: The paper keeper cited lines 139 and 150; the text it meant is at lines 136/140 (local checking) and 147 (when two agents conflict). The engineer wrote that labels can only be added later by rewriting every call; under 'Bare unless marked labelled' adding them later breaks no user code.
- Left to other decisions: `&` at inout call sites and the overlap rule (M2-05); operators, string interpolation and wrapping arithmetic (M2-03); holes and example bodies (M2-06); function values and whether labels are part of a function type (M2-09); error-handling forms (M2-12); the built-in names table (M2-14); start, end and missing-grant rules (M2-15); canonical layout (M4-47).
- Label mark: The `;` in a parameter list (bare before it, labelled after it) is final, no longer provisional.
- Nesting limit: At most 64 open brackets: `(`, `[`, `{`, a type's `<` and an interpolation `{` each count; operator and `else if` chains do not. The 65th opener is the error `nesting-too-deep`.
- Full record: docs/decisions/M2-02.md holds every rule with samples, the grammar, the exact newline rule, the kept-or-changed table for the M1-06 rules, the M1 cases that stop holding and the new diagnostic codes.

## M2-03 · Decide the primitive types and the exact meaning of every operation

### M2-03.1 How wide is Int, and is there more than one integer type?

**One Int, 64-bit signed** (recommended). One integer type, -9223372036854775808 to 9223372036854775807. A result outside it traps. Costs: no small or unsigned types, so byte-level work is clumsy.

```
fn next_id(n: Int) -> Int {
  n + 1   // traps overflow when n is 9223372036854775807
}
```

- Why: It is the width Rust's i64 gives, covers every M2-01 task class, and keeps one row per operation. More types can be added later without breaking a program.
- Locks: Every Int row of the table, the overflow points of every program, the value layout in M4-22 and M6. Narrowing Int later breaks programs; adding types later does not.
- Not chosen: Sized family (Int plus I32, U8, U64 and others. Costs a conversion rule between every pair and many more table rows, and a model must pick a width at every signature.) · Unbounded Int (Integers grow as needed and never overflow. Costs heap numbers on every operation, which ends the native-speed promise for integer loops, and removes the overflow trap the paper lists.)
- Look again when: A task class or a benchmark needs bytes or unsigned numbers that Int cannot express at acceptable cost.

### M2-03.2 What is one position in a String?

**Code points, one-character Strings** (recommended). A String is a row of Unicode code points (character numbers). len counts them and text[i] gives a String of one character; there is no Char type. Costs: native back ends need a fast path so ASCII text indexes in constant time; other text may index in linear time.

```
fn initial(name: String) -> String {
  name[0]            // "É" for "Élan": a String of one character
}

example len_counts_characters {
  len("Élan") == 4
}
```

- Why: No operation can cut a character, so text never traps for a reason the agent cannot see on ASCII tests, and there is one text type to learn. On ASCII it means the same as Rust and TypeScript.
- Locks: len, index, slice, comparison and every String built-in in M2-14; the string layout in M4-32 and M6-11. Changing the unit later changes results of existing programs.
- Not chosen: UTF-8 bytes (Positions are bytes, as in Rust. Always constant time, but a slice that cuts a character in two must trap, so a program that passes on ASCII can stop on real text.) · Code points plus Char (As the first option, but text[i] gives a Char, a fifth primitive type with its own literal and conversions. Cheaper per character in native code; costs reference tokens and a conversion at every mix of Char and String.)
- Against: Bytes are constant time with no fast path to build, and they are what a toolchain written in Rust stores anyway.
- Look again when: M6 benchmarks show string indexing or one-character Strings dominating text-heavy code even with the ASCII fast path.

### M2-03.3 What do Int division and remainder give for negative numbers?

**Truncate toward zero** (recommended). / drops the fraction and % takes the sign of the left side, as Rust and the hardware do; TypeScript's % agrees. Costs a manual fix-up in wrap-around index code with negative numbers.

```
example divide_truncates {
  -7 / 2 == -3 && -7 % 2 == -1
}
```

- Why: It is what Rust does and what a model writes by hand in TypeScript (Math.trunc and %), so the same tests hold in all three languages. It is also the machine instruction.
- Locks: Results of every program that divides a negative number. Cannot change after programs exist.
- Not chosen: Floor (/ rounds down and % takes the sign of the right side, as in Python. Nicer for wrap-around indexes, but differs from both comparison languages and needs a fix-up after the machine instruction.)

### M2-03.4 How are wrapping arithmetic and bit operations written?

**Named functions** (recommended). Built-in functions: wrap_add, wrap_sub, wrap_mul, bit_and, bit_or, bit_xor, bit_not, shift_left, shift_right. No new syntax; costs longer text in hash-like code.

```
fn step(seed: Int) -> Int {
  let mixed = wrap_mul(seed, 6364136223846793005)
  bit_xor(mixed, shift_right(mixed, 33))
}
```

- Why: No experiment task depends on these (M2-01), so the cheapest form wins: nine table rows, no grammar. The choice is visible at every operation, and operators can be added later without breaking a program.
- Locks: Nine built-in names taken from users; the wrapping benchmarks in M6-28 are written with them. Little else.
- Not chosen: Extra operators (Symbols such as +% *% for wrapping and ^ << >> for bits. Shortest, but adds a dozen tokens and precedence rows, and << >> collide with generic brackets in the parser.) · Wrapping block (Inside wrapping { } the ordinary + - * wrap. Closest to the paper's wording, but the same operator then means two things depending on where it stands.)
- Look again when: M6 benchmark kernels or real programs show wrapping code is common enough that the long spelling costs tokens.

### M2-03.5 How is a range of integers written?

**Only a..<b** (recommended). One form, end not included, as in Swift and Kotlin. Used in for and in slices. Costs an edit when a model writes Rust's 0..n; the error carries the fix.

```
fn sum_to(n: Int) -> Int {
  var total = 0
  for i in 0..<n { total += i }
  total
}
```

- Why: The spelling says which end is left out, so the mistake is a check error with a fix instead of a wrong answer. One spelling per construct (line 51).
- Locks: Every for loop over numbers and every slice.
- Not chosen: Only a..b (Rust's spelling, end not included. Familiar from the comparison language, but a model with Kotlin habit reads it as including the end and gets a silent off-by-one.) · Both ..< and ... (Half-open and closed forms, as in Swift. Two spellings for one construct, against line 51.)
- Look again when: The experiment's practice runs show the Rust-habit error costing many counted edits; accepting 0..n as well is then a free loosening.

### M2-03.6 How does a Float become text?

**Fixed digits only** (recommended). The only way is fixed(x, digits: n), n from 0 to 20: the Float's exact value rounded to n decimals, ties away from zero. A Float inside "{ }" is a check error that names fixed; costs an edit when a model interpolates a Float by habit.

```
fn show(ratio: Float) -> String {
  let text = fixed(ratio, digits: 2)   // 0.125 gives "0.13"
  "ratio {text}"
}
```

- Why: One form, no exponent rules to define, and one tie rule in the whole language: the same as the round function. Adding a second form later breaks nothing.
- Locks: The rounding rule: ties away from zero. It matches Rust's round and TypeScript's toFixed, but not Rust's formatter on exact ties (2.5 to 0 digits is "3" here, "2" in Rust).
- Not chosen: Fixed digits, ties to even (The same single form, but an exact tie goes to the even digit, as Rust's formatter and C's printf do. Costs a second rounding rule beside the round function, and differs from TypeScript's toFixed.) · Shortest round-trip too ("{x}" also works and prints the fewest digits that read back as the same number. Convenient, but needs rules for exponents and whole numbers that match neither Rust nor TypeScript, and bit-exact tests for all of them.)
- Look again when: Reference solutions in M3-14 to M3-16 dispute many tests on exact ties.

### M2-03 · settled without asking

- The primitive types: Int, Float, Bool, String. No Char, no sized or unsigned integers, no byte type and no tuples: two values travel in a named type, which fits M2-02's flat patterns. Adding any of them later breaks no program.
- Int operations: + - * and unary minus trap overflow when the result does not fit; abs of the smallest Int traps overflow. / or % by zero traps divide_by_zero; smallest Int / -1 traps overflow; smallest Int % -1 is 0. Comparisons == != < <= > >= give Bool. += -= *= /= %= mean the same as the long form.
- Wrap and bit rows: wrap_add, wrap_sub and wrap_mul keep the low 64 bits of the exact result and never trap. bit_and, bit_or, bit_xor and bit_not work on the two's-complement bits. shift_left drops the bits pushed out and fills with zeros; shift_right copies the sign bit in; a count outside 0 to 63 traps out_of_bounds.
- Int literals: Decimal digits only: no underscores, no hex, one spelling. A unary minus written before a literal is part of the literal (a parser rule, so a - 1 still subtracts), which lets the smallest Int be written. A literal that does not fit is a check error.
- Float: IEEE 754 64-bit, round to nearest with ties to even, and no fused multiply-add (a multiply and an add done with one rounding; line 158). + - * / and unary minus never trap: 1.0 / 0.0 is infinity, 0.0 / 0.0 is NaN; += -= *= /= mean the long form; no % on Float. Literals need digits on both sides of the point, have no exponent form, give the nearest Float, and are a check error when that would be infinity.
- Canonical NaN bits: Every operation whose result is NaN gives exactly 0x7FF8000000000000, unary minus and abs included. NaN == x is false and NaN != x is true for every x, itself included; < <= > >= with NaN are false; -0.0 == 0.0 is true. If min and max take Floats (M2-09, M2-14), a NaN argument gives NaN.
- Float functions: sqrt (below zero gives NaN; sqrt of -0.0 is -0.0), floor, ceil, trunc, round (ties away from zero) and abs, all Float to Float. IEEE gives each exactly one result. Names are confirmed by M2-14.
- sin, exp, pow and the maths library: None in M2, as M2-01.4 ruled. No maths library is chosen. Adding them later breaks nothing but needs one pinned library.
- Conversions: None is implicit (line 47). Mixing Int and Float is a check error whose fix names to_float; an Int literal where a Float is needed is a check error whose fix writes it with a point (2.0). to_float(n) gives the nearest Float, ties to even. to_int(x) drops the fraction and traps bad_conversion when x is NaN, infinite or outside Int; 9223372036854775808.0 traps.
- Text of Int, Bool and Floats: Int prints in decimal with a leading - when negative; Bool prints true or false. fixed never uses an exponent: a large Float prints all its digits. It prints NaN, inf and -inf for those values and a leading - whenever the sign bit is set, so -0.001 to 2 digits is "-0.00". digits outside 0 to 20 traps out_of_bounds.
- Text to number: Parsing is a built-in whose name is M2-14's and whose failure form is M2-12's. Its meaning is fixed here: an optional -, then decimal digits, and for Float an optional point followed by digits; no spaces, no +, no exponent. An Int that does not fit is a failure, not a trap; a Float is the nearest Float, ties to even, and infinity when the text is too large.
- String details: Source files are UTF-8 and a String is a row of Unicode code points with no surrogates (the half-characters of UTF-16). Literals use double quotes with escapes \n \r \t \\ \" \{ and \u{hex}. == compares code point by code point with no normalisation (no merging of two spellings of one accent); < <= > >= order by code point.
- String positions: text[i] needs 0 <= i < len and text[a..<b] needs 0 <= a <= b <= len; anything else traps out_of_bounds. A position cannot be assigned or passed with &: a String changes only as a whole. What a read file that is not UTF-8 does is M2-15's.
- Interpolation: "{expr}" takes an Int, Bool or String expression that contains no string literal; a literal { is written \{. Float is rejected with a message that names fixed. User types wait for M2-09's to-text ruling.
- + on String: Not in this table: text is joined by interpolation or by M2-14's built-ins. Whether + also joins Strings is part of M2-09's ruling on operators over several types.
- Bool: && and || evaluate the right side only when needed; ! negates; == and != compare. Bools have no order and do not convert to Int.
- Operator precedence: From tightest: call, index and field access; unary - and !; * / %; + -; ..<; comparisons; &&; ||. Same-level operators group left to right, except that comparisons do not chain (a < b < c is a check error). Parentheses override.
- Evaluation order: Strictly left to right: operands, then arguments in written order, then the call; items of a list and fields of a built record in written order; only the chosen branch of if and match runs. In an assignment the index and field parts of the left side come first, then the right side, then the read (for += and the like), the operation and the write. A check happens when its operation runs.
- Ranges: a..<b is not a value: it is written only after the in of a for and inside [ ]. Both ends are Int and are evaluated once, before the first step. In a for, a start at or above the end runs zero times; in a slice, a start above the end traps out_of_bounds. Writing a..b or a...b is a check error whose fix is a..<b.
- Trap names used by this table: overflow, divide_by_zero, out_of_bounds, bad_conversion. M2-06 fixes the full list and the report.
- Changed in the final record: The full record is docs/decisions/M2-03.md. It differs from the text above in five places: NaN bits cannot be observed by any program, so compiled arithmetic does not canonicalise and test vectors match NaN as a class; the `\u{hex}` escape is dropped; a bare `}` in a string is an error with the fix `\}`; a source number literal has no leading zeros, while parse_int accepts them; an interpolation holds no `"`, `{` or `}`. Five M1 lex cases stop holding.

### M2-03 · what this changes in TASKS.md

- M2-08b: Dropped: no sin, exp or pow, so there are no maths rows to implement.
- M4-29: Dropped with M2-08b: no maths functions to call from compiled code.
- M6-08: Shrinks: no M2-08b vectors and no private maths names; the NaN and no-fused-multiply rules stay.
- M2-08: Grows a little: nine wrap and bit rows, code-point String rows, and fixed needs its own exact rounding with ties away from zero instead of Rust's formatter; no shortest float printing.
- M2-07: Token list gains ..< and loses any need for << >> ^ and character literals; number tokens have no hex, underscore or exponent; the string escapes are the eight listed.
- M2-11: A range is parsed only in a for header and inside [ ]; a unary minus on a literal is folded into the literal by the parser.
- M4-26: Reword: shifts are built-in functions whose bad count reports out_of_bounds, not a separate shift trap.
- M4-27: Reword its trap list to overflow, divide_by_zero, out_of_bounds and bad_conversion.
- M4-32: Grows: String positions are code points, so the runtime needs a constant-time path for ASCII text.
- M6-11: Grows the same way as M4-32 for release strings.
- M2-09: Must rule whether + joins Strings and how to-text feeds interpolation; Float equality is already fixed as IEEE.
- M2-14: Must supply names for text parsing, character number conversion, the Float functions and Map iteration without tuples.
- M3-04: Tasks with Float output must name their digits and keep clear of exact ties, NaN and infinity, or state Paka's rule; M3-11 writes the statements that way.

## M2-05 · Decide ownership: copies, inout exclusivity, borrows, recursive types, changing items in place

### M2-05.1 What does it mean to assign a value to another name or store it somewhere?

**Copy in meaning** (recommended). Every binding, field and list item is its own value; assigning or storing makes an independent copy. The compiler skips the copy when nobody can tell; the cost is that a copy of a big list is not visible in the source.

```
fn demo() -> Int {
  let a = [1, 2, 3]
  var b = a          // b is its own list
  append(&b, 4)
  len(a)             // 3
}
```

- Why: It is the mutable value semantics the paper names as its rule (line 427): no aliasing, no move errors, nothing to track in the checker, and the interpreter is a plain clone. The only cost is speed, which the experiment does not measure and M6 does.
- Locks: Programs may rely on a name staying usable after it was assigned elsewhere, so moving to 'Move, copy by name' later breaks them. A warning on large hidden copies or a 'take' parameter mark can be added later without breaking anything.
- Not chosen: Move, copy by name (Assigning moves the value and the old name is dead, as in Rust; a duplicate needs copy(a). Every copy is visible, but the checker needs move tracking through branches and loops and the agent meets use-after-move errors.) · Shared until written (The same meaning as the first option, plus a promise that a copy is cheap: every value carries a count and is copied on its first write, as in Swift. Costs a count on every value and a check on every write, which cuts into the native speed of line 85.)
- Against: A stricter rule can be loosened for free and this one cannot be tightened; the counter-case is that move errors cost counted edits before the go/no-go.
- Look again when: M6 measurements show hidden copies costing more than the estimated pointer-heavy gap of line 99.

### M2-05.2 What happens when a call passes the same data as inout and as a read-only argument?

**Refuse, with a fix** (recommended). The call is a check error; the fix is to bind the read-only part to a let first. An argument of type Int, Float or Bool, or any computed value (a literal, an operator result, a call result), is a value and never overlaps; costs an edit now and then.

```
fn grow(xs: inout List<Int>, more: List<Int>) {
  for x in more { append(&xs, x) }
}

fn twice(xs: inout List<Int>) {
  grow(&xs, xs)        // rejected: xs is inout and read in one call
  let old = xs
  grow(&xs, old)       // accepted
  append(&xs, xs[0])   // accepted: an Int is a value, not a place
}
```

- Why: It is the paper's rule, it keeps the copy visible, and the two exemptions remove the rejections that would be pure noise. Loosening to a silent copy later is free.
- Locks: Each call is checked against its own arguments. A call nested in an argument has finished before the outer call starts, so append(&xs, len(xs)) is accepted; the one extra rule is that a read-only place may not be changed by a nested call in the same argument list. M2-21 checks this, M6-22 attacks it, M6-23 may turn it into an optimizer promise.
- Not chosen: Copy silently (The compiler passes a copy of the read-only argument when it overlaps. Never an error, but a whole-list copy can hide inside a loop, and it departs from the paper's wording.) · Refuse any reuse (The variable behind an inout argument may not appear anywhere else in the call. One sentence to state, but it rejects everyday lines such as adding the sum of two earlier items.)
- Look again when: M6-22 finds an accepted call that overlaps at run time; the rule is then tightened before M6-23.

### M2-05.3 When do two places overlap?

**Fields apart, items not** (recommended). A place is a variable followed by field names and index steps. Two places overlap unless they start at different variables or part ways at two different field names; any two index steps on the same list count as the same. Costs a let when a call needs two items of one list.

```
// grow(xs: inout List<Int>, more: List<Int>) appends more to xs
type Pair { left: List<Int> = [], right: List<Int> = [] }

fn mix(p: inout Pair, rows: inout List<List<Int>>) {
  grow(&p.left, p.right)        // accepted: different fields
  grow(&rows[0], rows[1])       // rejected: two items of one list
  let other = rows[1]
  grow(&rows[0], other)         // accepted
}
```

- Why: It is a syntactic check of the kind line 427 describes, sound without knowing any index value, and it accepts the common state-update shape. Accepting more later is free.
- Locks: The accept and reject line agents learn and M6-22 tests. What may follow &: a var local, an inout parameter, their fields, and existing items of a List (a missing item traps out_of_bounds).
- Not chosen: Whole variable (Two places overlap whenever they start at the same variable. Simplest to check and can be loosened for free, but rejects passing one field inout and reading a sibling field, which state updates do all the time.) · Literal indexes apart (As the first option, and two different literal indexes are also apart. Accepts a few more calls, but adds a rule that only helps hand-written constants.)
- Look again when: The experiment's practice runs show many rejected calls on list items; a swap built-in or the literal rule can then be added.

### M2-05.4 Are the names bound by a match arm or a for loop values or places, and may the body change what is matched or walked?

**Values; walked list frozen** (recommended). A bound name is a read-only value. A match arm may assign to the matched thing, but a for body may not change the list it walks: that is a check error whose fix is to walk by index or bind a copy first. Costs an edit when a model appends to the list it is walking, and no in-place change of a variant's fields.

```
enum Shape { Dot, Rect(w: Int, h: Int) }

fn widen(s: inout Shape) {
  match s {
    Dot        => {}
    Rect(w, h) => s = Rect(w + 1, h)   // accepted: w and h are values
  }
}
// for x in xs { append(&xs, x) }        rejected: xs is being walked
// for i in 0..<len(xs) { xs[i] += 1 }   accepted
```

- Why: A loop that appends to its own list means one thing in TypeScript, another in Swift and is refused in Rust, so the checker says so instead of picking one silently. Lifting the rejection later is free; taking a meaning back is not.
- Locks: Little: the rejection can be lifted later without changing any program. Bound names stay values; place bindings can be added later under their own spelling.
- Not chosen: Values; loop sees old items (A bound name is a read-only value, and a for loop walks the list as it was at the start, so the body may change it. No extra rule, but a queue loop that appends to its own list stops early without a word, and the compiler copies the whole list.) · Places when changeable (If the matched thing is a var or inout, bound names can be assigned and passed with &. Trees can be changed in place, but the matched thing must then be frozen inside the arm, a second exclusivity rule.) · Frozen in match too (As the first option, and an arm may not assign to the matched thing either. One rule for both, but it rejects the usual state-machine step, where an arm sets the state it matched on.)
- Against: The author picked 'Values; loop sees old items': it follows from 'Copy in meaning' with no new rule, and it is what Swift does.
- Look again when: Practice runs show many rejected loops, or tree-changing tasks and M6 pointer-heavy benchmarks show rebuild costs that an in-place binding would remove.

### M2-05.5 Is a slice of a list or String its own kind of thing?

**A slice is a value** (recommended). xs[a..<b] is an ordinary List (or String) value with those items. Passed straight to a read-only parameter it is not copied; stored, it is a copy. No slice type and no inout slices: in-place work on a part takes the list and two indexes.

```
fn total(xs: List<Int>) -> Int {
  var sum = 0
  for x in xs { sum += x }
  sum
}

fn tail_total(xs: List<Int>) -> Int {
  total(xs[1..<len(xs)])     // a List value; passed without a copy
}
```

- Why: It keeps line 91's saving without a new type or a new rule; the agent writes what it would write in any language. In-place work by index is what models already do in Rust and TypeScript.
- Locks: Read-only List and String parameters must accept a borrowed part in native code (M4-05, M4-34). In the overlap rule a slice of a place counts as that place, so grow(&xs, xs[0..<2]) is rejected. Adding inout slices later breaks nothing.
- Not chosen: A Slice type (Slice<T> is a view that can be a parameter but cannot be stored or returned. Makes the no-copy promise visible, but adds a type with its own restrictions and two ways to take a list.) · Inout slices too (As the first option, and &xs[a..<b] passes a part for in-place change. Useful for sorts, but needs a rule for a callee that changes the part's length.)
- Look again when: M6 benchmarks need in-place work on parts of an array and the index form blocks the optimizer.

### M2-05.6 How does a type contain itself?

**Allowed, stored indirectly** (recommended). An enum variant may hold its own enum, directly or through other types; the compiler stores such fields behind a pointer. The tree is one owned value: copying it copies all of it. Costs a hidden allocation per node.

```
enum Expr { Num(value: Int), Add(left: Expr, right: Expr) }

fn eval(e: Expr) -> Int {
  match e {
    Num(value)       => value
    Add(left, right) => eval(left) + eval(right)
  }
}
```

- Why: The agent writes the type the way it thinks of it and learns no extra word. Layout already belongs to the compiler and to the interface hash (line 285).
- Locks: The agent never writes the indirection; which fields get the pointer is M4-22's. A type with no finite value is a check error: a record that holds itself with no enum or collection between, or an enum whose every variant holds it.
- Not chosen: Marked indirection (The agent writes a mark such as Box<Expr> where the type recurs, as in Rust. The cost is visible, but it adds a built-in type and wrapping at every construction.) · Only through List (A type may hold itself only inside a List. Nothing new to build, but a two-child node becomes a list that the checker cannot see has two items.)
- Look again when: Copies of deep trees hit the stack limit in real programs, or rebuilt trees dominate M6's pointer-heavy benchmarks.

### M2-05 · settled without asking

- The exact overlap test: A place is a variable followed by field, index and slice steps; parentheses change nothing. Two places overlap unless they start at different variables or their steps first differ at two different field names. Index and slice steps on one list always count as the same.
- The exact call rule: In one call: (1) an argument written with & may not overlap any other place argument of that call, except a read-only one of type Int, Float or Bool; (2) a read-only place argument of any other type may not overlap a place written with & inside another argument of that call. The operands of an operator count as read-only arguments. Anything that is not a bare place (a literal, an operator result, a call result, an if or match value, a built record or list) is a fresh value and overlaps nothing.
- The & mark at call sites: Required on every inout argument and nowhere else, as in the paper's sample (line 214). A missing or extra & is a check error with a fix.
- What may follow &: A var local or an inout parameter, with any number of field steps and List index steps. Rejected: a let, a read-only parameter, a name bound by for or by a pattern, a String position, a slice, and any computed value.
- Parameter kinds: Two: read-only (the default, passed without copying) and inout. No 'take' or 'sink' kind; a callee that stores a read-only parameter copies it. Adding a third kind later breaks nothing.
- Borrows: There is no borrow or reference in any type. A read-only parameter cannot be assigned or passed with &; returning or storing it copies it. So nothing borrowed can be stored or returned (line 91) and no lifetime exists.
- Changing items in place: xs[i] = v, xs[i].count += 1 and &xs[i] work on var locals and inout parameters, to any depth of fields and items. A let is fixed all the way down. Two items are swapped by three assignments.
- Results: A function returns an owned value. Returning a local moves it; returning a parameter or part of one copies it.
- When a value ends: A value is released when its owner goes out of scope or is overwritten. Nothing runs at that moment that a program can observe; there are no destructors, and releasing never traps.
- Deep values: Copying, comparing or releasing a deeply nested value must never crash the process. A back end releases it with a loop; a copy or comparison that is too deep for the back end traps stack, and like every stack trap its place is not compared between back ends.
- When the inout item is looked up: All arguments are evaluated left to right first, the index of &xs[i] among them. The item itself is looked up, with its bounds check, when the call starts; a missing item traps out_of_bounds at that [.
- Assignments that call with & on the same variable: Allowed and defined by the evaluation order of M2-03: xs[i] = f(&xs) runs f first and then stores, with the bounds check at the store. An assignment holds no access across the call.
- Map entries, swap and pools: Whether &m[key] is a place and what a missing key does is M2-14's and M2-12's; a swap built-in for two items of one list is M2-14's. Pools are not built (M2-01.1), so no pool rule is set.
- Shared immutable data: Not needed for meaning under these picks; it is a cost matter only. M2-10 still rules on it with its tree sample.
- Accept and reject lines: the & target: After let a = [1], append(&a, 2) is rejected; after var a = [1] it is accepted. In fn f(xs: List<Int>) the line append(&xs, 1) is rejected; with xs: inout List<Int> it is accepted and the caller writes f(&ys).
- Accept and reject lines: one call: With user functions that take the marked arguments inout: swap_lists(&p.left, &p.left) and grow(&xs, xs[0..<2]) are rejected; swap_lists(&p.left, &p.right), append(&xs, len(xs)) and push(&st, pop(&st) + pop(&st)) are accepted.
- Accept and reject lines: the order rule: merge(xs, take(&xs)) is rejected: xs is read and then changed before merge starts. let old = xs followed by merge(old, take(&xs)) is accepted.
- Accept and reject lines: types that hold themselves: type Node { next: Node } and enum Chain { Link(next: Chain) } are rejected: no finite value. type Node { kids: List<Node> = [] } and enum Chain { End, Link(next: Chain) } are accepted.
- Changed in the final record: The full record is docs/decisions/M2-05.md. All six picks stand. It is stricter than the text above in two places: an assignment inside a sibling `if` or `match` value counts as a write for the call rule, and the call rule also covers operators, list and map literals, record builds and interpolation parts. `&` is allowed only as a whole call argument. String is not exempt from the call rule and a `for` takes no snapshot; both can be loosened later without breaking a program.

### M2-05 · what this changes in TASKS.md

- M2-21: Shrinks: no move or use-after-move analysis; it checks what may follow &, the two call rules, and that a for body does not change the list it walks.
- M2-16: Gains one rule: reject a type that has no finite value.
- M2-26: The interpreter copies on bind and store, needs no snapshot for a for loop, releases deep values with a loop and counts depth when it copies or compares; no borrow machinery.
- M4-33: Carries the cost side: skip the copy at a last use, and copy a matched value's fields only when the arm changes what was matched.
- M4-34: Shrinks: no inout slices; read-only slices of List and String are passed as borrowed parts.
- M4-05: Must give read-only List and String parameters a form that can point at part of a caller's value.
- M4-22: Must implement indirect storage for types that contain themselves and say which fields get the pointer.
- M6-22: Reword its count: slices appear only as read-only arguments and there are no pool nodes while pools stay undecided.
- M6-22a: The detector checks two things: at each call start no inout argument shares storage with another argument, and no read-only place argument was written between its evaluation and the call's return.
- M2-14: Must decide a swap built-in and whether a map entry can be passed with &.
- M2-10: Shared data is now only a cost question, since copies already give the same meaning.
- M2-40: Shrinks to the overlap, order-rule, walked-list, place and recursive-type lines; no move cases, no pool cases.

## M2-06 · Decide traps, holes and what an example means

### M2-06.1 Which parts of a trap report belong to the program's meaning?

**Kind and position** (recommended). The trap kind and the file, line and column of the token that names the failing operation are meaning. The detail line and the call chain are help and may differ between back ends; costs one constant per trap site in native code.

```
$ paka run avg.paka data.txt
avg: trap divide_by_zero at avg.paka:14:11
  0 / 0
  in mean, called from update
$ echo $?
3
```

- Why: It is what TASKS.md already compares, it tells the agent where to look, and it leaves the optimizer free to inline. The help lines cost the interpreter nothing.
- Locks: The position rule: the operator for arithmetic, the [ for an index or slice (also in &xs[i]), the function name for a built-in call or assert, the ?? for a hole; a trap inside a built-in is reported at the user's call. Every native back end carries it.
- Not chosen: Kind only (Only the kind is compared. Lets the optimizer merge trap sites, but two back ends may point the agent at different lines for the same run.) · Everything shown (Kind, position, values and call chain are all meaning. Best feedback guaranteed, but inlining and tail calls in release would have to keep every frame.)
- Look again when: M6 shows per-site positions blocking an optimization that matters; position could then drop to line only.

### M2-06.2 When more than one trap is possible, which one is reported?

**First in evaluation order** (recommended). The trap reported is the one the reference interpreter reaches first under M2-03's order. Stack and memory traps are the exception: where they strike differs by mode (line 161). Costs: release code may not reorder checks unless it can still name the first one.

```
fn pick(xs: List<Int>, i: Int) -> Int {
  xs[i] + xs[i + 1] * 2
}
// order: xs[i], i + 1, xs[i + 1], *, +. The first that fails is the trap.
```

- Why: It is the only option that is one meaning. Release code can still hoist a check: functions are pure and a trap drops the handler's effects (line 159), so after a failed early check it may rerun the exact path to find the first trap.
- Locks: M6-01 takes it as given; M6-19 may remove or hoist checks only with that guarantee. For stack and memory traps only the kind is meaning.
- Not chosen: Any possible trap (A back end may report any trap the run would hit. Frees the optimizer fully, but the same program and input can give different reports, which breaks line 161.) · Fixed rank by kind (When several traps are possible in one statement, a fixed order of kinds decides. Looks orderly, but the interpreter would have to run past a trap to find the others.)

### M2-06.3 Which trap kinds exist?

**Eight plain kinds** (recommended). overflow, divide_by_zero, out_of_bounds (index, slice, shift count, digits), bad_conversion (Float to Int), assert, hole (carries its name), stack, memory. stale_handle is reserved until pools exist; costs eight names in the reference.

```
wc: trap out_of_bounds at wc.paka:9:14
  index 5, length 5
wc: trap hole at wc.paka:12:3
  ??scan
```

- Why: Each kind names a different mistake and the detail line carries the rest. It is the paper's list plus the two cases the paper's wording does not fit.
- Locks: The kind names, which tests and agents match on. Adding a kind later is cheap; renaming or merging breaks expectations in every conformance case.
- Not chosen: Paper's list only (Division by zero and failed conversion are folded into overflow. Fewer names, but 'overflow' for 1 / 0 sends the agent looking for the wrong bug.) · One kind per operation (Separate kinds for index, slice, shift, negative length and so on. Most exact, but each is one more thing every back end must tell apart, and the detail line already says it.)
- Look again when: M2-12, M2-14 or M2-15 needs a kind this list lacks (they may add; they may not rename).

### M2-06.4 How does a hole get its type?

**From position, else error** (recommended). ??name takes the type its position expects: a parameter, a field, a declared local, a result, a condition, a branch or operand whose other side has a type. Where nothing is expected it is a check error that says to bind it to a let with its type written. As a statement on its own it has no value.

```
fn price(n: Int) -> Int {
  let base: Int = ??base_price    // type written, so the hole is Int
  if n > 10 { ??bulk } else { base * n }
}
// let x = ??later   is rejected: write the type
```

- Why: It is the paper's sentence made exact, with one spelling, and the code around a hole stays fully checked. The error tells the agent exactly what to write.
- Locks: Hole checking in M2-20 and the placeholder rule M5-06 builds on. Holes stand for expressions and statements only, not for patterns, types or whole definitions.
- Not chosen: Type may be written (As above, and a hole may also state its type: ??later: Int. Never stuck, but two spellings for a hole.) · Fits anything (A hole with no expected type gets an unknown type that matches everything it touches. Never an error, but mistakes around the hole go unreported until it is filled.)
- Look again when: Agents in M4 or M5 often hit the no-expected-type error.

### M2-06.5 What is the body of an example, and when does it pass?

**Block ending in Bool** (recommended). The body is statements followed by one Bool expression: true is pass, false is fail. When that expression is a comparison, the fail report shows both sides. One example makes one claim; costs more examples for several claims.

```
example dec_stops_at_zero {
  var s = Counter {}
  update(&s, Dec)
  s.count == 0
}
// dec_stops_at_zero fail at counter.paka:15:11
//   left: -1  right: 0
```

- Why: It keeps the paper's sample as written, needs no new form, and keeps 'fail' (the claim is false) apart from 'trapped' (the code stopped).
- Locks: Every example's shape, the four statuses, and what M5-06 extends for UI examples. Allowing more forms later breaks nothing.
- Not chosen: Assert lines inside (The body is statements with assert(...) lines and no final value; it passes if it runs to the end. Several claims per example, but a failed claim then looks like a trap, and the paper's sample changes.) · Call and expected value (A fixed two-part form: an expression and the value it must equal. Easiest to report, but cannot express steps on changing state, which the paper's own sample needs.)
- Look again when: M7-01 finds owners need several claims per example to keep acceptance examples readable.

### M2-06 · settled without asking

- Trap report for a headless program: One line on standard error, 'app: trap kind at file:line:col', then optional indented help lines; exit status 3. It is the same family as M1-06's effect lines ('light: effect Print refused: ...'); the wording of help lines is not fixed. What a handler trap does to state and effects is M2-15's.
- Position format: The file's own name without its directory, then the 1-based line and column in the file's text, in the form M2-04 uses. A program is one file or one flat directory, so the name is unique, and the position is the same wherever the program is built or run from.
- Interpreter limits: The 10,001st nested call traps stack; a single List, Map or String growing past 16,777,216 items traps memory; no step or time limit. Both numbers are the interpreter's own, not part of meaning, and can change without breaking a program. 10,000 is near what TypeScript runs get, so recursive tasks stay fair; raise it when a task or real program needs more.
- assert: A built-in assert(cond) with no result; a false condition traps assert. No message argument: the position names it. A form that stops in a value position is M2-12's and would reuse this kind.
- Effect refused or failed: M1-06 ends the program with one line and status 3, like a trap. Whether these become trap kinds or stay separate is M2-15's; until then they keep M1-06's lines.
- Hole names: ??name with a lower-case name, required. Names need not be unique; the report gives name and position.
- Where holes run: paka check accepts holes, lists them as notes and exits 0. The interpreter and dev mode run them: reaching one traps hole. paka build refuses the program and names each hole (line 308).
- Example statuses: Four, computed by the toolchain only (line 185): pass, fail, trapped (any trap but hole), unfinished (a hole was reached). A hole that is not reached does not matter. If the program does not check, no example has a status: paka examples prints the errors and exits 1.
- paka examples output and status: One line per example on standard output, 'name status', with ' at file:line:col' and help lines for fail, trapped and unfinished; for example 'price_bulk unfinished at shop.paka:3:15'. Order is by name. Exit 0 when all pass, 3 when any is fail, trapped or unfinished.
- What an example may do: Call functions and build values; it performs no effects and needs no grants. It may call an update function on a var state and look at the state afterwards, dropping returned effects with _ =; comparing effect lists waits for M2-09's equality ruling. Examples take no arguments, cannot be called, share nothing and run in no promised order.
- What an example is for: An example names no target. What it covers is whatever it uses, found by the who-uses-what index later (line 279). Agent-written and human-owned labels arrive in M7.
- Endless loops: Not a trap and not a status in M2. A run that never ends is stopped by whoever started it: the M3 sandbox, later M4-39. Known gap, accepted for now.
- Stack and memory traps across modes: Kind is reported and status is 3 as for any trap, and stack also covers a value nested too deep to copy or compare (M2-05). Whether and where they happen is not compared between back ends: only an unbounded-recursion case is shared, compared on kind and status. The exact 10,001 and size-cap tests are the interpreter's own unit tests (M2-26), outside the conformance suite.
- Changed in the final record: The full record is docs/decisions/M2-06.md. All five picks stand. It differs from the text above in these places: a failed comparison prints `left:` and `right:` on two lines; the call-chain line is the same in every back end, though still outside meaning; `stack` and `memory` traps print the first line only; an untyped hole is `cannot-infer` and an example not ending in a Bool is `type-mismatch`, so the only new codes are the `hole` note and `hole-in-build`; example names share the one name pool; the M1 case parse/example stays a reject.

### M2-06 · what this changes in TASKS.md

- M2-26: Adds the size cap beside the call-depth limit, tests both limits in its own unit tests, and must run on a thread whose stack holds 10,000 nested Paka calls.
- M2-27: Statuses stay the four listed; it also prints both sides of a failed comparison and exits 3 unless all pass.
- M2-20: A hole with no expected type is an error that says to bind it to a let with a written type; a hole alone as a statement is accepted.
- M2-12: Must decide whether a stop-in-value-position form exists; if so it reuses the assert kind.
- M2-14: Names the trap kind of every built-in from this list, or adds a kind; it may not rename one.
- M2-15: Takes the report shape and status 3 as given and rules only on whether refused and failed effects are trap kinds, and on rollback.
- M2-41: No case may depend on the depth number or on a memory trap; one unbounded-recursion case expects kind stack and status 3.
- M4-27: Stack trap position is not compared with the interpreter; only that it is a clean stack trap.
- M4-40: Compares kind and position for all traps except stack and memory, where only kind and status are compared.
- M6-06: Same exception as M4-40 for stack and memory traps.
- M6-16: Its stack traps are checked for kind and clean exit only, never for position.
- M6-19: A hoisted or merged check must still report the first trap in evaluation order, for example by falling back to an exact path.
- M3-04: Tasks must stay under 10,000 nested calls and 16,777,216 items in one collection.

## M2-09 · Decide generics, interfaces and function values together

### M2-09.1 How do ==, <, +, max and len work on several types when there is no overloading?

**One closed built-in table** (recommended). Each operator and built-in name has one row that lists the types it takes; only the language adds rows, and a user function has one signature with written types. Costs: a user cannot make max or len work on their own type, and cannot define a function named like a built-in.

```
// len takes a String, a List or a Map; max takes two Ints, Floats or Strings
// max(1, 2.0) is rejected: both sides must be one type; fix: 1.0
// a user fn named len is rejected: len is a built-in name
fn widest(a: String, b: String) -> Int {
  max(len(a), len(b))
}
```

- Why: It is what M2-03 and M2-05 already assume and what M2-01.2 ruled: built-in generics only. A use is checked from one table row and the argument types, so line 136's promise holds with nothing new to build.
- Locks: The table form: one row per operator or built-in, and a user may not add or reuse a row. Rows already fixed upstream (M2-03, M2-05): comparisons on Int, Float and String; + - * / on Int and Float; % on Int only; len on String and List. The points below add ==, order and + on text; M2-14 adds its own rows. A later interface design must fit around these rows.
- Not chosen: One name per type (Every built-in works on one type: max_int, max_float, text_len, list_len. No table of type sets, but more names to learn and a wrong pick possible at every call.) · Named built-in interfaces (The reference shows the type sets as names (Equal, Ordered) inside built-in signatures, though users cannot write them. Ready for user generics later, but spends reference tokens on words no program can use.)
- Look again when: After 'go', when user generics and interfaces are designed; the table rows then become the first interfaces.
- Owner must still: Section 8's question on generics and interfaces (line 354) stays open, as M2-01.2 ruled, and TASKS.md has no task that decides it after 'go'. Add a decision task before M4-38, or record that Paka ships without user generics.

### M2-09.2 Do records, enums and collections have == without the agent writing it?

**Every type, by structure** (recommended). == and != work on any two values of one type: records field by field, enums by variant then fields, Lists item by item, Maps by the same keys with equal values. Costs: a deep comparison hides behind one operator, and a record that holds a NaN is not equal to itself, as IEEE says.

```
type Point { x: Int = 0, y: Int = 0 }

enum Shape { Dot(at: Point), Line(p: Point, q: Point) }

example same_shape {
  let a = Line(Point {}, Point { x: 3, y: 4 })
  let b = Line(Point {}, Point { x: 3, y: 4 })
  a == b && a != Dot(Point {})
}
```

- Why: Values are copies with no identity (M2-05), so structure is the only meaning equality can have. One sentence in the reference, and every example can compare whole values.
- Locks: Results of == on every user type; comparing is deep and can trap stack (M2-05). Field and variant order do not matter to ==, so reordering them stays safe. What a function value inside a type does is the fourth point's.
- Not chosen: Only when marked (A type has == only if its definition says so. The cost of a deep comparison is visible, but it is one more mark to forget and one more error kind.) · Primitives only (== works on Int, Float, Bool and String; for anything else the agent writes a function. Nothing to build, but every example on a record needs hand-written comparison code.)
- Look again when: M6 shows hidden deep comparisons costing real time; a warning can be added without breaking a program.

### M2-09.3 What can be a function value?

**Named functions and variants** (recommended). A function value is the name of a user fn or of a user variant that has fields; its type is written fn(Int, Int) -> Bool. No function written in place (a lambda) and no capture; costs a named helper or a loop where a model would write a lambda.

```
// ordered("fig", "kiwi", by_length) is true
// with wc's Msg: let wrap: fn(String) -> Msg = Loaded
fn by_length(a: String, b: String) -> Bool {
  len(a) < len(b)
}

fn ordered(a: String, b: String, less: fn(String, String) -> Bool) -> Bool {
  !less(b, a)
}
```

- Why: It is exactly what the paper's sample (line 248) and M5-01's event arguments need and nothing more. Anonymous functions and capture can be added later without breaking a program.
- Locks: The function type spelling fn(A, B) -> R and that a function value is one name with no data. M4-37b and M6-15 stay level 3, and M5-03's on_change keeps taking a variant name.
- Not chosen: Anonymous, no capture (Also a function written in place that may use only its own parameters. Saves naming a helper, but adds a form, and the first one that needs an outer variable is an error anyway.) · Closures that copy (A function written in place may use outer variables; it gets a copy of each. Closest to model habit, but a function value then carries data: new layout, new equality rule, level 4 work in two back ends.)
- Against: Models write filter and map with lambdas by habit in Rust and TypeScript; each one here costs an edit and a rewrite as a loop.
- Look again when: Practice runs show lambdas written by habit costing many counted edits, or M2-14 finds its collection built-ins unusable without capture.

### M2-09.4 Can two function values be compared with ==?

**Equal when same name** (recommended). Two function values are equal when they name the same function or the same variant, so == really works on every type, effect lists included. Costs: native code must compare which definition it is, not a code address.

```
example wc_asks_for_the_file {
  var s = Tally {}
  update(&s, Start("a.txt")) == [ReadFile("a.txt", then: Loaded)]
}
```

- Why: With no capture a function value is only a name, so the answer is exact and the same in every back end (line 161). It keeps the rule 'every type has ==' whole and lets examples check the effects an update returns (lines 166, 183).
- Locks: Function identity is part of meaning: it is the definition's name, so editing a body or swapping code in dev mode (line 304) does not change it, and release code may not merge two functions into one identity. If closures arrive later they must compare by function and captured values.
- Not chosen: Not comparable (== on a function type, or on any type that holds one, is a check error. Free to loosen later, but no example can state which effects an update returned, and 'has ==' becomes a property the checker tracks through every type.) · Compare, false if unsure (== is allowed and gives false whenever a function value is reached. Never an error, but a list is then not equal to its own copy.)
- Against: Rejecting the comparison is the option that is free to loosen later; this pick is harder to take back.
- Look again when: Closures are designed, or M6 finds that keeping function identities apart blocks an optimization that matters.

### M2-09.5 Which types have an order (< <= > >=, min, max)?

**Int, Float, String only** (recommended). Only these three compare with < and feed min and max. Records, enums, Lists and Maps have no order: the agent compares a field, or passes a named function to a sorting built-in, which costs a small helper per sort of records.

```
// a > b is rejected: a Player has no order; fix: compare a field
type Player { name: String, score: Int }

fn ahead(a: Player, b: Player) -> Bool {
  a.score > b.score
}
```

- Why: An order taken from field or variant position is the kind of silent dependence M1-06.3 refused for the first variant. The rejected line is an exact error with a fix, and more types can be added later for free.
- Locks: min and max take two values of one ordered type. Sorting anything else needs a named function, which M2-14 must supply a built-in for.
- Not chosen: Every type, by structure (Records order by their fields in written order, enums by variant order, Lists item by item. No helper needed, but moving a field or a variant silently changes results.) · Lists too (As the first option, plus Lists of ordered items compare item by item. Handy for a few tasks; one more row and one more deep operation.)
- Look again when: Practice runs show many rejected comparisons on Lists or records.

### M2-09.6 Does + join two Strings?

**Yes, + and +=** (recommended). + joins two Strings and += appends to a var String; both sides must be Strings. Costs a second way to build text beside interpolation, and two text fields joined where a sum was meant pass the checker.

```
// "n = " + n is rejected when n is an Int; fix: "n = {n}"
fn banner(words: List<String>) -> String {
  var out = ""
  for w in words {
    out += w + " "
  }
  out
}
```

- Why: Rust, TypeScript, Swift and Kotlin all join text with +, so a model writes it without thinking, and a rejected + would cost the first-try pass rate (line 372) in every run. Types still cannot be mixed: Int + String is an error with a fix.
- Locks: Programs will use it, so it cannot be removed. out += x copies in meaning; M4-33 may append in place.
- Not chosen: No, interpolation only (+ is for numbers; text is joined by interpolation or a built-in. One spelling, and free to loosen later, but a + written by habit is an error and loops that build text get longer.)
- Against: Rejecting + on text keeps one spelling (line 51) and could be loosened later for free; this pick cannot be tightened.
- Look again when: Practice runs show text joined where a sum was meant (a number read as text and never parsed) passing the checker and failing tests.

### M2-09.7 Can a record, enum or collection be turned into text automatically?

**No automatic text** (recommended). Interpolation takes Int, Bool and String only; for any other type the agent writes the text from its parts. Costs a small function per printed type and an error where a model writes "{p}" by habit.

```
// "{p}" is rejected: write the fields
fn show(p: Point) -> String {
  "({p.x}, {p.y})"
}
```

- Why: No task's expected output matches a debug form, so a habitual "{p}" is better met by an error with a fix than by output that is silently wrong. Adding a built-in later breaks nothing.
- Locks: Nothing in what an M2 program can compute. The tools still need one way to print a value (failed examples in M2-27, the tree text of M5-06); that form is fixed under defaults and becomes meaning in M5, where ui_tree hands it to a program.
- Not chosen: One fixed debug form (A built-in gives every value one fixed text, such as Point { x: 3, y: 4 }. Handy for debugging, but task output never wants that form, and its exact layout becomes program meaning for good.)
- Look again when: Agents in M4 ask for print-debugging of whole values often enough to justify a built-in that returns the tools' value text form.

### M2-09 · settled without asking

- User-written generics and interfaces: Not decided, as M2-01.2 ruled. No type parameter, no interface and no seventh definition kind exist in M2; a helper is written once per type. Adding them later breaks no program. Line 354 stays an open question; see the first point's owner action.
- Checked from signatures alone: There is no user generic definition to check. Each built-in generic type and function has one row in the built-in table; a use is checked from that row and the types of its arguments, and no other body is read. With var xs: List<Int> = [], the line append(&xs, 1) is accepted and append(&xs, "a") is rejected from the append row and the two argument types alone.
- Where a type cannot be worked out: An empty list [] and any other built-in value with no arguments to fix its type takes the type its position expects, by the same rule as a hole (M2-06). let ys = [] is a check error whose fix is to write the type; let ys: List<String> = [] is accepted.
- Where type arguments are written: Only in type positions after a built-in type name: List<Int>, Map<String, Int>, Effect<Msg>, and M2-12's Option<Int> and Result<Int, String>. Never in an expression, so < and > in an expression are always comparisons. List<List<Int>> closes two brackets; there is no >> token (M2-03).
- How generic code is compiled in dev mode: Built-in generic functions are shared runtime code, one copy for all item types, driven by the item type's layout record from M4-22 (size, copy, release, equality). Editing a program never recompiles them. Release may make one copy per type (M6-14). How user generics would be compiled is not decided.
- What a user's interface hash covers: What line 285 lists and nothing more: the signature as written (parameter types, function types among them, inout marks, labels, result), and for a type its fields, defaults and variants with the layout of everything held by value. No derived property is added: every type has ==, and order exists only on three built-in types.
- Equality details: Both sides must have the same type; Int == Float is a check error whose fix names to_float (M2-03). Float compares by IEEE, also inside records and lists. Two Maps are equal when they hold the same keys with equal values, whatever their order. Ui stays outside comparison, as M5-06 ruled.
- max and min: Two arguments of one type out of Int, Float, String; the result has that type. For Floats a NaN argument gives NaN (M2-03); max(-0.0, 0.0) is 0.0 and min(-0.0, 0.0) is -0.0 in either argument order, so every back end agrees. Strings order by code point.
- += and the other compound forms: Each means the long form: += on Int, Float and String; -= *= /= on Int and Float; %= on Int. No + on Lists: append is the way.
- len and index: len takes a String, a List or a Map and gives an Int. [ ] reads a String or a List; whether a Map has an index form is M2-14's, under M2-12's line between trap and returned failure.
- Other rows fixed upstream: abs and unary minus take an Int or a Float; Bool has == and != but no order (M2-03). Nothing here changes them.
- What for can walk: A List and an integer range, as M2-02 and M2-03 fixed. Whether a String or a Map can be walked is a table row for M2-14 to add; no user type can be walked.
- Function types: Written fn(A, B) -> R, or fn(A) for no result, with one space after each comma and around the arrow (M4-47 asked for this sample). Labels are not part of the type: a call through a function value passes every argument by position. A function with an inout parameter cannot be a value; lifting this later breaks nothing.
- Using and calling a function value: A lower-case function name, or an Upper variant name with fields, written without ( ) is the value; a variant without fields is just its value, so then: Saved passes a Msg value as in M2-01.3's sample. It may be a parameter, a local, a field, a variant field or a list item, and is called with ( ) like any function: less(a, b), s.on_done(x). Which effect field takes a function and which a value is M2-15's.
- Built-ins as function values: A built-in function, or a built-in variant that has fields (Some, Ok, Err, ReadFile), cannot be a value, because its row may take several types. The fix is a one-line user function that calls it. Built-in variants without fields (None, Exit) are plain values. Can be loosened later.
- Hiding a built-in name: A local may hide a built-in function name, as M2-02 allows for user functions: var max = xs[0] and let count = len(xs) are accepted, and adding a built-in later breaks no body. A user definition (fn, type or variant) may not reuse a built-in name.
- Value text form for the tools: One spelling, used by paka examples for the two sides of a failed comparison (M2-06, M2-27) and by M5-06's tree text: a value prints as it is written in source. A variant with fields prints them in round brackets by position (Toggle(7)); a variant without fields and a function value print as the name (Add, Loaded); a record prints its type name and every field in declared order (Point { x: 3, y: 4 }); a List prints in square brackets; a String is quoted with M2-03's escapes; Int and Bool print as M2-03 says. How a Float and a Map print inside it is M2-14's, since M2-03 gives a Float no text but fixed. No M2 program can reach this form.
- Bool, records and enums as Map keys: M2-14's call. M2-01 lets tasks rely on Int and String keys only; structural == makes record keys possible later.
- Changed in the final record: The full record is docs/decisions/M2-09.md. All seven picks stand; there are no user generics, interfaces or anonymous functions in M2. It is stricter than the text above in two places: the interface hash covers every named type a signature reaches, not only types held by value, and no app state, headless included, may hold a function type. Function values are equal when they are the same definition. One new code, `not-a-value`.

### M2-09 · what this changes in TASKS.md

- M2-08: Gains rows: + and += on String; == and != on every type with the Map, NaN and function-value cases; min and max on Int, Float and String with the signed-zero and NaN results.
- M2-13: A local may hide a built-in function name; a user definition that reuses a built-in name is rejected with a fix.
- M2-14: Must supply a sorting built-in that takes a named function and gives one defined result for any such function, the same in every back end; decide Map key types and whether for walks a String or a Map; fix how a Float and a Map print in the value text form. Map equality ignores order, so if Map order is insertion order two equal Maps can walk differently: say so or pick key order. It may not add automatic to-text; len, min, max and + follow the rows fixed here.
- M2-15: Effect values are comparable, so examples may compare an update's effect list; M2-06's 'drop effects with _ =' is no longer the only way. It says which effect fields take a function value and which a message value.
- M2-16: Adds function types to signatures and rejects a user definition that reuses a built-in name; no equality or order property per type is stored.
- M2-17: Shrinks: no generic declarations and no type arguments in expressions, so the bracket-versus-comparison trap is gone. It still parses nested type brackets such as List<List<Int>>, function types and M2-12's forms. Its level is the owner's to re-grade; level 3 looks right.
- M2-18: The reference carries the table rows and the one-line rules of this page: == on every type, order on three types, + on text, named function values, no automatic text.
- M2-22: Shrinks to checking uses of built-in generic types against the closed table and the typed-from-position rule. Its done-when clause 'a generic body's result never depends on any caller' cannot be met, as there is no generic body: reword it to 'a use is checked from the table row and the argument types, reading no other body'. Likely level 3.
- M2-27: Prints the two sides of a failed comparison in the value text form fixed here; that text is the tool's help, not something a program can reach.
- M2-29: Shrinks: no interface calls and no interface lookup; only built-in generic calls and structural == reach the core form.
- M2-30: Its done-when clause 'every rejected capture' is empty: all definitions are top level and there is no function written in place, so capture cannot be written. Its reject cases are a built-in used as a value and a function with an inout parameter used as a value; it adds == on function values.
- M2-39: Its generics cases are uses of built-in generic types and the accept and reject lines on this page; function-value cases include == on function values and on effect lists.
- M3-16: When it rechecks M3-04's weak-spot tags: a sort takes a named function and there are no closures, records have no order, + joins text. The tags on T3 (custom sort order) and R2 (priority queue) stand until M2-14 rules.
- M4-03: Function types appear in the type text of takes and gives queries in the canonical spelling. Its look-again for agent-written generic functions does not fire.
- M4-11: The interface hash needs no generic or derived-property input; it covers what line 285 lists plus labels and inout marks.
- M4-22: Each type's layout record also gives its copy, release and equality routines for the shared collection code.
- M4-37b: Stays level 3 (no capture). A function value must carry an identity that == compares, the same for a function and for a variant used as a function, and unchanged by a code swap.
- M4-38: Shrinks: no user generics; it only passes the item type's layout record to the shared runtime code of built-in collections. Its done-when clause 'editing a generic body recompiles only what the decision says' has no generic body to edit: reword it to 'editing any user definition recompiles no collection code, shown by the compile counter'.
- M5-16: Message values in trees print in the value text form fixed here, plus M2-14's spelling for a Float and a Map.
- M5-28: M5-07.2's look-again fires: a function value can now sit in state. Decide when M5 starts; the cheapest rule is that a visual app's stored types may not hold a function type. Headless apps are rebuilt and rerun (line 74) and are not affected.
- M6-14: Shrinks to making per-type copies of built-in collection code; no user generic instances.
- M6-15: Stays level 3; release code must keep function identities apart so == gives the interpreter's answer.
- M6-21: The planted 'wrong generic instance' bug can only be in a built-in collection instance.

## M2-10 · Decide shared immutable data and pools with handles

### M2-10.1 Is there a shared immutable type in M2?

**No shared type yet** (recommended). Nothing in the language says 'shared': a value used in two places is two copies in meaning, and the compiler may skip a copy when nobody can tell. Costs: a big value held in many places is really copied until M6 measures and decides.

```
// a tree: e is used twice, so the result holds two copies of it
enum Expr { Num(value: Int), Add(left: Expr, right: Expr) }

fn double(e: Expr) -> Expr {
  Add(e, e)
}

example double_num {
  double(Num(2)) == Add(Num(2), Num(2))
}
```

- Why: A Shared<T> would mean exactly what T means, so it adds vocabulary and three build tasks (M2-35, M4-35, M6-12) with no change to any program's result. No M2-01 task class has a subtree with two parents, and adding the type later breaks nothing.
- Locks: Nothing in meaning. The plan of line 269 (immutable, no cycles, counted) stays the plan for when a shared type is added; it is recorded as not decided. The M6 figure for reference counting (line 360) cannot be measured until then.
- Not chosen: A Shared type now (A built-in Shared<T> holds a counted, read-only value; copying it copies a pointer. Makes the cost visible, but adds a type, a wrap and an unwrap to learn, and work in M2-35, M4-35 and M6-12 before any measurement asks for it.) · Promise cheap copies (No new type, but the language promises that copying a let value never copies its storage. This is the 'shared until written' option M2-05 turned down: it costs a count on every value and a check on every write.)
- Look again when: M6 measurements show copies of large unchanged values costing more than the pointer-heavy gap of line 99, or a real program holds one big value in many places.
- Owner must still: Approving this page records the shared type as not decided. Approve the reworded M6 exit check listed under effects, and after 'go' either add a decision task for shared data and pools (TASKS.md has none) or confirm that M6 reports copy cost instead.

### M2-10.2 What is recorded about pools and handles, and how is a graph written until then?

**Not decided; list indexes** (recommended). Graph tasks in, pools not built, pool part dropped: no pool type, no handle type, no stale-handle rule and no pool iteration order exist, and a graph keeps its nodes in a List and refers to them by Int position. Costs: a position outside the list traps out_of_bounds, but a wrong position inside it reads the wrong node without a trap.

```
type Graph { next: List<List<Int>> = [] }

fn add_node(g: inout Graph) -> Int {
  append(&g.next, [])
  len(g.next) - 1
}

fn link(g: inout Graph, a: Int, b: Int) {
  append(&g.next[a], b)
}
```

- Why: It is M2-01.1 carried through: rules nobody runs before the go/no-go are guesses. The stale-handle rule is the one new idea in pools and should be designed against real graph programs.
- Locks: The go/no-go says nothing about pools. The trap kind stale_handle stays reserved and unused (M2-06). The M6 figure for handle checks (line 360) cannot be measured until pools exist.
- Not chosen: Rules now, build later (Write the pool, handle and stale-handle rules on paper now so later milestones have a target, but build nothing. Costs a design made with no evidence from the experiment, which its first user will likely reopen.)
- Look again when: After 'go': graph programs that remove nodes show wrong-node bugs, or M6 pointer-heavy benchmarks need pools to be measured (line 360).
- Owner must still: The note on M2-35 asks the owner to record pools as not decided; approving this page is that record.

### M2-10 · settled without asking

- The types and their operations: None. M2 has no shared type, no pool type and no handle type, so M2-14 takes nothing from M2-10.
- The stale-handle rule: Not decided. Nothing in M2 can raise stale_handle; the kind name stays reserved as M2-06 ruled.
- Iteration order: No pool exists, so there is no pool order. A graph's order is its List's order; Map order is M2-14's.
- Tree sample: A tree is a type that contains itself (M2-05): one owned value, copied whole in meaning, compared by structure (M2-09). A subtree used twice is two copies. An enum tree such as Expr is changed by building the changed tree, because names bound by a pattern are values; a record tree such as type Node { kids: List<Node> = [] } can be changed in place through fields and indexes (M2-05).
- Graph sample: Nodes sit in a List and point at each other by Int position, as in M2-01.1. link(&g, a, b) traps out_of_bounds at the [ when a is no node. Removing a node is the program's own business: it marks the node dead or rebuilds the list; the language gives no help and no trap.
- Cycles: Values cannot form cycles: there are no references, only owned values and Int positions. A cycle in a graph is a cycle of positions.
- Cost of copies: Left to M4-33 (skip a copy at a last use) and to M6 measurements. The language makes no promise about copy cost beyond M2-05's: a read-only argument is passed without copying.
- The names Shared, Pool and Handle: Not reserved. A user may name a type Pool or Handle, which are ordinary names in tasks; a reservation would cost counted edits before the go/no-go for a feature that does not exist. If a built-in takes one of the names later, the few programs that use it are renamed mechanically; names can be reserved at any later point.
- Changed in the final record: Nothing: both picks stand. The record adds the tree and graph samples spelled out, the refused spellings, the accepted cost and what evidence reopens the question.

### M2-10 · what this changes in TASKS.md

- M2-35: Dropped: there is no shared type and no pool to implement. It is not flagged conditional, so say it outright: tasks that need it treat it as done, and its tree and graph samples move to M2-40 as golden tests.
- M2-40: Loses M2-35 from needs and its shared-data and pool cases. It runs this page's tree and graph samples and adds one case showing that a value used twice in a tree is two independent copies.
- M2-33: Its done-when names M2-35 as a boundary; no built-in falls there any more.
- M2-14: Takes no types or operations from M2-10.
- M2-18: The reference spends no tokens on shared data or pools; it says in one line that graphs use list positions.
- M4-35: Dropped while no shared type and no pool exist. It is not flagged conditional, so M4-42 drops it from needs.
- M4-42: Drops M4-35 from needs.
- M5-28: Reword its test list: no 'shared value or pool' case. M5-07 said M2-10 fixes what those words mean; neither exists.
- M6 exit check: The figure for 'reference counting and handle checks' cannot be measured while M6-12 and M6-13 are dropped. Reword it to 'the cost of copying owned trees and of bounds checks on list-index graphs', or bring in a decision on shared data and pools after 'go'.
- M6-02: Its cost-measurement method must say how copy cost of owned trees and bounds checks on list-index graphs stand in for the reference-counting and handle-check figures of line 360.
- M6-06: The tags 'shared data' and 'pools' have no cases.
- M6-10: The layout table gains no shared-box and no pool-slot header.
- M6-12: Dropped while no shared type exists; it returns if M6 measurements bring one in. Not flagged conditional, so M6-15, M6-29 and M6-30 drop it from needs.
- M6-13: Dropped while pools are not decided. Not flagged conditional, so M6-21, M6-29 and M6-30 drop it from needs.
- M6-15: Drops M6-12 from needs.
- M6-21: Drops M6-13 from needs. The planted wrong-reference-count and wrong-stale-handle bugs cannot exist; the owner replaces them or lowers the count of ten.
- M6-29: Drops M6-12 and M6-13 from needs; the pointer-heavy benchmarks use owned trees and list-index graphs.
- M6-30: Drops M6-12 and M6-13 from needs; the families 'reference-count updates' and 'handle checks' have nothing to leave out, so it measures bounds checks and copies instead.
- M6-32: The published table shows reference counting and handle checks as not present, with the copy-cost figure beside them.

## M2-12 · Decide the one error-handling mechanism

### M2-12.1 How does a function say it can fail or have no answer?

**A value in the result** (recommended). Failure and absence are ordinary values of built-in enum types, named in the signature and taken apart with match (the sample uses the next point's names). No hidden jumps; costs a wrapped result type on every function that can fail.

```
fn ratio(a: Int, b: Int) -> Result<Int, String> {
  if b == 0 {
    return Err("divide by zero")
  }
  Ok(a / b)
}

example ratio_reports_zero {
  ratio(1, 0) == Err("divide by zero")
}
```

- Why: It needs no new control flow: enums, match and the use-or-discard rule already exist, and the signature carries the whole fact (line 136). It is Rust's model, which the model under test knows well.
- Locks: Every signature that can fail, every built-in that can fail (M2-14) and how effect failures reach a program (M2-15). Moving to exceptions later rewrites all of them.
- Not chosen: Marked throwing functions (A signature says throws, the body uses throw, callers use try and catch, as in Swift. Familiar, but it adds four keywords and a second way for control to leave a function.) · Unmarked exceptions (Any function may throw and any caller may catch, as in TypeScript. Least to write, but a signature no longer tells whether a call can fail, which breaks checking from signatures alone.)
- Look again when: Practice runs show failure handling costing clearly more tokens or edits than Rust's and TypeScript's on the same tasks (lines 373-374).

### M2-12.2 Which built-in types carry failure and absence?

**Option and Result** (recommended). Option<T> is Some(value) or None; Result<T, E> is Ok(value) or Err(error), where E is any type the function picks. Costs four variant names taken from users and two types to learn.

```
fn describe(found: Option<Int>) -> String {
  match found {
    Some(i) => "found at {i}"
    None => "missing"
  }
}

example nothing_found {
  describe(None) == "missing"
}
```

- Why: They are the two shapes a model already writes in Rust, names included, so nothing new is learnt. A free error type lets a task use text where a message is enough and an enum where it must branch.
- Locks: The names Option, Some, None, Result, Ok, Err and their meaning; a user type or variant with one of these names is an error (M2-02.2). Flat patterns (M2-02) mean a Result inside an Option needs two matches.
- Not chosen: Result with text error (As above, but Result<T> has one parameter and the error is always a String. Shorter signatures and any two failures combine, but a program cannot branch on the kind of error except by comparing text.) · Option only (Only Option<T> is built in; a failure with a reason is a user enum written once per result type. Smallest built-in list, but more code per task and no shared shape for a failure.)
- Look again when: Practice runs show mismatched error types between functions costing many edits; a conversion built-in can then be added.

### M2-12.3 Where is the line between a trap and a returned failure?

**Bugs trap, outside data returns** (recommended). If the program can tell beforehand by comparing numbers it already holds (a position against a length, a divisor against zero), failing is a bug and traps. If telling needs the same work as doing it (is this text a number, is this key present, does this file exist), the answer comes back as Option or Result, which costs a match at every parse and lookup.

```
// xs[2] on a short list traps out_of_bounds: len(xs) could have told
// parse_int stands for M2-14's name; only parsing can tell
fn count_of(text: String) -> Result<Int, String> {
  match parse_int(text) {
    Some(n) => Ok(n)
    None => Err("not a number: {text}")
  }
}
```

- Why: It keeps the traps already decided and makes the checker, not a test run, tell the agent that input can be bad (line 33). M2-01 lets a task demand an error message and exit status on malformed input, which a trap cannot give.
- Locks: The result type of every built-in that reads outside data or looks something up (M2-14, M2-15). Turning a returned failure into a trap later, or the reverse, changes signatures.
- Not chosen: Everything returns (Indexing, division and conversion also return Option. No program stops by surprise, but every xs[i] needs handling, and it reverses M2-03 and M2-05.) · Traps plus test functions (No built-in returns Option; the program asks first (is_number, has) and the operation traps if it did not. No wrapped results from built-ins, but every check is done twice and a forgotten one ends the program on bad input.)
- Look again when: Practice runs show a parse or lookup result costing many edits where the task guarantees the data, or a trap ending programs on input the task calls malformed.

### M2-12.4 Is there a short form to pass a failure up to the caller?

**Postfix ?** (recommended). expr? gives the inside of Some or Ok; on None or Err the function returns that None or Err at once. Allowed only where the function's own result is an Option, or a Result with the same error type; costs one operator, a less visible exit, and a token that sits next to the hole mark ??.

```
// ratio(a, b) gives Result<Int, String>, as in the first point
fn mean(total: Int, count: Int) -> Result<String, String> {
  let m = ratio(total, count)?
  Ok("mean {m}")
}

example mean_passes_the_failure_up {
  mean(10, 0) == Err("divide by zero")
}
```

- Why: It is checked from the function's own signature, it is a known small rewrite into match and return, and it is the spelling that goes with Option and Result in the model's training. Without it Paka pays tokens on every chained step.
- Locks: The ? token and the early exit it hides. ? never converts: an Option in a Result function, or two different error types, is a check error with the expected and found types. x?.name leaves the function as in Rust; it is not TypeScript's optional chaining.
- Not chosen: Prefix try (The same meaning written try expr, as in Swift and Zig. No clash with ??, but a new keyword, and it differs from the Rust spelling the model knows for these very types.) · Match only (No short form: every failure is taken apart with match. Nothing to build and free to extend later, but the longest programs of the three languages on failure-heavy tasks.)
- Against: Match only is the option that can be extended later for free, and it keeps every exit of a function visible.
- Look again when: Practice runs show ? written in places it cannot work (update functions, examples) costing many edits.

### M2-12.5 Is there a form that stops the program where a value is expected?

**unreachable()** (recommended). A built-in with no arguments that takes the type its position expects, like a hole, and traps with kind assert when reached. Unlike a hole it builds in release; costs one built-in name and gives no message beyond the position.

```
enum Mode { Idle, Busy(job: Int) }

fn job_of(m: Mode) -> Int {
  match m {
    Busy(job) => job
    // reaching this traps with kind assert, reported at the name
    Idle => unreachable()
  }
}
```

- Why: It reuses the hole's typing rule and the assert kind, so it adds one name and no new rule. The name says what it is for, which keeps it apart from reporting bad input.
- Locks: The name unreachable and that reaching it is trap kind assert, reported at the name. Adding a message argument later breaks nothing.
- Not chosen: panic with a message (The same, with a String that is printed as a help line. Matches Rust habit, but invites using a trap to report bad input, which gives status 3 and the toolchain's wording instead of the task's.) · No such form (The arm must give some value, after an assert(false). Nothing new, but a made-up value hides the bug it stands for.)
- Look again when: Agents write panic or unwrap by habit often enough in practice runs to cost counted edits; either can be added as one more row.

### M2-12 · settled without asking

- Typing of None, Ok and Err: Some(x) fixes its own type. None, Ok(x) and Err(e) take the missing type from their position by the hole rule of M2-06: a result, a parameter, a declared let, the other side of ==. The expected type passes into the branches of an if, the arms of a match and the last expression of a block, so Some(v) => Ok(v) beside None => Err("missing") is typed by the function's result. let r = Ok(3) is a check error whose fix is to write the type.
- Blocks that leave: A block that ends in return, break or continue, and unreachable() itself, give no value and fit any expected type. So an arm may be None => { return Err("missing") } with the block broken over three lines. A return in an arm needs a block (M2-02's arm forms; M4-47 puts return on its own line); loosen to a bare => return only if practice runs show it costing edits.
- Handling is forced by rules already made: A Result or Option cannot be used as its inside value: only match, ? or == reach it. A call whose Result is left unused is the M2-02.6 error; its fix offers match, ? or _ =. Writing _ = on purpose drops a failure and is allowed. One gap is accepted: a Result bound to a let and never read gets only the unused-name warning.
- Where ? may stand: Inside a fn whose result is an Option (for an Option) or a Result with the same error type (for a Result). Anywhere else, an update that returns effects or an example included, it is a check error whose fix is a match.
- Telling ? from a hole: ?? followed directly by a lower-case letter is a hole; any other ? is the postfix operator. It binds as tightly as a call or an index. A ? at the end of a line does not continue the line: it is not a trailing two-sided operator under M2-02's line rule.
- unreachable() as a statement: Accepted on its own line with no _ =, like a hole: it has no value there. A hole in the same place would report 'unfinished' and stop paka build; unreachable() builds.
- No extra kinds of failure: No exceptions, no catch, no way to recover from a trap inside a program, no error codes and no null. A trap ends the handler or program as M2-06 and M2-15 say.
- Text to number: The parsing built-ins of M2-03 give Option<Int> and Option<Float>: there is one way to fail and no reason to carry. An Int that does not fit is None. M2-14 names them.
- Lookups, searches and taking by position: A key lookup that may miss gives Option<V>; a search gives Option<Int>. Taking an item by position from an empty List (pop, first or last, if M2-14 has them) traps out_of_bounds like xs[0], because len could have told. If M2-14 also keeps an index form m[key], it is the form that claims the key is there: a missing key traps, and M2-14 names the kind (out_of_bounds fits; M2-06 lets it add a kind but not rename one).
- Effect failures: A failed file read or write must reach update as data the program matches on (a Result in the message or a second message), which agrees with M7-02's 'as a message, never as a trap'. The exact form is M2-15's, as is whether a failed Print stays the error M1-06.2 made it.
- Comparing and printing: Option and Result compare with == like any enum (M2-09), so an example may end in place_of(xs, "a") == Some(0). A program gets no automatic text for them; the tools print Some(3), None and Err("divide by zero") in M2-09's value text form.
- Trap kinds: None added. unreachable() reuses assert, as M2-06 asked.
- Helpers: Whether there are built-ins that give a default for None or turn None into an Err is M2-14's; each is one table row and none changes the mechanism. Without the second one every Option met inside a Result function costs a four-line match, so M2-14 should weigh it.
- Layout of the new forms: As M4-47 asks: Result<Int, String> with one space after the comma; expr? with no space before the ?; unreachable() like any call; a match on an Option or Result has one arm per line. The samples on this page are in that layout.
- Changed in the final record: All five picks stand and no diagnostic code is added. Added: an if or match used as a value with no expected type takes the type of its first branch that carries its own type, and that type flows into the other branches; `?` on an Option inside a function returning Result is refused (write `ok_or(opt, text)?`); Some, None, Ok and Err are signature rows of the built-in table; unreachable-code also fires after unreachable(); inout writes made before a `?` stay made. The full record is docs/decisions/M2-12.md.

### M2-12 · what this changes in TASKS.md

- M2-07: The token list gains ?; ?? stays one token only when a lower-case letter follows directly.
- M2-11: A postfix ? at the end of a line ends the statement; it does not continue the line.
- M2-17: Parses postfix ? at call-and-index tightness and the two-parameter type Result<T, E>; no throw, try or catch forms.
- M2-20: None, Ok and Err with no expected type are errors that say to write the type. The expected type passes into the branches of an if, the arms of a match and the last expression of a block; holes use the same path, which only accepts more than M2-06 listed. A block that ends in return, break or continue fits any type. unreachable() is typed like a hole and accepted as a statement; a match on Option or Result must name both variants.
- M2-23: The core form needs no new node beyond early return: ? lowers to a match that returns, and a block that ends in return has no value.
- M2-31: Its forms are the two built-in enums, ? as a rewrite into match and return, and unreachable(). Its 'unhandled failure is rejected' case is the M2-02.6 unused-result error with a fix that offers match, ? or _ =.
- M2-14: Takes Option, Some, None, Result, Ok, Err and unreachable into the names table. Gives every parse, search and key lookup an Option result and lets take-by-position on an empty List trap, under the line fixed here; names the trap kind of a missing key if it keeps m[key]; decides the helper rows, the None-to-Err one first.
- M2-15: Must deliver file effect failures as data to match on and supply a way to print a message and exit with a status the program chooses (M3-04 needs 20). It must keep the paper's wc sample (line 248) running as the M2 exit check asks, or list the change to it.
- M2-18: The reference shows Option, Result, ? with the places it may stand, the trap-or-return test in one sentence, and unreachable().
- M2-39: Error cases cover ? in each allowed and rejected place, mismatched error types, a dropped Result, None and Err typed from position and rejected without one, and unreachable() reached and not reached.
- M3-16: When it rechecks M3-04's weak-spot tags on the file tools and the tasks with error messages: failures are values, ? passes them up inside functions, and ? cannot stand in update.
- M4-30: unreachable() is compiled like a hole that traps with kind assert, but is allowed in release builds.
- M4-31b: Small: Option and Result are ordinary enums (M4-31); it adds the early exit of ? and the assert trap of unreachable().
- M6-15: Error handling is enum lowering plus an early return; nothing else to lower.
- M7-02: A failed foreign call reaches the program as an Option or Result inside a message, the same form M2-15 gives a failed file read.

## M2-14 · Decide the built-in types and functions a headless program may call

### M2-14.1 In what order is a Map walked, and which types may be keys?

**Key order, Int or String** (recommended). A Map keeps its entries in rising key order: Ints by number, Strings by code point; only Int and String may be keys. Costs: a lookup is a search in a sorted structure, slower than hashing, and a record cannot be a key.

```
example keys_come_in_key_order {
  var m: Map<String, Int> = [:]
  m["pear"] = 2
  m["fig"] = 1
  keys(m) == ["fig", "pear"] && m == ["fig": 1, "pear": 2]
}
```

- Why: The order follows from the content alone, so equal Maps always walk the same and M2-09's warning is answered. It needs only the order Int and String already have (M2-09), no hashing, and it is Rust's standard ordered map in the interpreter.
- Locks: Results of every program that walks a Map. The key types can be widened later for free; the order cannot change once programs print in it.
- Not chosen: Insertion order, any key (Entries stay in the order they were first added, as in TypeScript, and any type may be a key because every type has ==. Costs a hash rule for every type that all back ends must share, and two equal Maps can walk in different orders.) · Insertion order, Int or String (As the second option but with the two key types only. Less to build than any-key, but equal Maps still walk differently, and removing an entry needs a rule for the gap it leaves.)
- Look again when: M6 benchmarks show Map lookups dominating real programs, or a task needs a record as a key.

### M2-14.2 How is a Map written, read and changed?

**Literal, index and get** (recommended). A Map is written [key: value, ...], or [:] when empty, as in Swift; m[key] reads an entry that must be there and traps out_of_bounds on a missing key, m[key] = v adds or replaces, &m[key] passes an existing entry in place, and get(m, key) gives an Option. Costs two ways to read, one that traps and one that does not, and counts[word] += 1 traps on a new key.

```
fn add_edge(g: inout Map<String, List<String>>, a: String, b: String) {
  if !has(g, a) {
    g[a] = []
  }
  append(&g[a], b)
}

fn add_word(counts: inout Map<String, Int>, word: String) {
  counts[word] = unwrap_or(get(counts, word), 0) + 1
}
```

- Why: It follows M2-12's line exactly: the index form claims the key is there and traps, the lookup that may miss returns an Option. In-place change through &m[key] keeps group-by code as short as in Rust and TypeScript.
- Locks: The literal spelling, and that m[key] = v on a new key adds it while xs[i] = v on a missing List item traps. A Map index step counts like a List index step in M2-05's overlap rule: two entries of one Map always overlap.
- Not chosen: Functions only (No index form and no literal: get, put and remove are the only ways in and out. One way to read, but a value held in a Map cannot be changed in place, so every grouped list is copied out and put back.) · Index gives an Option (m[key] always gives Option<V>, so a missing key can never trap. Costs a match or a helper at every read, and [ ] then means two different things on List and Map.)
- Look again when: Practice runs show missing-key traps, above all from += on a new key, costing counted edits; a row that adds to a key with a starting value can then be added without breaking a program.

### M2-14.3 How does code walk a Map or a String when there are no tuples?

**Lists from keys and chars** (recommended). for walks a List or an integer range and nothing else; keys(m), values(m) and chars(text) give ordinary Lists to walk. Costs a lookup per entry, and for c in text written by habit is a check error whose fix names chars(text).

```
fn report(counts: Map<String, Int>) -> List<String> {
  var out: List<String> = []
  for name in keys(counts) {
    let n = counts[name]
    append(&out, "{name} {n}")
  }
  out
}
```

- Why: No new syntax and no new type: three table rows. keys(m) is a fresh List, so the loop body may change the Map and M2-05's frozen-list rule needs no extension. Letting for walk more later breaks nothing.
- Locks: Little. The names keys, values and chars, and that their results are in key order and position order.
- Not chosen: Two names in for (A new for form binds key and value at once, and for also walks a String directly. Shortest to write, but it adds a loop form and a rule for changing a Map or String while it is walked.) · A built-in Entry record (entries(m) gives a List of a built-in record with key and value fields. No new loop form, but one more built-in generic type and one more Upper-case name taken.)
- Look again when: Practice runs show for over a String or Map written by habit costing many counted edits; accepting it is then a free loosening.

### M2-14.4 Do the built-ins that change a List change it in place or give back a new List?

**In place, with &** (recommended). Every built-in that changes a List takes it as inout: append, extend, pop, insert, remove_at, swap, reverse, sort, sort_by. One rule; costs a var copy first when the List is a let or a read-only parameter.

```
fn top_three(scores: List<Int>) -> List<Int> {
  var best = scores
  sort(&best)
  reverse(&best)
  best[0..<min(3, len(best))]
}
```

- Why: One rule covers every change to a List, the copy is visible, and it is what a model writes in Rust (v.sort()). The second form can be added later as plain new rows.
- Locks: Little: the names. Adding sorted and reversed later breaks only a program that defined those names itself.
- Not chosen: New List returned (sorted(xs) and reversed(xs) give new Lists and nothing but append works in place. Reads well inside expressions, but it is a second rule beside append, and the copy is hidden.) · Both forms (sort and sorted, reverse and reversed, as Swift and Kotlin have. Never an error whichever a model writes, but two spellings for each and twice the rows.)
- Look again when: Practice runs show sort or reverse called on a let or a read-only parameter costing many counted edits.

### M2-14.5 How is a List sorted in an order the program chooses, and what is the result for any function it passes?

**before function, fixed merge sort** (recommended). sort_by(&xs, before) takes a named fn(T, T) -> Bool that says whether its first argument must come first; the sort is stable (equal items keep their order) and is one fixed merge sort, so even a function that contradicts itself gives the same result everywhere. Costs: release code may not swap in a faster algorithm.

```
type Player { name: String, score: Int }

fn before(a: Player, b: Player) -> Bool {
  a.score > b.score || a.score == b.score && a.name < b.name
}

fn rank(ps: inout List<Player>) {
  sort_by(&ps, before)
}
```

- Why: A Bool needs no new type and no sign rule, and it handles any number of keys. Fixing the algorithm is the only way a trap inside the function, or a function that contradicts itself, means one thing (M2-06: first trap in evaluation order).
- Locks: The exact sequence of calls to the function, which every back end must copy: M6-14 may make per-type copies but not change the algorithm. sort(&xs) is the same sort with < as the function.
- Not chosen: Three-way function (The function gives an Int: below zero, zero or above, as in TypeScript and C. One call tells everything, but the sign convention is a classic silent mistake and a - b can trap overflow.) · Key function only (sort_by_key(&xs, key) takes a function that gives an Int or String to order by. Shortest for one key, but two keys or a falling order cannot be written without tuples.)
- Look again when: M6 benchmarks show sorting dominating a real program; a faster sort could then be allowed where the order is a built-in one on Int or String.

### M2-14.6 How large is the built-in vocabulary?

**About sixty rows, loops otherwise** (recommended). Text, List, Map, number and Option helpers that each replace a loop a model would otherwise write by hand: 63 functions in all, listed under defaults, with no map, filter or fold, no Set, no priority queue and no text patterns. Costs: common names such as find, get, insert and words cannot be user function names.

```
fn long_words(text: String, n: Int) -> List<String> {
  var out: List<String> = []
  for w in words(text) {
    if len(w) > n {
      append(&out, w)
    }
  }
  out
}
```

- Why: Each row is something at least one of M2-01's seven task classes uses and that Rust and TypeScript give for free. Functions that take functions are held to the one that cannot be done without, the sort.
- Locks: The names: none can be renamed after the experiment, and a built-in added later breaks any program that already defined that name, though the fix is a plain rename. So the list is meant to be enough for the experiment, not a start.
- Not chosen: Wide, with map and filter (Also map, filter, fold, any, all, sum, a Set and a priority queue. Closer to model habit, but without closures (M2-09) a passed function cannot use a local value, so most habitual uses are errors anyway.) · Bare minimum (Only len, append, index, slice and the number rows of M2-03. Fewest names taken, but every program writes its own split, trim, join and find, which costs tokens and bugs before the go/no-go.)
- Look again when: Practice runs show a helper written by hand in most solutions, or a habitual name (map, filter, sum) costing many counted edits.

### M2-14.7 Do the text built-ins follow Unicode rules or ASCII rules for case and white space?

**ASCII rules only** (recommended). upper and lower change the letters a to z and A to Z and leave every other character alone; trim, trim_start, trim_end and words treat only six ASCII characters as white space (listed under defaults). Exact and table-free; costs: accented letters do not change case.

```
example case_is_ascii_only {
  upper("élan") == "éLAN" && lower("ÉLAN") == "Élan" && trim(" a\t\n") == "a"
}
```

- Why: The result is one sentence, never changes with a library version and is the same in every back end with no data to ship. Unicode versions of these functions can be added later under their own names.
- Locks: Results of upper, lower, trim, trim_start, trim_end and words on non-ASCII text. Changing them later changes what existing programs print.
- Not chosen: Full Unicode rules (Case and white space follow the Unicode tables. Right for real text, but the tables change with each Unicode version, some case changes alter the length, and every back end must carry the same tables.)
- Look again when: A real program after 'go' needs case-blind matching of non-ASCII text.

### M2-14 · settled without asking

- Built-in types: Int, Float, Bool, String, List<T>, Map<K, V>, Option<T>, Result<T, E> and Effect<Msg> (M2-15). No Set (use Map<K, Bool>), no tuple, no priority queue, no pool or shared type (M2-10 supplies none).
- Upper-case built-in names: Int, Float, Bool, String, List, Map, Option, Some, None, Result, Ok, Err, Effect; from M2-15 the effects Print, ReadFile, WriteFile, Exit, ExitWith and the grants Console, ReadFiles, WriteFiles. Grant names sit in the one pool of names (M7-02). A user type or variant may not reuse any of these (M2-02.2).
- How the table is written: One row per name in M2-02's signature form, name(parameters) -> result, with T, K, V, E standing for any type; a row with no arrow gives no result. Labels are on three rows only: fixed (digits:, from M2-03), insert (at:) and list_of (count:), the two where an Int item and an Int position or count could be swapped unseen. Every other row is positional, with the value worked on first. All 63 function names are lower case; a user definition may not reuse one, a local may hide one (M2-09). A few rows take several types, as len already does; user functions cannot.
- Rows shared by several types (4): len(x) -> Int and is_empty(x) -> Bool take a String, List<T> or Map<K, V>. contains(text: String, part: String) -> Bool and contains(xs: List<T>, item: T) -> Bool. find(text: String, part: String) -> Option<Int> and find(xs: List<T>, item: T) -> Option<Int> give the first position. An empty part is always found, at 0. Items are compared with ==. None of these traps.
- Number rows (20, names confirmed from M2-03 and M2-09): min(a: T, b: T) -> T and max(a: T, b: T) -> T for T of Int, Float or String. abs(x: Int) -> Int and abs(x: Float) -> Float. to_float(n: Int) -> Float. to_int(x: Float) -> Int. sqrt, floor, ceil, trunc and round are (x: Float) -> Float. fixed(x: Float; digits: Int) -> String. wrap_add, wrap_sub, wrap_mul, bit_and, bit_or and bit_xor are (a: Int, b: Int) -> Int. bit_not(a: Int) -> Int. shift_left(a: Int, count: Int) -> Int and shift_right(a: Int, count: Int) -> Int. Results are exactly those M2-03 and M2-09 fixed. No power, no sum over a List, no random numbers, no dates.
- Text to number (2): parse_int(text: String) -> Option<Int> and parse_float(text: String) -> Option<Float>, with M2-03's meaning. Nothing is trimmed: parse_int(" 7") is None, parse_int("") is None, parse_float("7") is Some(7.0).
- Number to text: An Int goes into text by interpolation ("{n}"), a Float by fixed. There is no other to-text function (M2-09).
- Character numbers (2): code_at(text: String, i: Int) -> Int gives the code point at position i and traps out_of_bounds like text[i]. from_code(n: Int) -> String gives a one-character String and traps bad_conversion when n is outside 0 to 1114111 or inside 55296 to 57343. A digit test is written c >= "0" && c <= "9" on a one-character String.
- White space, exactly: The six characters with code points 9 to 13 and 32: tab, line feed, vertical tab, form feed, carriage return and space. trim, trim_start, trim_end and words use this set; no other row does. On ASCII text this is what Rust's trim and split_whitespace and TypeScript's trim do.
- Splitting text (4): split(text: String, sep: String) -> List<String> cuts at each sep found left to right: split("a,,b", ",") is ["a", "", "b"] and split("", ",") is [""]. It gives at least one piece unless sep is empty; an empty sep gives chars(text), so split("", "") is []. chars(text: String) -> List<String> gives one String per character. lines(text: String) -> List<String>: each line ends at a line feed, which is dropped, and text after the last line feed is one more line only if it is not empty. So lines("a\nb\n") is ["a", "b"], lines("") is [], lines("\n") is [""], lines("a\n\n") is ["a", ""]. A carriage return is an ordinary character and stays; trim_end removes it. words(text: String) -> List<String> gives the runs between white space and never an empty piece; words("") is [].
- Joining and padding text (4): join(parts: List<String>, sep: String) -> String; join([], sep) is "". repeat(text: String, n: Int) -> String gives n copies and traps out_of_bounds when n is below zero. pad_left(text: String, width: Int) -> String and pad_right(text: String, width: Int) -> String add spaces until len is width and leave longer text alone.
- Trimming, testing and replacing text (8): trim, trim_start and trim_end are (text: String) -> String and remove white space from both ends, the start or the end. starts_with(text: String, part: String) -> Bool and ends_with(text: String, part: String) -> Bool are true for an empty part. replace(text: String, old: String, new: String) -> String replaces every occurrence found left to right without overlap; an empty old changes nothing. upper(text: String) -> String and lower(text: String) -> String follow the ASCII rule.
- List rows (10): append(xs: inout List<T>, item: T). extend(xs: inout List<T>, more: List<T>) appends each item in order. pop(xs: inout List<T>) -> T removes and gives the last item. insert(xs: inout List<T>, item: T; at: Int) needs 0 <= at <= len. remove_at(xs: inout List<T>, i: Int) -> T. swap(xs: inout List<T>, i: Int, j: Int), the built-in M2-05 asked for; i == j does nothing. reverse(xs: inout List<T>). sort(xs: inout List<T>) for Int, Float or String items. sort_by(xs: inout List<T>, before: fn(T, T) -> Bool). list_of(item: T; count: Int) -> List<T>. A bad position, pop on an empty List or a count below zero traps out_of_bounds.
- The sort, exactly: A List of 0 or 1 items is left alone. Otherwise the items before position len / 2 are sorted, then the rest, then the two halves are merged from the front: the next item comes from the right half only when before(right, left) is true, else from the left. sort uses < as before, so a NaN stays where this leaves it.
- Map rows (5): get(m: Map<K, V>, key: K) -> Option<V>. has(m: Map<K, V>, key: K) -> Bool. remove(m: inout Map<K, V>, key: K) gives no result and does nothing when the key is missing. keys(m: Map<K, V>) -> List<K> in rising order. values(m: Map<K, V>) -> List<V> in key order. In a literal, entries are added in written order and a later equal key replaces the earlier one. A key type other than Int or String is a check error.
- Map index details: m[key] as a read, m[key] += 1, m[key].field = v and &m[key] all need the key to be there; a missing key traps out_of_bounds at the [ with the key in the help line. Only the last step of a plain assignment may add a key: m[key] = v adds, and m[a][b] = v needs key a. The reference shows the count idiom counts[word] = unwrap_or(get(counts, word), 0) + 1. This answers M2-05: a Map entry is a place.
- Option and Result helpers (2): unwrap_or(opt: Option<T>, fallback: T) -> T gives the inside of Some or the fallback. ok_or(opt: Option<T>, error: E) -> Result<T, E> turns Some(x) into Ok(x) and None into Err(error), so an Option can be passed up with ? in a Result function. Both arguments are evaluated before the call. No unwrap and no panic (M2-12).
- assert and unreachable (2): As M2-06 and M2-12 fixed them; both are rows of the table.
- Functions that take functions: Only sort_by. No map, filter, fold, any or all: without closures they could not use a local value, so loops are the form. M2-09's look-again does not fire.
- Trap kinds: None added. out_of_bounds: a bad position (index, slice, code_at, insert, remove_at, swap), pop on an empty List, a missing Map key at an index, a count below zero (repeat, list_of), a shift count outside 0 to 63, fixed digits outside 0 to 20. bad_conversion: to_int and from_code. overflow: abs of the smallest Int. assert: assert and unreachable. Every other row never traps. A trap is reported at the user's call (M2-06) and the help line says which case it was.
- What for can walk: A List and an integer range only. for over a String or a Map is a check error whose fix names chars(text) or keys(m).
- Value text form for the tools: A Float prints as the shortest digits that read back as the same Float, with no exponent and at least one digit on each side of the point (1.0, 0.1, -0.0); NaN, inf and -inf print as fixed prints them. A Map prints as its literal in key order, [:] when empty. No M2 program can reach this form (M2-09).
- Coverage of the task classes: Text processing: the String rows, sort and sort_by. Parsing and small interpreters: chars, code_at, slices, parse_int, parse_float, Map for names. State machines and simulations: enums and match, Map, list_of. Grids and dynamic programming: list_of, index in place, swap, min and max. Graph algorithms by list index: Lists of Lists, append, pop, remove_at, Map for name to position. Integer and float reports: the number rows, fixed, pad_left, join. File tools: the effects of M2-15 with lines, split and join. Tree data in any class uses recursive types (M2-05), no built-in.
- The paper's samples: The counter's update (line 208) needs max, which is a row. The wc's count_words is written len(words(text)). Its field words stands beside the built-in words, because field names clash with nothing (M2-02).
- Pool and shared-data operations: None: M2-10 decided there is no shared type and no pool, so nothing is taken from it.
- Layout of the new forms: As M4-47 asks: a Map literal is flat like a List, with one space after each colon and comma: ["a": 1, "b": 2]; the empty one is [:].
- Room in the reference: The 63 signatures with their one-line results take about 2,000 of the reference's 8,000 tokens under M2-18's bytes-divided-by-3 rule. If M2-18 cannot fit them, the first rows to go are extend, pad_right, trim_start, trim_end, is_empty and values, in that order; each dropped row is a change to this table and is listed in that pull request.
- Changed in the final record: All seven picks stand: 9 types, 4 constructors, 63 functions; no new diagnostic code or trap kind. Made exact: the sort is a fixed procedure (halves split at lo + (hi - lo) / 2; the right item is taken only when before(right, left) is true), so any passed function has one result; a Map key type other than Int or String is a type-mismatch; swap checks both positions even when they are equal; a pad width below zero changes nothing; every growing row traps `memory` at the item limit; lines is split on line feed minus a trailing empty piece. No wrong-name table is built until practice runs show which spellings cost edits. The full record is docs/decisions/M2-14.md.

### M2-14 · what this changes in TASKS.md

- M2-08: The names of M2-03's number rows are confirmed as written; no row is added or renamed.
- M2-13: The built-in names table holds the 63 function names and the Upper-case names listed here, grant names included; a user definition that reuses one is rejected with a fix.
- M2-16: Adds one rule: a Map key type other than Int or String is rejected.
- M2-20: Types [:] and a Map literal from position like [], and types m[key] as a read, an assignment target and a place.
- M2-21: A place after & may pass through Map index steps; in the overlap rule they count like List index steps.
- M2-34: Grows a little: it parses and checks the Map literal (or M2-17 does, if it has not started), builds Map as an ordered map and implements the one fixed merge sort.
- M2-33: Its list is every row here outside the List and Map rows and outside M2-08's number rows; non-ASCII tests show that case and white-space functions leave those characters alone.
- M2-18: The reference carries the 63 rows as one signature table with one-line results and the count idiom; plan about 2,000 tokens for it, with the fallback order under defaults.
- M2-27: Prints a Float and a Map in failed comparisons in the text form fixed here.
- M2-39b: Its cases include a sort with a function that contradicts itself, Map key order, the missing-key trap with += on a new key, the six white-space characters, and the edge cases of split, lines and words.
- M3-16: The weak-spot tags stand: T3 needs a named before function with no closure, and R2 has no built-in priority queue.
- M4-03: len and split in its samples are real built-in names, so the count_words text there stays valid as written.
- M4-38b: The map is ordered by key, so it is a sorted structure with comparison on Int and String and no hashing.
- M4-32: String built-ins need no Unicode tables: case and white space follow the ASCII rule.
- M5-16: A Float and a Map in tree text print in the form fixed here.
- M6-14: Per-type copies of sort and sort_by must make the same calls in the same order as the fixed merge sort.

## M2-15 · Decide headless apps: first message, effects, grants, ending, effect failure, standard input and handler traps

### M2-15.1 How does a failed file read or write reach the program?

**A second message, or:** (recommended). ReadFile and WriteFile take two labelled fields, both required: then: for success and or: for failure, a function from the reason to a message. Costs one more variant and one more arm in every program that touches a file, and the paper's wc gains them.

```
enum Msg { Start(path: String), Loaded(text: String), Failed(why: String) }

fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path) => [ReadFile(path, then: Loaded, or: Failed)]
    Loaded(text) => [Print(text), Exit]
    Failed(why) => [Print("cannot read: {why}"), Exit]
  }
}
```

- Why: A failure is then one more message, which fits flat patterns and the rule that every variant is named: the checker, not a test run, makes the program say what a missing file does, which is M2-12's own reason. Paka has no unit type and no tuples, so a Result for a write would have nothing in Ok. Making or: optional later is a free loosening; the reverse is not.
- Locks: The shape of every file effect and of M7-02's foreign effects, which follow it. The paper's Loaded(text: String) stays as written. Earlier samples that write ReadFile(path, then: Loaded) alone gain or: when they become tests or reference text.
- Not chosen: A Result in the message (then: is the only field and its message carries a Result. It reuses M2-12's type, but patterns are flat (M2-02), so every answer needs a match inside a match, ? cannot stand in update, and a finished write has nothing to put in Ok.) · or: may be left out (As the first option, but without or: a failure ends the program like a failed Print, status 3. The paper's wc runs unchanged, but a forgotten or: shows up only when a test feeds a missing file.) · Failure always ends (A failed file effect always ends the program with status 3. Nothing to handle, but a task cannot print its own message, which M2-01 and M2-12 require.)
- Against: A Result in the message keeps one failure mechanism in the whole language (line 357) and needs no second field. Leaving or: optional would keep the paper's wc and five decided samples as written and save a variant and an arm, about 30 tokens, in every program; every task reads a file, and a forgotten or: is a check error that counts as an edit (M3-01.5).
- Look again when: Practice runs show the extra variant and arm, or a forgotten or:, costing Paka counted edits or clearly more tokens than Rust's and TypeScript's file error handling on the same tasks. Making or: optional is then free.

### M2-15.2 What does a file failure carry?

**One of six fixed texts** (recommended). The reason is a String: "not found", "no permission", "not a file", "not text", "no space" or "failed"; the run loop maps each system error to one of them. Costs: a misspelt comparison is not caught by the checker.

```
fn explain(path: String, why: String) -> String {
  if why == "not found" {
    "no such file: {path}"
  } else {
    "cannot read {path}: {why}"
  }
}
```

- Why: It takes no names from users, prints directly, and a new cause can be added without breaking a match. Most programs only pass the reason on or ignore it.
- Locks: The six texts, which programs may compare against. Adding a text later is cheap; changing one breaks comparisons.
- Not chosen: A built-in enum (The reason is a built-in enum with one variant per cause. Checked spelling, but six variant names are taken from every program, a match must name them all, and adding a cause later breaks every such match.) · The system's own text (The reason is whatever the operating system reports. Most detail, but it differs between systems and library versions, so the same program prints different things.)
- Look again when: Programs after 'go' branch on the reason often enough that misspelt comparisons become a real source of bugs.

### M2-15.3 In what order are a handler's effects performed and their answers handled?

**List order, answers queued** (recommended). The runtime performs the effects one at a time in list order; each answer joins the end of one queue of messages, and when the list is done the next message in the queue is handled. Costs: nothing runs in parallel, and an answer still queued when the program exits is dropped.

```
// one handler returns
//   [ReadFile(a, then: Loaded, or: Failed), ReadFile(b, then: Loaded, or: Failed), Print("asked")]
// the runtime reads a, reads b, prints asked,
// then handles the answer for a and performs its effects,
// then handles the answer for b
```

- Why: It is a plain loop over one queue, gives one result for the same inputs, and lets a program match answers to requests by counting. It is the rule M5-03 already fixed for visual apps: effect results wait in one queue and each becomes a message when its turn comes. Another order here would be a second model.
- Locks: The order of all output and file changes of every program. Concurrency (line 356) and the step-by-step notation (line 355) stay not decided; whatever comes later must give this order for programs written now.
- Not chosen: Answer handled at once (Each answer is handled, with all its own effects, before the next effect of the list is performed. A failure then cuts off the rest of the list, as in step-by-step code, but update runs in the middle of another handler's list, against the one queue M5-03 fixed.) · No promised order (The runtime may perform effects in any order or at the same time. Fast for many files, but the same program can print in different orders, against line 161.)
- Against: Handling each answer at once would let a failed write stop the Print and Exit that follow it in the same list; under the pick that failure is dropped without a word.
- Look again when: After 'go', when concurrency or a sequential notation is designed; or earlier if practice runs show programs losing a write failure by exiting in the same list.

### M2-15.4 How do the command-line arguments reach the first message?

**One String field per argument** (recommended). M1-06.3 is kept: the start: line names the first message, and its fields, all Strings, are filled from the arguments by position. A wrong number of arguments ends the run before any handler with a usage line and status 2; costs: a program cannot take a varying number of arguments.

```
enum Msg { Start(src: String, dest: String) }

// paka run copy.paka a.txt b.txt sends Start("a.txt", "b.txt")
// paka run copy.paka a.txt gives a usage line and status 2
app copy {
  start: Start
  update: update
  grants: [ReadFiles, WriteFiles, Console]
}
```

- Why: It is the paper's wc as written and the owner's start: line with nothing reserved. The runtime checks the count, so no program carries usage code; a List field for the rest can be allowed later without breaking a program.
- Locks: The start variant of a headless app has only String fields. Status 2 for a wrong argument count, the same number paka itself uses for bad usage.
- Not chosen: One List of all arguments (The start variant has one List<String> field holding every argument. Any number of arguments works, but each program checks the count itself, and the paper's Start(path: String) changes.) · Arguments by an effect (The first message carries nothing; the program asks for the arguments with an effect and gets them in a second message. Uniform with files, but every program pays one more round and one more grant question.)
- Look again when: A task or real program needs a varying number of arguments; a last field of type List<String> is then a free loosening.

### M2-15.5 Do the owner's two M1-06 rules on refused effects and failed Print stand, and are they trap kinds?

**Both kept, as effect stops** (recommended). Both rules stand as the owner chose them: an effect without its grant is refused when it is about to be performed, a Print that cannot write always ends the program, and each ends the program as a trap does, with one line on standard error and status 3. They are not added to the list of trap kinds, because they happen after the handler and have no source position; costs: a missing grant is found only when that effect runs.

```
$ ./light_no_console
light: effect Print refused: grant Console is missing
$ ./light > /dev/full
light: effect Print failed: no space left on device
(status 3 in both)
```

- Why: Nothing in the wider language makes either of the owner's choices impossible, so both stand unchanged: the program ends as it does at a trap. Keeping them out of the list of trap kinds keeps M2-06's rule that a trap has a kind and a position.
- Locks: Two stop lines of the run loop, effect refused and effect failed, both status 3, in M1-06's wording. Only the stream, the status and that it is one line are meaning; the wording is help, as M1-06.1 says. No M1 case changes by this point. In M4-03's run reply trap is null for a stop.
- Not chosen: Kept, as trap kinds (The same behaviour, reported as two new trap kinds with the position where the effect was built. One report shape, but every effect value must then carry a source position through every back end.) · Missing grant at check time (The checker rejects a program whose code names an effect its app does not grant. Earlier feedback, but it needs the reach of every function through the whole program, which the paper rejected (line 344), and it reverses the owner's choice.)
- Look again when: Agents in M4 often hit refusals late in a run; a check-time warning can then be added without changing what a program means.

### M2-15.6 How does a program end, and how does it choose its exit status?

**Exit and ExitWith(status)** (recommended). Exit ends the program with status 0, as now; ExitWith(n) ends it with status n, which must be 0 or 5 to 255, since 1 and 2 belong to the paka tool and 3 to traps and stops. Costs one more effect name.

```
fn finish(errors: Int) -> List<Effect<Msg>> {
  if errors == 0 {
    [Print("ok"), Exit]
  } else {
    [Print("{errors} errors"), ExitWith(20)]
  }
}
```

- Why: It keeps the paper and every M1 case as written and adds one row. Under M2-09 one variant name cannot be both a bare value and a call, so a second name is the only way to keep Exit.
- Locks: The two names, and that status 3 always means a trap or a stop. An ExitWith status outside 0 and 5 to 255 ends the program as a failed effect, status 3. 101 is legal here; M3-11 never uses it, as M3-04 ruled.
- Not chosen: Exit(status) always (One effect that always carries its status. One name, but light.paka, its M1 cases and the paper's samples must all be rewritten to Exit(0).) · Exit only, status 0 (A program cannot choose a status. Nothing to add, but M3-04's error status cannot be given and tasks could not tell an error from success.)
- Look again when: A real program after 'go' must exit with status 1, as many command-line tools do; the range can be widened for free.

### M2-15.7 What happens when no message is left and the program has not performed Exit?

**An error, status 3** (recommended). The program ends with one line on standard error and status 3; its earlier effects stay done. Costs: a forgotten Exit is found only when the program runs, and the M1 spec draft's rule for this case changes.

```
$ paka run quiet.paka
done
quiet: ended without Exit
$ echo $?
3
```

- Why: Every program then ends in one written way (line 51), and the rule still makes sense when later milestones add messages from outside. Allowing a quiet end later is a free loosening; forbidding it later breaks programs.
- Locks: A third stop line in the same form as the two above, '<app>: ended without Exit', status 3. It changes the M1 spec draft's end rule, so two of its run cases get new expected results, listed under defaults.
- Not chosen: Ends quietly, status 0 (Nothing left to do means success. Least to write, but then Exit is optional at the end, two spellings for one program, and programs that rely on it would hang once messages can arrive from outside.) · Waits forever (The program stays alive, as a server would. Honest about the model, but in M2 it is only a hang that the harness must kill.)
- Against: The M1 spec draft's quiet end is the simplest rule and needs no Exit at the end; a model that forgets Exit loses a first try on a program that printed the right answer.
- Look again when: M3 logs show 'ended without Exit' behind more than about one in twenty of Paka's counted edits; a quiet end with status 0 is then a free loosening.

### M2-15.8 Can a headless program read standard input?

**None in M2** (recommended). No effect reads standard input, so there is one way in: files. Costs: a Paka program cannot sit in a pipe yet.

- Why: Nothing needs it before the go/no-go, and adding an input effect later breaks no program.
- Locks: Nothing. M4-03 adds no stdin field.
- Not chosen: Whole input as one text (One effect gives all of standard input as a String, like a file read. Small, but it adds an effect, a grant question and an end-of-input rule that no task uses.) · Line by line (An effect gives the next line. Needed for interactive programs, but it forces the order of prints and reads and makes the open notation of line 355 hard to put off.)
- Look again when: After 'go', the first program that must run inside a pipe or answer a person.

### M2-15 · settled without asking

- The app form: app name { } with one key per line, each written once: state (optional in a headless app), start (required in a headless app), update, grants (required, may be []). An app with no view key is headless (line 126). Key order has no meaning.
- A program with no app, or with several: A program with no app checks: paka check accepts it and paka examples runs its examples, because M2-43 tests the paper's counter with no app and most conformance cases are examples only. paka run refuses it with one line and status 1. More than one app in a program stays a check error, as the M1 spec has it; allowing several later is a free loosening.
- The update function: With state: fn update(s: inout S, m: Msg) -> List<Effect<Msg>>; without: fn update(m: Msg) -> List<Effect<Msg>>. The update a headless app names must give the effect list, since returning nothing could never end the program. The name update is not reserved; the app block names the function. A function no app names is an ordinary function, so the counter's update, which returns nothing, checks.
- First state: The state type built from its field defaults (Tally {}), so it is a record with a default on every field: the same rule as M5-01.6, whose look-again does not fire. Then the start message is handled.
- The effect list: Print(text: String). ReadFile(path: String; then: fn(String) -> Msg, or: fn(String) -> Msg). WriteFile(path: String, text: String; then: Msg, or: fn(String) -> Msg). Exit. ExitWith(status: Int). All are variants of the built-in Effect<Msg>, where Msg is the app's message enum.
- Which fields take a function and which a message: A field takes a function when the answer carries data: then: of ReadFile gets the text, every or: gets the reason. then: of WriteFile takes a message value, because a finished write carries nothing; so it may carry context, as in then: Wrote(name). A function here is a named fn or a variant with fields (M2-09). It is called when the answer's turn comes in the queue; a trap inside it ends the program like a trap in a handler.
- Grants: Console allows Print, ReadFiles allows ReadFile, WriteFiles allows WriteFile; Exit and ExitWith need none. WriteFiles is a new name in the style of the paper's ReadFiles and Console (line 259). Grants take no arguments, so M7-01's look-again does not fire. An unknown or repeated name in grants is a check error; a grant that is never used is accepted.
- When a grant is checked: When the effect is about to be performed, in list order. A refused effect is not performed, the effects after it are dropped, and the program ends. An effect that stands after an Exit is never reached and so never refused.
- Print, exactly: Writes the text as UTF-8 and one line feed to standard output. It is written before the next effect is performed, so a failed Print stops everything after it. No effect writes to standard error and none prints without the line feed.
- ReadFile and WriteFile, exactly: ReadFile gives the whole file as text, bytes unchanged; a file that is not UTF-8 fails with "not text" (the case M2-03 left here), and one too large for the interpreter's String limit fails with "failed". WriteFile creates or replaces the file with the text's UTF-8 bytes and makes no directories; after a failure the file's content is not promised. A relative path starts at the working directory.
- Reason texts: "not found": the file or a directory on its path does not exist. "no permission": the system refused. "not a file": the path is a directory. "not text": not UTF-8. "no space": the disk is full. "failed": anything else.
- Answers carry no path: A then: or or: answer holds only the text or the reason. A program that asks for several files keeps the names it is waiting for in its state, in the order asked; answers arrive in the order the effects were performed.
- Answers still queued at Exit: Dropped, a failure included: [WriteFile(...), Print("done"), Exit] ends with status 0 even if the write failed. This cannot be made a stop, because an ExitWith from an or: handler rightly leaves later answers queued. A program that must know a write worked exits from its then: message; the reference says so in one line.
- A trap in a handler: Ends the program (line 160): the trap report of M2-06 on standard error, status 3. None of that handler's effects is performed, because a trapped handler returns no list. Effects of earlier handlers stay done.
- Rollback visibility: None in a headless app: the program ends at a trap, so nobody can see the restored state. M2-36 tests no rollback; it arrives with M5-25b for visual apps.
- Ending: Exit and ExitWith end the program at once with output flushed; later effects in the list and queued messages are dropped. ExitWith(0) is allowed and means the same as Exit. The process never dies by signal on its own account. What a kill from outside does is not meaning.
- The three stop lines: All on standard error, each one line, status 3: '<app>: effect <Effect> refused: grant <Grant> is missing'; '<app>: effect <Effect> failed: <reason>', which also covers an ExitWith status outside its range; '<app>: ended without Exit', which may name the last message handled. Stream, status and the one-line shape are meaning; the wording is help.
- Effects are plain values: Any function may build and return effects; only the list the app's update returns is performed. Effects compare with == (M2-09), so an example may end in update(&s, Start("a.txt")) == [ReadFile("a.txt", then: Loaded, or: Failed)]. Examples perform nothing (M2-06).
- Sequencing notation: Not decided (line 355). Chains are written by hand as messages; the order rule above is what any later notation must compile to.
- Concurrency: None, and not decided (line 356). One handler runs at a time and one effect is performed at a time; the run loop waits for each.
- The usage line: On a wrong argument count, or an argument that is not UTF-8: one line on standard error naming the app, the fields expected and the number given, for example 'copy: expected 2 arguments (src dest), got 1'; status 2; no handler runs. The wording is help. A released binary started with exactly --paka-examples runs its examples and sends no first message (M7-03); that argument is taken before the count check.
- wc under the rules: type Tally { words: Int = 0 }

enum Msg { Start(path: String), Loaded(text: String), Failed(why: String) }

fn count_words(text: String) -> Int {
  len(words(text))
}

fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path) => [ReadFile(path, then: Loaded, or: Failed)]
    Loaded(text) => {
      s.words = count_words(text)
      [Print("{s.words} words"), Exit]
    }
    Failed(why) => [Print("cannot read file: {why}"), ExitWith(20)]
  }
}

app wc {
  state: Tally
  start: Start
  update: update
  grants: [ReadFiles, Console]
}
- wc: each change from the paper: The start: line (M1-06.3, kept here). or: Failed with its variant and arm (this page, first point). count_words written out with words (M2-14; the paper calls it an ordinary function and does not show it, line 263). Single spaces instead of aligned arrows and keys (M4-47). Nothing else differs from lines 242 to 260.
- Earlier samples without or:: Samples decided before this ruling write ReadFile(path, then: Loaded) alone: in M2-01.3, M3-04.4, M4-03, M4-47 and M7-02. Each gains or: with a failure variant when it is turned into a test or reference text. M7-02's look-again on the form of built-in effects fires in this narrow sense only.
- M1-06.3 start rule: Kept and widened: the start: line stays; the start variant may now have String fields filled from the arguments. light.paka is unchanged. Case run/light: standard output 'let there be light' and a line feed, status 0, as before.
- M1-06 end rule: Kept for Exit: status 0 after output is flushed. Changed for a program that stops without Exit, which the M1 spec draft ends quietly with status 0. If the spec lands as drafted: run/no_exit gives 'let there be light' and a line feed on standard output, the line '<app>: ended without Exit' on standard error, status 3; run/empty_list gives empty standard output, the same line, status 3. Every other M1 run case keeps its result.
- M1 no-app rule: Changed, as ruled above. If the M1 spec lands as drafted: check/no_app and check/empty_file now check, status 0 and no output; check/order_no_app_bad_type reports only the bad type. check/two_apps and check/order_two_apps_bad_grant keep their results.
- M1-06.1 missing-grant rule: Kept unchanged. Case run/light_no_console: empty standard output, the line 'light: effect Print refused: grant Console is missing' on standard error, status 3, as before.
- M1-06.2 failed Print rule: Kept unchanged. The /dev/full run and the closed-pipe run each give one line on standard error and status 3, as before, on both back ends.
- M1's narrowing on arguments: Lifted: arguments are now delivered. light takes none, so running it with an argument gives the usage line and status 2. No M1 case passes an argument, so none changes.
- Changed in the final record: All eight picks stand. Changed or added: the ExitWith range is 0 or 5 to 255, because 4 is the internal-error status; a literal status outside it is refused at check time as literal-out-of-range; the 'ended without Exit' line names no message; an answer's then: or or: function is applied when the answer reaches the front of the queue; Print writes straight to standard output with no flush step, and the file-size signal is ignored like the broken-pipe one; one new code, bad-state. Nine M1 cases are re-expected, not five; run/no_exit, run/empty_list and check/order_no_app_bad_type do not change. The full record is docs/decisions/M2-15.md.

### M2-15 · what this changes in TASKS.md

- M2-18: Writes re-expected copies under tests/cases-pending for run/no_exit, run/empty_list, check/no_app, check/empty_file and check/order_no_app_bad_type if the M1 spec lands as drafted; the reference carries the app form, five effects, three grants, six reason texts, the order rule and the one line on answers dropped at Exit.
- M2-23: The core form needs an app node, effect values whose fields hold function values, and one message queue; no rollback node.
- M2-32: Checks that the start variant has only String fields, that a headless app's update returns the effect list, that the state type has a default on every field, and that grants are known and not repeated; a second app stays rejected.
- M2-36: Implements, in the interpreter and in paka-rt's run loop, the queue order, the argument-count check with the usage line and status 2, ExitWith and the 'ended without Exit' stop. It also lifts M1's no-app check error (paka check accepts, paka run refuses with status 1) and moves all five re-expected M1 cases over the old ones. It tests no rollback, since a headless app does not survive a trap.
- M2-37: Its effects are Print, ReadFile, WriteFile, Exit and ExitWith; there is no argument effect, since arguments arrive in the start message; it maps system errors to the six reason texts. The missing-grant rule is unchanged, so it moves no M1 case.
- M2-41: Cases cover both then: forms, or: with each reason text, an effect refused after others were performed, effects after Exit, a wrong argument count, a program that ends without Exit, a write failure dropped because Exit stood in the same list, and a trap inside a then: function.
- M2-42: New reject cases: a missing or: field, a start variant with a field that is not a String, a headless update without a result, an unknown grant, a second app.
- M2-43: The counter is checked and its example run in a program with no app, which the no-app rule here allows; the wc is the text under defaults with its four listed changes.
- M3-11: The error status is 20, written ExitWith(20). Every task takes a fixed number of arguments; a task with a varying number of files names them in a list file. Statements must not test a wrong number of arguments, statuses 1 to 3 or 101, standard input, standard error text, or output that does not end in a line feed.
- M3-16: The file-tool tags stand: each file effect costs Paka a failure variant and arm on top of the hand-written chain, and a program reading several files tracks the pending names itself.
- M4-03: The args of a run call fill the start variant's fields in order; a wrong count gives status 2 in the run reply; no stdin field is added; trap is null for a stop, whose line is in stderr. Its wc samples gain or: Failed, so the version strings shown for those definitions are recomputed.
- M4-36: The compiled run loop follows the same queue order and the same three stop lines; function-valued effect fields are called through M4-37b's identities.
- M6-17: A trapping handler ends the program with status 3 and none of its effects performed; there is no rollback to link in for headless apps.
- M5-06: This settles its open case: an effect a UI example's app does not grant stops the example with M2-06's trapped status, its report being the refusal line with no position; granted effects are still dropped in examples.
- M5-22: Computes that trapped status for a refused effect, as M5-06 now has it.
- M5-25b: State rollback after a trapping handler is first built and tested here, not in M2-36.
- M5-35: On a window close the effect being performed finishes and the rest are dropped, like effects after Exit.
- M7-02: A foreign effect takes then: and or: like ReadFile, and its failure reasons (host could not start, host died, timed out, bad reply) are fixed texts in the same style; its own ReadFile sample gains or:.

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

## M6-01 · Decide what release IR may promise the optimizer and what proves it

### M6-01.1 What may count as proof that a promise to the optimizer is true?

**Emitter alone, shape checked** (recommended). A promise is allowed only when the compiler's own way of writing the IR makes it true for every program, with nothing resting on an analysis of the user's code, and where possible only in a shape the linter can see. Costs some speed where only a checker fact would help.

```
  %ok = icmp ult i64 %i, %len
  br i1 %ok, label %in, label %trap7, !prof !0
in:
  %p = getelementptr inbounds double, ptr %data, i64 %i
  %x = load double, ptr %p, align 8   ; inbounds: every use is a load or store
```

- Why: A promise the emitter makes true by construction cannot be broken by any program, so the proof does not grow with the language. Checker facts get their own evidence and their own signed decision (M6-23), not a row here.
- Locks: The rule every later row must meet, and what M6-05's linter checks. Loosening it later is a new owner decision of M6-23's kind; tightening it later only costs speed.
- Not chosen: Checker facts count too (Facts the checker proved about user code, such as 'inout arguments never overlap' (GENESIS line 90), may be passed on as promises. Faster, but one hole in a checker rule becomes undefined behaviour.) · No promises at all (Keep M1's ban list for good. Nothing can be wrong, but without 'inbounds' LLVM must assume an address can wrap around, which can block loop optimizations and puts the array-loop parity of line 97 at risk.)
- Look again when: M6-23 signs the no-overlap promise: that is the one planned exception, added as its own row with its own evidence.

### M6-01.2 Who may remove a bounds or overflow check?

**Only the optimizer** (recommended). Paka writes every check the source has into the IR and LLVM removes the ones it can prove never fire; Paka's part (M6-19) is only to write loops in a shape LLVM can see through. Costs the checks LLVM cannot prove.

```
fn total(xs: List<Int>) -> Int {
  var sum = 0
  // bounds check on xs[i]: written, then proven away by LLVM
  // overflow check on +=: written, and it stays
  for i in 0..<len(xs) {
    sum += xs[i]
  }
  sum
}
```

- Why: There is one prover, it is LLVM's and it is the most tested, and it is sound as long as the IR holds no false promise, which the first point guards. Each trap site returns its own position, so LLVM cannot merge or reorder traps that differ.
- Locks: M6-19 shapes IR and removes nothing. Adding a second prover later breaks no program.
- Not chosen: Paka's own range analysis (The Paka compiler also proves ranges and leaves checks out before LLVM sees them. Removes more, but adds a second prover whose every bug is a missing check with no symptom.) · Hoist with exact rerun (Paka moves checks out of loops and, when a moved check fails, reruns the exact path to find the first trap (the route M2-06 left open). Helps loops LLVM gives up on, but needs two copies of every such loop.)
- Look again when: M6-19's loop list has loops that matter where the check stays and no reshaping helps.

### M6-01.3 How large is the first table?

**Small core, grown by measurement** (recommended). Start with the few rows array loops need (listed under defaults) and add a row later only with a by-construction proof, a linter rule, a measured gain on an M6-02 benchmark and a reviewer told to break it. Costs some speed until rows are added.

```
allowed:  nounwind | inbounds (use is load/store) | natural align
          hints: !prof, cold, inline marks | internal, private, constant
never:    nsw nuw exact | fast-math | undef poison unreachable
          assume | mustprogress willreturn | tail musttail | noalias
```

- Why: Nothing is promised before a number shows it is needed, and each added row arrives with its own proof and its own review. Adding a row breaks no program; removing a wrong one after release does.
- Locks: The procedure for adding a row. The first rows themselves are cheap to extend.
- Not chosen: Rust's full set now (Also noundef, nonnull, dereferenceable, readonly, value ranges and lifetime marks from the first day. Likely faster on pointer-heavy code, but a dozen more rows to prove before a single benchmark shows which of them matter.)
- Look again when: M6-28 or M6-29 show a gap to Rust that a missing row explains; that row is then proposed through the procedure.
- Owner must still: Approve each later pull request that adds a promise row: it changes the table you signed.

### M6-01.4 What is the tail-call rule?

**Never written, never promised** (recommended). Paka's IR never marks a call 'tail' or 'musttail', though LLVM may still turn a call into a jump where it proves that safe. The language promises nothing, so a program that needs a bounded stack must use a loop.

```
// interpreter: count_down(1000000) traps stack past 10,000 nested calls
// release: may run as a loop and give 0
fn count_down(n: Int) -> Int {
  if n == 0 {
    0
  } else {
    count_down(n - 1)
  }
}
```

- Why: The 'tail' mark is itself a promise about the caller's stack slots, and inout arguments often point at them, so Paka never writes it. Leaving the choice to LLVM costs nothing and adds no meaning to the language.
- Locks: Programs cannot rely on deep tail recursion. Promising it later breaks no program; taking a promise back would.
- Not chosen: Promised with musttail (A call in tail position always reuses the frame, in every mode. Deep recursion then works everywhere, but the interpreter and dev mode must do the same and the reference must define 'tail position'.) · Forbidden with notail (Every call is marked so it is never turned into a jump. Stack use is then closer between modes, but it costs speed for a difference the paper already accepts.)
- Look again when: Agents in M3 or M4 often write deep tail recursion that traps stack in the interpreter.

### M6-01.5 What does the IR say at a place the checker proved cannot be reached?

**Return internal-error status** (recommended). Such a place returns one reserved status that the runtime reports as an internal error (M1-14's line and exit status 4). Costs one compare and branch per match that LLVM cannot remove.

```
  switch i64 %tag, label %bad [ i64 0, label %arm0
                                i64 1, label %arm1 ]
bad:
  ret i32 255        ; the reserved internal-error status; M4-05 picks the number
```

- Why: A compiler or runtime bug then shows up as one clear line instead of silent damage, for the price of a predictable branch. Changing to 'unreachable' later is a table row like any other.
- Locks: Little: one reserved status value in compiled code, numbered by M4-05, which the runtime turns into exit status 4.
- Not chosen: Write unreachable (The place is marked unreachable and LLVM drops the branch. Fastest, but a wrong tag then runs into whatever code comes next, with no report.)
- Look again when: M6-29 shows match dispatch costing measurable time in parsers.

### M6-01.6 What may the IR promise about runtime functions written in Rust?

**Only: does not unwind** (recommended). Runtime declarations carry 'nounwind' and nothing else; it holds for any Rust code, because Rust stops the process rather than let a panic leave a C-convention function. Costs LLVM's knowledge that, say, a length call changes nothing.

```
declare i32 @paka_rt1_list_grow(ptr, i64) nounwind
```

- Why: It is the one fact the Rust language itself guarantees for every entry, so it can never go stale. Hot paths such as item access are written inline in the IR (M6-11), where LLVM sees them anyway.
- Locks: Little. More marks can be added row by row through the procedure.
- Not chosen: Allocator and read-only facts too (Also 'fresh memory' on the allocator's result and 'reads only' on query functions. Helps LLVM keep values in registers across calls, but each mark must stay true through every later edit of the Rust code.) · Nothing (No attribute on any declaration. Safest to state, but every runtime call then looks as if it could unwind, which blocks optimizations around it for no gain in safety.)
- Look again when: A benchmark shows a runtime call inside a hot loop that LLVM cannot move for lack of a read-only or fresh-memory fact.

### M6-01 · settled without asking

- Row: nounwind: Allowed on every function Paka defines and every runtime declaration. Proof: the emitter never writes invoke, landingpad or resume (the linter rejects them), a trap is a status return, and Rust stops the process rather than let a panic leave a C-convention function. Each runtime entry also catches panics, so what the user sees is status 4 and not an abort.
- Row: inbounds: Allowed on a getelementptr only when every use of its result is the address of a load or store (or another such getelementptr), so it promises nothing the access itself does not already need. The emitter writes it only in the one helper that writes the bounds check, the address and the access together; field addresses use constant offsets inside a live value. An address passed to a call is written without inbounds.
- Row: alignment: Loads and stores use the type's natural alignment from M4-22's layout table, written out on every instruction. Proof: the compiler lays out every value itself and the runtime allocates at 8-byte alignment or more; M6-10's test compares both sides.
- Row: hints that cannot change meaning: Allowed freely: !prof branch weights (to mark trap branches unlikely), llvm.expect, cold, hot, noinline, inlinehint, alwaysinline. A wrong hint costs only speed.
- Row: linkage and constants: Allowed: internal and private linkage, unnamed_addr (a Paka program cannot see an address), and 'constant' on globals. Proof for constant: release IR defines no changeable global, the linter rejects any store whose address is a global, and a list or string whose bytes sit in a constant is marked as such and copied before its first write, by compiled code and by the runtime alike (M6-11 tests it). A global the runtime owns, such as a stack limit, is declared without 'constant' and only loaded.
- Plain words that promise nothing: Besides the rows, the linter's allow-list holds the words that carry no flag and are defined for every value they are given: integer and float arithmetic and comparison, select, phi, br, switch, ret, getelementptr without flags, extractvalue, insertvalue, zext, sext, trunc, sitofp, ptrtoint, the overflow-reporting intrinsics and llvm.memset, plus load, store, call and alloca under the rules below. A lowering task adds a word it needs in its own pull request, with a note that no input makes LLVM call its result undefined or poison.
- Instructions that are undefined on bad input: sdiv and srem only after the zero and smallest-Int-by-minus-one checks; shl and ashr only after the 0 to 63 check; fptosi only after the range and NaN check that traps bad_conversion. Each is written only by the helper that writes its check first. Any other such instruction the emitter comes to need (udiv, urem, lshr, fptoui) follows the same rule: behind its check, or with a constant operand that is in range. Checked + - * use LLVM's overflow-reporting intrinsics; wrap_add, wrap_sub and wrap_mul are plain add, sub and mul.
- Copying memory: llvm.memmove by default. llvm.memcpy only when the destination was allocated in the same function, since memcpy is undefined when the two ranges partly overlap.
- Never allowed: nsw, nuw, exact, disjoint, nneg, samesign and nusw; undef and poison as written values; unreachable; llvm.assume; mustprogress and willreturn (an endless loop is a legal Paka program under M2-06 and must not be deleted); tail and musttail; invoke; inline assembly; type-based alias metadata; value-range, nonnull, noundef, dereferenceable, readonly, readnone, memory(...), captures and lifetime marks until a row is added for them.
- Float: No fast-math flag ever (fast, nnan, ninf, nsz, arcp, contract, afn, reassoc), and no llvm.fmuladd or llvm.fma, so nothing is fused (M2-03). Allowed float intrinsics are the exactly rounded ones: sqrt, floor, ceil, trunc, round, fabs. How the canonical NaN is kept is M6-08's: the rule is only that every place a program or a test can see a NaN's bits (today the text form) sees the canonical ones. Forcing them after each operation is also correct, but it puts a compare and a select into every float loop and so risks the parity of line 97.
- noalias: Not allowed anywhere in the first table, neither on parameters nor on results. Only M6-23 can add it for inout parameters, at the exact places it signs. Until then a loop over two lists keeps LLVM's own run-time overlap check, which line 90 expects to be absent; the benchmark table is made after M6-24, so it shows whatever was signed.
- Stack slots: alloca only with a constant size and only in a function's first block, so M6-16 can bound every frame. No lifetime marks in the first table. M6-16 may read the stack position (the address of one of its own slots, or an intrinsic it adds as a plain word), compare it with a limit the runtime owns, and mark a large-frame function noinline; it uses no attribute that relies on a guard page, since no signal handler is installed.
- What a trap shows and which trap wins: Taken from M2-06 as given: kind and position, first in evaluation order. Each trap site returns its own constant, so LLVM cannot merge sites that differ; stack and memory traps are compared on kind only.
- What the linter reads: The IR text Paka writes, before clang optimizes it. It is an allow-list: any keyword, attribute, metadata name or intrinsic that is neither a row nor a plain word fails. What LLVM adds for itself afterwards is LLVM's own proof and is not linted.
- The clang command: It may add no flag that loosens meaning: no -ffast-math and no floating-point contraction flag. Optimization level and target CPU are M6-02's one constant.
- Adding a row later: A promise row needs one pull request with: the proof that the emitter alone makes it true, the linter rule, a measured gain on an M6-02 benchmark, a level 4 reviewer told to break it, and the owner's approval, since it changes the signed table. A plain word needs only its note and the normal review. No row rests on a checker analysis of user code except through a signed decision like M6-23.
- Measurement-only builds: A build that omits a check family (M6-30) stands outside this table. Its numbers count only under M6-02's rule that the normal build ran the same input without a trap, so every omitted check would have passed.
- What the table does not cover: Every load and store also relies on its address being live and written. That rests on the language itself (bounds checks, one owner, borrows that cannot be stored) and on the layouts of M6-10 to M6-13, and is tested by three-mode conformance and the fuzz. No flag is involved, so it is not a row.

### M6-01 · what this changes in TASKS.md

- M6-05: Its allow-list has two parts, the promise rows and the plain words, and it adds two checks that read whole lines: every use of an inbounds result is a load or store address, and no store targets a global. Its level 2 reason ('token scan') no longer fits fully and should be looked at again.
- M6-07: Checked arithmetic uses the overflow-reporting intrinsics with no nsw or nuw; division and shifts go through the helpers that write their checks first; the fall-through of a match returns the reserved internal-error status.
- M6-08: No fmuladd or fma and no flag of any kind on float operations; only the six exact float intrinsics are called. It chooses where NaN bits are made canonical; if it does so after every operation, M6-02's float kernels will show the cost.
- M6-09: Field addresses that are passed to a call (inout arguments) are written without inbounds.
- M6-10: Its test also covers two facts that rows lean on: the runtime allocates at 8-byte alignment or more, and a buffer that sits in a constant is marked as such.
- M6-11: Item access goes through one helper that writes check, address and access together; copies use memmove unless the destination is fresh; a list or string whose bytes sit in a constant is copied before its first write, with a test; runtime declarations carry nounwind only.
- M6-16: May assume fixed-size stack slots in the first block only and no 'tail' marks written by Paka; it adds the word it reads the stack position with as a plain word and uses no guard-page attribute.
- M6-19: Shapes loops only and removes no check itself; a check that stays is fixed by reshaping or by proposing a table row with a measured gain.
- M6-20: When the interpreter ends in a stack trap, release may finish or keep running (a tail call LLVM made into a loop), so such runs are cut by a time limit and not filed as a divergence.
- M6-21: Its planted 'wrongly removed check' bugs are emitter bugs that leave a check out, since Paka has no pass that removes checks.
- M6-24: The noalias row does not exist until M6-23 signs it; M6-24 adds the row and its linter rule together.
- M6-30: A measurement build drops checks but keeps the promises that rested on them, so it is valid only on inputs the normal build ran without a trap.
- M2-41: The shared unbounded-recursion case must use the call's result in a checked operation after the call (for example 1 + down(n + 1)), so LLVM cannot turn it into a loop and it still traps stack in release.
- M4-05: Reserves one status number for 'internal error' beside the trap numbers; the runtime reports it with M1-14's line and exit status 4.

## M6-02 · Propose the benchmark set, fairness rules and cost-measurement method

### M6-02.1 Which optimization level and target CPU does paka build use, and do the Rust and C++ builds use the same?

**Baseline x86-64, -O3** (recommended). Full optimization and the plain x86-64 instruction set that clang and rustc target when given no CPU flag, so binaries run on any 64-bit PC. Costs speed on loops that wider vector instructions (AVX2) would help, equally in all three languages.

```
paka:  clang -O3 prog.ll libpaka_rt.a ...
rust:  rustc --edition 2024 -C opt-level=3 -C codegen-units=1 -C panic=abort prog.rs
c++:   clang++ -O3 -std=c++20 -fno-exceptions -fno-rtti -ffp-contract=off prog.cpp
```

- Why: No CPU flag is each compiler's own default, so no language gets an instruction set the others lack, and the constant is safe to ship. It is one constant and cheap to change.
- Locks: The flag constant in M6-04 and the conditions under which every published number was measured. Raising the CPU level later means re-running the whole set.
- Not chosen: x86-64-v3 (AVX2) (The 2013-and-later level, with wider vector instructions. Faster float and array loops, but shipped programs stop with an illegal instruction on older or low-end CPUs.) · The build machine's CPU (native) (Tune for the Ryzen 3900X the benchmarks run on. Best numbers here, but the published table then describes one machine and the build constant cannot ship.)
- Look again when: A user needs AVX2 speed; a CPU-level option on paka build is then added and the set is re-run at that level beside the baseline.

### M6-02.2 How must the Rust and C++ versions of a benchmark be written?

**Idiomatic safe code, tuned once** (recommended). Same algorithm, input and checksum in all three, standard library only, one thread: safe Rust with iterators allowed, C++ with std::vector and plain indexing, no vector intrinsics, pragmas or 'restrict'. Each version gets one tuning pass by a second agent and all are frozen by commit before any timing; costs that tuning pass.

```
// rust: safe, the idiom a Rust programmer would write
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
```

- Why: It compares what a competent writer gets in each language, which is the claim the paper makes. The tuning pass keeps the rivals from being straw men, and the freeze keeps numbers from steering the code.
- Locks: The frozen commit of all benchmark sources. After the first timing a program changes only by a logged reason, and then all three versions are re-run.
- Not chosen: Line-by-line translation (The Rust and C++ sources mirror the Paka source statement by statement. Easiest to audit, but it denies Rust its iterators and so flatters Paka.) · Fastest known versions (Take the fastest published programs, with unsafe code, hand-written vector code and threads. It tests the row where the paper already says Paka loses.)

### M6-02.3 Which programs make up the set?

**Eleven small own programs** (recommended). Four float and array kernels, three integer kernels and four pointer-heavy programs, each under about 150 lines per language (list under defaults). Costs about 40 small source files to write and keep in step, since two kernels also have a wrapping twin.

```
float/array: saxpy_dot, matmul, stencil, nbody
integer:     sieve, collatz (each checked and wrapping), mix (wrapping only)
pointer:     expr_parse, binary_trees, bfs, word_count
```

- Why: Each program stresses one thing the paper makes a claim about, so a gap can be traced to a cause. Small programs can be checked line against line across three languages.
- Locks: The program list M6-28 and M6-29 write. Adding a program later is free; dropping one after seeing its number is not allowed.
- Not chosen: Benchmarks Game ports only (Port the well-known public set. Rival versions already exist, but most of the fast ones use threads, unsafe code or big-number and regex libraries Paka does not have.) · A few large programs (Two or three programs of a thousand lines each, such as a full interpreter. Closer to real use, but one number then mixes every cost and explains nothing.)
- Look again when: M2-09 or M2-10 add user generics, shared data or pools before M6: one program per new feature is then added before the freeze.

### M6-02.4 What are the integer kernels compared against, given that Rust and C++ do not check overflow by default?

**Both checked and unchecked Rust** (recommended). Paka checked is shown beside Rust built with overflow checks on (like for like) and beside default Rust and C++ (what their users get); Paka wrapping is shown beside Rust's wrapping operations and C++ unsigned arithmetic. Inputs never overflow, so all checksums agree; costs one extra Rust build per integer kernel.

```
// checked form: traps overflow if a value leaves Int
x = 3 * x + 1
// wrapping twin: same result on these inputs, never traps
x = wrap_add(wrap_mul(3, x), 1)
```

- Why: The two columns answer two different questions, and both matter: is Paka's checked code as good as Rust's checked code, and what does checking cost against the usual build.
- Locks: The columns of the integer rows in the published table.
- Not chosen: Default Rust only (Compare Paka checked with Rust and C++ as shipped. Shows the gap users see, but hides how much of it is the checks and how much is Paka.) · Checked Rust only (Compare only like for like. Fair to Paka, but hides the price of the language's own choice to check.)

### M6-02.5 Are measurement-only builds that leave out one family of checks accepted?

**Accepted, after a clean run** (recommended). Yes, under M6-30's fence: only a compiler built with the measure feature takes the switch, the binary prints a warning banner on standard error, and it is never shipped. A measured run counts only if the normal build ran the same program and input first, without a trap and with the same checksum; costs the switch through several lowerings.

```
(paka built with the measure feature; the normal paka rejects --omit)
$ paka build stencil.paka --omit bounds -o stencil_nb
$ ./stencil_nb
MEASUREMENT BUILD: bounds checks omitted, not a Paka program   (standard error)
4183902771
```

- Why: It is the only way to put a time on a check that cannot be switched off. The clean-run rule means every omitted check would have passed, so the measured program computes the same thing.
- Locks: M6-30 is built; the spelling of its switch is M6-30's. The overflow family needs no such build: its cost is the checked kernel against its wrapping twin.
- Not chosen: Refused: count checks instead (No build without checks ever exists. The table reports how many checks stay in hot loops and how often they run, not their time, so the paper's cost question gets a count, not a cost.)

### M6-02.6 What is timed, and how?

**Whole process, timed outside** (recommended). The runner times each program from start to exit, pinned to one core, with inputs sized so a run takes 1 to 3 seconds and start-up cost is too small to matter. Costs that input building is inside the timed span, equally for all three.

```
$ paka-bench run stencil
stencil  paka 1.412 s  rust 1.398 s  c++ 1.371 s   (median of 10, spread 1.9%)
stencil  paka/rust 1.01   paka/c++ 1.03   expected: parity
```

- Why: It needs nothing added to the language and treats the three languages alike. Runs of a second or more keep the fixed start-up cost well under one percent.
- Locks: The run protocol in M6-27 (details under defaults).
- Not chosen: Timer inside the program (Each program times only its kernel. Most exact, but Paka would need a Clock effect and grant built just for benchmarks.) · Count instructions with perf (Count executed instructions instead of time. No noise at all, but instruction counts hide cache misses and vector width, which are what the paper's claims are about, and on the build machine perf needs a kernel setting only the owner can loosen.)
- Look again when: Spread stays above the noise limit on the build machine even when idle; instruction counts are then added beside the times, which needs the owner to allow perf.
- Owner must still: Leave the machine idle while the published run is made. Setting the CPU governor to 'performance' or turning frequency boost off is optional and is a system setting only the owner can change; the runner records whatever is set (schedutil with boost on today).

### M6-02.7 How does the published table state that Rust is optimized by LLVM 22 and Paka and C++ by LLVM 21?

**State it; C++ is control** (recommended). Every column header names its compiler and LLVM version, and a note says the C++ column, built with the same clang 21.1.8 as Paka, is the same-optimizer comparison while a small gap to Rust may be the optimizer's. Costs nothing; the Rust gap stays partly unexplained.

```
            paka             rust             c++
            clang 21.1.8     rustc 1.96.1     clang++ 21.1.8
            (LLVM 21)        (LLVM 22)        (LLVM 21)
Note: Rust is optimized by a newer LLVM. The C++ column is the like-for-like one.
```

- Why: It needs no download and hides nothing. Compiling C++ with the same clang as Paka already gives one comparison where only the language differs.
- Locks: The table header and note that M6-32 generates.
- Not chosen: Older rustc with LLVM 21 (Build the Rust benchmarks with an older rustc that carries LLVM 21. Same optimizer everywhere, but it is a download and it measures a Rust nobody uses now.) · Move Paka to clang 22 (Re-pin clang to 22 so all three match. Cleanest table, but it is an install and it changes the pin M1 was proven on.)
- Look again when: The project re-pins clang to 22 for any other reason; the set is then re-run and the note goes away.

### M6-02.8 Which measured ratio is called 'parity'?

**Parity means within 10%** (recommended). The table always prints the raw ratio and beside it one word, fixed now: 'parity' when Paka's median time is at most 1.10 times the other's, 'as expected' on a pointer-heavy row up to the paper's 1.30, 'worse than expected' above the row's line, and 'measured' where the paper names no amount (checked integer loops). Costs that a real 8% loss is called parity.

```
matmul   paka/c++ 1.04            expected: parity         -> parity
bfs      paka/rust 1.22           expected: behind 10-30%  -> as expected
bfs      paka/c++ 1.36            expected: behind 10-30%  -> worse than expected
collatz  paka/rust 1.31           expected: behind         -> measured
collatz  paka/rust(checked) 1.06  expected: none stated    -> measured
```

- Why: The line must sit above what noise (limit 5%, see defaults) and a one-version optimizer gap can produce. The raw ratio is always printed, so a stricter reader loses nothing.
- Locks: The verdict words in the published table. Fixed before the first timing and not changed after.
- Not chosen: Within 5% (A stricter line. More honest on a quiet machine, but run-to-run noise and the LLVM version gap can each be that large, so noise would be labelled as loss.) · No words, ratios only (Print numbers and let the reader judge. Nothing to argue about beforehand, but the exit check asks for each number 'beside its expectation', and without a line nobody can say whether one was met.)

### M6-02 · settled without asking

- Program list: float and array: saxpy_dot (scale-and-add over two lists, then a dot product; the two-list kernel M6-24 names), matmul (plain matrix product on flat lists), stencil (five-point average over a grid, many sweeps), nbody (five bodies as records of Floats, with sqrt).
- Program list: integer: sieve (primes below N in a List<Bool>), collatz (longest chain below N), each in a checked form with + - * and a wrapping twin written with wrap_add and wrap_mul; mix (a 64-bit mixing generator and running hash, wrapping only, since wrap is its meaning). mix and the seed generator are defined with operations Paka has (wrapping multiply, xor, and a right shift that copies the sign bit), so all three languages compute the same bits.
- Program list: pointer-heavy: expr_parse (parse an ASCII arithmetic text into a tree of a self-containing enum, then evaluate), binary_trees (build and walk many trees), bfs (shortest paths on a graph kept as List<List<Int>>, per M2-01.1), word_count (count words of an ASCII text in a Map<String, Int>).
- Programs that wait on other decisions: If M2-10 builds shared immutable data, expr_parse gains a shared-subtree form for the reference-count row. If pools exist by M6, bfs gains a pool form for the handle-check row. A cost whose feature the language does not have reads 'not built' and no number is invented; today that is the case for handle checks, since pools are not decided (M2-01.1). The M6 exit check is reworded to allow this (see effects).
- Inputs and checksums: Numeric programs build their input from a fixed seed with the same wrapping generator in all three languages; expr_parse and word_count read one ASCII file made by the runner. Each prints one Int checksum line; a Float result is scaled and turned to Int by a rule written in the program list, so float-to-text rules never matter. The runner refuses a row whose three checksums differ.
- Input sizes: Chosen so the default Rust build runs 1 to 3 seconds and building the input takes under 5% of that (a kernel repeats its loop as often as needed), by a Rust-only calibration run, and committed before any Paka binary is timed.
- Compiler versions: clang and clang++ 21.1.8 and rustc 1.96.1, checked by the runner before each run; C++ uses the system's libstdc++, whose version the results file records. Both are already installed, so no download is needed.
- Flags per language: Paka: clang -O3 with no CPU flag, from M6-04's one constant. Rust: rustc --edition 2024 -C opt-level=3 -C codegen-units=1 -C panic=abort, single file, no crates; plus -C overflow-checks=on for the checked integer column. C++: clang++ -O3 -std=c++20 -fno-exceptions -fno-rtti -ffp-contract=off. No fast-math anywhere, no profile-guided or link-time optimization for anyone.
- Library rule: Standard library only in Rust and C++, with their default allocator: Vec and HashMap with the default hasher, std::vector and std::unordered_map, Box and std::unique_ptr for tree nodes. Numbers are 64-bit signed integers and 64-bit floats in all three, since Paka has no others. C++ keeps sieve flags in std::vector<char>, not the bit-packed std::vector<bool>. No program's checksum depends on map iteration order. The table's notes name the containers used.
- Paka sources: Written from the language reference as an agent would write them, with no compiler switch and no benchmark-only built-in. Each is a headless app whose first handler computes, prints the checksum and exits, built with plain 'paka build' (M7-03). They get the same single tuning pass as the Rust and C++ versions.
- The runner: M6-27's runner is plain Rust with the standard library only, started as 'paka-bench run'. It pins with the taskset program already on the machine and times with Rust's monotonic clock, as M4-01 does. Nothing is installed.
- Run protocol: Single thread, pinned to one fixed logical CPU with taskset (the machine has two per core; the load check keeps the other one quiet). Per benchmark: one unrecorded warm-up of each binary, then 10 timed rounds, each round running the three languages one after another. Reported: median, and spread as (slowest minus fastest) over median. A benchmark with spread above 5% is rerun once; if still noisy its row is published with a noise mark. The runner refuses to start when the one-minute load average is above 1.
- What is recorded: Raw times of every run, checksums, compiler versions, flags, CPU model, kernel, governor, frequency boost setting, the CPU pinned to and the commit of the sources, all in the committed results file. Build time and memory use are not measured.
- Expectation shown per row: Float and array rows: parity with Rust; parity with C++ only where no trap branch is left in the inner loop (line 97), otherwise 'none stated'. Wrapping integer rows: parity with both. Checked integer rows: 'behind' with no amount against default Rust and C++ (line 98), and 'none stated' against checked Rust. Pointer-heavy rows: behind by 10 to 30% (line 99).
- Cost: overflow checks: Time of the checked kernel over its wrapping twin, for sieve and collatz. No special build is needed.
- Cost: bounds checks: Time of the normal build over the measurement build without bounds checks, for saxpy_dot, matmul, stencil and sieve. Beside it stands a yes or no from M6-19's own check run on each kernel: is a trap branch left inside the inner loop.
- Cost: reference counting and handle checks: Time of the normal build over the measurement build without that family, on the shared form of expr_parse and the pool form of bfs. Reported as 'not built' while the feature does not exist.
- Cost: state copy per handler: A visual app run with no window handles 10,000 messages on a state holding a List<Int> of 1,000, 100,000 and 10,000,000 items, once with a handler that writes one small field and once with one that writes one list item. It must be a visual app: a headless program ends on a trap (line 160), so its runtime may keep no copy at all. A runtime built with the measure feature times the copy itself and prints the total; the figure is time per handler at each size.
- Cost: dev-mode slowdown: The to-do sample from M5-23, run with no window through one committed script (add 1,000 items, then tick each one), in dev and in release. A runtime built with the measure feature replays the script and times each frame, because UI examples do not run in release before M7-11. Two ratios are published: time in Paka code (update and view) and whole-frame time including layout; drawing to a real screen is not timed and the table says so. The numeric kernels are also run once in dev mode, timed like any other run, to place Paka between the 2 to 3 times and 10 to 100 times of GENESIS line 77.
- Order of events: Program list, sources, sizes, flags, protocol and the parity line are committed first. No Paka timing is looked at before that commit, and the runner refuses to publish from any other commit.
- What the table does not claim: Single-threaded code on one machine at the baseline CPU level. Nothing about hand-written vector code, threads or memory use, which the paper already puts outside its promise (line 100). The pointer-heavy rows are measured without pools, so until pools exist they do not test the cause the paper gives for that gap, handle checks (line 269).

### M6-02 · what this changes in TASKS.md

- M6 exit check: Reworded: a section 8 cost whose feature the language does not have when M6 ends is listed in the table as 'not built' with the reason, and that counts as shown. Today this is handle checks (pools are not decided) and, if M2-10 builds no shared data, reference counting.
- M6-04: The one constant is '-O3' with no CPU flag; M1-19's -O2 is replaced by it.
- M6-27: Timing is whole-process from outside with Rust's monotonic clock, pinned with taskset, 1 warm-up and 10 interleaved rounds, a 5% noise limit and a load check; it also builds the extra checked-Rust binary for integer kernels and records the machine facts. Standard library only.
- M6-28: Writes seven kernels: four float and array, sieve and collatz each in checked and wrapping form, and mix in wrapping form only; C++ has no checked column. Each kernel is also put through M6-19's inner-loop check, which here only reports yes or no.
- M6-29: Writes expr_parse, binary_trees, bfs by list index and word_count; the shared-data and pool forms exist only if those features are built, and its needs on M6-12 and M6-13 hold only for a feature that exists.
- M6-30: Is built as accepted. Its banner goes to standard error so the checksum line still compares; its families are bounds checks plus reference counts and handle checks where those exist (its needs on M6-12 and M6-13 fall away otherwise); its runs count only after a clean normal run with the same checksum.
- M6-31: Measures the state copy on a visual app run with no window, with the three state sizes and two handlers named here, and replays the committed to-do script in dev and release; copy and frame times are taken inside a runtime built with the measure feature, and both the Paka-code ratio and the whole-frame ratio are reported.
- M6-32: The table carries compiler and LLVM version in each header, the note on LLVM 22 versus 21, the raw ratio, the expectation and the pre-set word (parity, as expected, worse than expected, measured); a cost whose feature was never built reads 'not built'.
- M6-13: Stays dropped or pending with pools (M2-01.1); the handle-check figure depends on it.
- M2-10: Its ruling decides whether the reference-count and handle-check rows of the cost table can hold a number at all.

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
