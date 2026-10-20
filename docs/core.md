# The core form

The form every back end runs. Lowering (M2-25) builds it from a checked program; the interpreter
(M2-26) and the native back ends read it. It holds no syntax and no sugar: every name is resolved,
every operator is typed, every node that can trap carries its site. The Rust types are in
[`paka/src/core_form.rs`](../paka/src/core_form.rs), one type per heading below.

It follows docs/decisions M2-03 (operations and evaluation order), M2-05 (copies, places, calls),
M2-06 (traps, holes, examples), M2-09 (function values), M2-12 (Option, Result, `?`) and M2-15
(apps and effects). Section 9 lists what this page had to settle where they are silent.

## 1. Program

A program is its file name, a type table, its records, enums, functions, examples, at most one app
and its trap-site table. Everything refers to everything else by number: a type, record, enum,
function, local or site is its position in the list that holds it.

- **Type.** `Int`, `Float`, `Bool`, `String`, `List<T>`, `Map<K, V>` (`K` is `Int` or `String`),
  `Option<T>`, `Result<T, E>`, `Effect<M>` (`M` is the app's message enum), `fn(A, B) -> R` or
  `fn(A, B)`, a record, an enum. Each type is in the table once, so two types are equal when their
  numbers are.
- **Record.** A name and its fields in declared order, each a name and a type. Defaults are gone:
  lowering writes them into each literal (section 7).
- **Enum.** A name and its variants in declared order, each a name and its fields. A variant's
  number is its position.
- **Function.** A name, a site (its name in the definition), its parameters in order, each a type
  and whether it is `inout`, a result type or none, the types of its locals, and a body block.
  Its parameters are locals 0 to n - 1; every `let`, `var`, `for` name and pattern name gets the
  next local number. A `let` value is numbered before its name, a `for` list before its name, and
  an arm's fields in field order before its block. The locals list holds the parameters first,
  then every local in numbering order, including the ones lowering adds (the `let` of a match
  subject, section 7).
- **Example.** A function with no parameters and a `Bool` result, kept in its own list so nothing
  can call it (M2-06). The driver names its frame `example NAME`. It also holds `claims`: one site
  for each operand of its final expression's `&&` chain, in written order (section 9, item 18).
- **Site.** A line and a column, both from 1, and for a hole its name. The file is the program's.

## 2. Evaluation

Evaluating a node gives a value, or no value (a node of no type), or leaves early: a `return`,
`break` or `continue`, or a trap. A trap is a status return carrying a kind and a site, never a
fault; each frame it leaves appends its function's name (M2-06 §3). The first trap in evaluation
order is the one reported, and nothing after it runs.

Order is strictly left to right (M2-03 §2.8); each node below says where its checks fall. A check
happens when its operation runs, never earlier.

**Copies.** Values never share storage. A copy is a new value equal by structure, made where
M2-05 §1.1 puts one: binding a `let` from a place; storing a place into a field, item or entry;
building a record, list, map or variant from places; returning a parameter or a part of one;
binding a `for` name or pattern name; the value of an `if`, `match` or block that ends in a place.
There is no copy when passing an argument, returning a local (a move), or binding or storing a
computed value. A back end may skip a copy where M2-05 §1.1 allows; the interpreter may always
copy. A place read of type Int, Float or Bool is fresh and needs no copy; String is not exempt.

## 3. Places

A place is a root and steps. The root is a local, or for a read only, a computed value. The steps:

| Step | On | Gives | Check, at the step's `[` |
|---|---|---|---|
| field `n` | a record | field `n` | none |
| item `[i]` | a List, a String | the item; for a String a one-code-point String | `out_of_bounds` unless `0 <= i < len` |
| key `[k]` | a Map | the entry's value | `out_of_bounds` when `k` is missing |
| slice `[a..<b]` | a List, a String | a fresh value, items `a` up to `b` excluded | `out_of_bounds` unless `0 <= a <= b <= len`; then `memory` past the cap |

A place is evaluated in two phases (M2-05 §3, Rule D): first the root when it is a computed value,
then every index operand of every step in written order (outer first, a slice's start before its
end); then the walk, one step at a time, each with its check. A read walks right after its index
operands. An assignment and an `inout` argument walk later, as their rules say. Only a place rooted
at a local is written; it holds no slice step, and no item step on a String (M2-03 §2.5).

## 4. Expressions

Every expression carries its type, or none when it gives no value.

| Node | Type | Evaluation | Traps |
|---|---|---|---|
| Int, Float, Bool literal | its own | the value | never |
| String literal | `String` | the text | never |
| interpolation `parts` | `String` | each part in order, then the join; an Int part is its decimal text, a Bool part `true` or `false` | `memory` past the size cap, at the opening quote |
| hole | from position, or none | nothing | `hole` at the `??`, always; it never gives a value |
| place read | the last step's | section 3; copies by section 2 where it is bound or stored | each step's check |
| record `R { fields }` | `R` | every field in declared order (the written order) | never |
| list `[items]` | `List<T>` | items in order | never |
| map `[k: v, ...]` | `Map<K, V>` | `k1, v1, k2, v2`, ...; a later equal key replaces the earlier value | never |
| variant `V(fields)` | its enum | fields in declared order | never |
| function value | `fn(A) -> R` | the id of a user function with no `inout` parameter, or of a user variant with fields (M2-09) | never |
| unary `-` on Int | `Int` | the operand, then the negation | `overflow` for `-MIN`, at the `-` |
| unary `-` on Float | `Float` | the operand, then the sign bit flipped | never |
| `!` | `Bool` | the operand, then the negation | never |
| Int `+ - * / %` | `Int` | left, right, then the operation (M2-03 §2.1) | `overflow` at the operator for `+ - *` and `/` (`MIN / -1`); `divide_by_zero` for `/ %` by 0 |
| Float `+ - * /` | `Float` | left, right, then the IEEE operation | never |
| String `+` | `String` | left, right, then the join | `memory` past the size cap, at the `+` |
| `==`, `!=` | `Bool` | left, right, then equality by structure (M2-09 §1.4); Float by IEEE; function values by id | `stack` on values nested too deep, at the operator |
| `< <= > >=` | `Bool` | left, right, then the order of Int, Float (IEEE) or String (code points) | never |
| `&&`, `\|\|` | `Bool` | left; right only when the left does not decide | never |
| call | the callee's result, or none | below | below |
| `if c { a } else { b }` | the blocks' type, or none without `else` (section 7: a match on an Int, String or Bool) | `c`, then only the chosen block | never |
| `match` | the arms' type, or none | below | never |

**Call.** The callee is a user function, a built-in, or a function value. A built-in is an index into
`builtins::ROWS` (paka/src/builtins.rs), chosen once by the checker. Evaluation:

1. a function value callee, then the arguments in written order, each fully; an `inout` argument
   only evaluates its place's root and index operands, in its position;
2. each `inout` place, left to right, walked with its checks;
3. for a user function, or a function value holding one, the depth check: the call that would
   make depth 10,001 traps `stack` (M2-06 §1.4);
4. the body. An `inout` parameter reads and writes the caller's place directly; every write stays
   made, also when the callee leaves by `return` or by a `?` (M2-12 §1.4).

A call through a function value is positional and has no `inout` argument; calling a variant's id
builds the variant. A built-in's own checks (M2-03, M2-14 §1.15: `out_of_bounds`,
`bad_conversion`, `overflow`, `assert` for `assert(false)` and `unreachable()`, `memory`) happen in
step 4. Every call has one site: the callee's name, or the function value's first token. It is
where all of the call's traps are reported, including `stack` for a user function that a built-in
calls back.

A call carries no type arguments. A generic row (letters T, K, V, E: M2-14 §1.2; bound by M2-09
§1.3's walk) is one routine for every item type: the row index names it, and the checker has
already made every use agree on one type. `==` and `!=` are structural on every type (M2-09 §1.4),
so the core has one equality for all of them.

**Match.** The subject is a user enum, `Option`, `Result` or `Effect`; the decision is on its
variant number. Each arm holds one or more variants and a block, and every variant is in exactly
one arm. An arm with one variant binds each field to a local or to nothing; an arm with several
binds nothing. Evaluation: the subject; then the arm holding its variant; its bound fields are
copied into their locals; then its block.

## 5. Blocks and statements

A block is its statements in order, then its value: an expression or none. It gives that value.
A block that leaves early gives none, and an `if` or `match` takes its type from the blocks that do
not leave.

| Statement | Evaluation | Traps |
|---|---|---|
| `let` / `var` local `= e` | `e`, then the binding; a copy when `e` is a place read | never |
| place `= e` | the place's index operands, then `e`, then the walk with its checks, then the store (copying a place before the old value is released). A key step as the last step adds a missing key and never traps but for `memory` (M2-14 §1.11); a missing key at any other step traps | the walk's checks; `memory` past the cap at the last key step's `[` |
| place `op= e` | the index operands, `e`, then the read with its checks, the operation with its check, then the write with its checks. `op` is Int `+ - * / %`, Float `+ - * /` or String `+` | the read's checks, then the operation's traps at the operator |
| `return e` / `return` | `e`, then leave the function; a returned place rooted at a parameter is copied, a local moves | never |
| `break`, `continue` | leave or restart the innermost loop | never |
| `for x in list { body }` | the list once; for each item from the first, `x` is a copy of it, then the body. A place is walked where it is: the checker made sure the body writes nothing overlapping it (M2-05 §1.7) | never |
| `for x in a..<b { body }` | `a`, then `b`, once; `x` takes `a` up to `b - 1`; nothing runs when `a >= b`, and `x` never overflows | never |
| `while c { body }` | `c`; while true, the body, then `c` again | never |
| expression | the expression; its value, if any, is dropped (`_ = e` is this) | its own |

## 6. Traps, node by node

Exactly these nodes trap, with these kinds, at these sites; nothing else does.

| Node | Kinds | Site |
|---|---|---|
| hole | `hole` | the `??`; the site holds the name |
| Int unary `-` | `overflow` | the `-` |
| Int `+ - *` | `overflow` | the operator |
| Int `/` | `divide_by_zero`, `overflow` | the operator |
| Int `%` | `divide_by_zero` | the operator |
| String `+`, interpolation | `memory` | the `+`; the opening quote |
| `==`, `!=` | `stack` | the operator |
| item, key and slice steps | `out_of_bounds`; `memory` for a slice, or a key added by an assignment | the step's `[` |
| `op=` | the operator's kinds | the operator |
| call | `stack` (user function, function value, or a call back from a built-in), and `stack` from a compare or copy too deep inside the row (`contains`, `find`, `append`, `list_of` item copies); a built-in's own kinds | the call's site |
| function | `stack` from a copy too deep | its name in the definition |

`stack` and `memory` give only their kind as meaning (M2-06 §1.2); their sites only need to
exist. In native code an allocation the host refuses also traps `memory`, at the node's site, or
the function's site for a node that has none (a literal, a copy) (M2-06 §1.4). A site belongs to
a node, not to a kind: Int `/` reports either kind at one site, so the kind travels with the
trap. Not traps: a `?` that passes a value up (it is a `return`), the stop
lines of M2-15 (`refused`, `failed`, `ended without Exit`, an `ExitWith` status out of range),
an endless loop.

## 7. Sugar lowering removes

Each form below becomes the core nodes on its right; evaluation order and traps stay those of the
form, so no back end sees it.

- **`e?`** becomes the match of M2-12 §1.4: on `Result`, `Ok(v)` gives `v` and `Err(x)` runs
  `return Err(x)`; on `Option`, `Some(v)` gives `v` and `None` runs `return None`. The `Ok` or
  `Some` arm comes first, so `v` is numbered before `x`. It is sugar,
  not a node, because its rewrite is one decision on a variant and a `return`, both core already.
- **A match on an Int, String or Bool** becomes a `let` of the subject and an `if` chain testing
  `==` against each arm's patterns in order (joined by `||`), the `_` arm or the last arm being the
  final `else`. Core `match` decides on variants only. The result is one `if true` without an
  `else`, its block holding the `let` and then the chain, the chain being the block's value when the
  match gives one (a match with only a `_` arm is that arm's block). That `if` takes the match's
  type, the one case where an `if` without `else` gives a value; it never fails. The `let` is the
  next local after the subject's own, before any arm's. Each `==` is at the arm's pattern.
- **`else if`** becomes an `if` inside the `else` block.
- **A record literal with fields left out** gets each default written in its place. A default is a
  constant (M2-02 §1.9), so where it stands in the order changes nothing.
- **Labelled arguments** are passed by position; labels come in declared order (M2-02 §1.1), so
  written order is declared order.
- **`var`** is `let`; the checker already refused every write to a `let`.
- **A match arm written as an assignment or an expression** becomes a block.

## 8. The app, messages and effects

The app is a name, an optional initial state, the start message, the handler and the grants
(M2-15 §1.1).

- **State.** The record literal `S {}` with every default written in, built once before the first
  message. No app, no state.
- **Messages.** The message enum is a user enum: the type of the handler's last parameter. The
  start message is a variant number of it; its fields are all `String` and are filled from the
  command-line arguments in order (M2-15 §1.4).
- **Handler.** A function `fn(s: inout S, m: Msg) -> List<Effect<Msg>>`, or `fn(m: Msg) -> ...`
  without state. It runs to its end before any effect is performed; a trap in it performs none.
- **Effects.** `Effect<Msg>` is a built-in enum of five variants, built and matched like any other:
  `Print(text)`, `ReadFile(path, then, or)`, `WriteFile(path, text, then, or)`, `Exit`,
  `ExitWith(status)`. `then` and `or` are function values, except `WriteFile`'s `then`, a message.
- **Grants.** A set of `Console`, `ReadFiles`, `WriteFiles`, tested only when an effect is about to
  be performed.
- **Run loop.** M2-15 §1.5: the run loop calls the handler at depth 1 with each message in queue
  order and performs its effects in list order. An answer's `then` or `or` is called through its
  function value when the answer reaches the front, its name being the outermost frame.

### 8.1 Views

A view lowers to a function whose result type is `View<M>` (M5-01, M5-02). Its body is a block of
element lines ending in `Root`, which gives the one node those lines add.

- **Element lines.** Each line is a `Child`: its line number among the element lines of its child
  block (counted from 0 through `if`, `match` and `for` bodies), whether a `for` holds it, its key
  if written, and its node: an `Element`, a view call or a `Placeholder`. Running it adds the node
  to the child list being built; its step is the key, else `#n.r` for a placeholder in a loop, else
  the line number.
- **Elements.** `Element` names its table row, holds its arguments in table order with each
  default written in, and its child block. A negative gap traps `assert` at the element. Siblings
  sharing a key become one error node at the first one's place (M5-02 §1.7).
- **Messages.** A node's message keeps its value and its value text form.
- **Traps.** A trap in a view ends the view; the caller keeps its message, `trap kind at place`.
- **Dev code.** A compiled view writes each node as it is made into a buffer of words in the run's
  context, through `rt.view`: an op, then its operands, a String as its byte length and its
  packed bytes. A child list opens before its children and closes into its Row, Col or root; a
  child line follows its node with its line, loop flag and key. A message's words follow its
  node, copied with the blocks they own. The runtime decodes the buffer into the same tree, its
  rules as above, and frees the messages when the tree is released.

## 9. Points the records left open

Each is the smallest reading consistent with M2-03, M2-05 and M2-06.

1. A chain of steps evaluates every index operand before the first check (M2-05 Rule D and §3),
   also for a read and for a computed root: `xs[9][f()]` runs `f()` before checking `9`. M2-03
   §2.8 is worded for one step.
2. A function value callee is evaluated before the arguments.
3. Several `inout` places in one call are walked left to right, after all arguments.
4. The site table has no kind column: one node may raise several kinds at one site.
5. `stack` from a copy too deep is reported at the function's site, the one node every copy is in.
6. `memory` from an interpolation is reported at its opening quote.
7. `stack` raised by a call that a built-in makes to a function value is reported at the built-in
   call's site.
8. String `+` and interpolation trap `memory` past the size cap (M2-06 §1.4).
9. A match on an Int, String or Bool is sugar; core `match` decides on variants only.
10. A call through a function value has no name to report at; its site is its first token.
11. `m[k] = v` on a new key traps `memory` at the last key step's `[`, the one step of an
    assignment that can add.
12. An allocation the host refuses is reported at the node's site, or the function's site for a
    literal or a copy, which have none of their own (section 6).
13. A copy or a compare counts its own nesting, apart from the call depth; past 10,000 levels it
    traps `stack`.
14. See M2-15 §1.4.
15. `to_int` shows its operand in the value text form of M2-14 §1.16:
    `to_int(1000000000000000000000000000000.0)`.
16. The 256-byte cut applies to the detail text without its two-space indent, at the last char
    boundary at or before byte 256.
17. Expression nesting never traps on its own; the interpreter continues deep evaluation on a
    fresh stack segment, so only the call depth limit gives `stack`.
18. M2-06 §1.6 reports a failed example at the operand of the final `&&` chain that was `false`:
    the operator for a comparison, else the operand's first token. A chain is read through both
    sides of `&&`, so `a && b && c` has three operands whichever way it nests. The sites are
    `claims` on the example, one per operand in written order; every other function has none. An
    example that ends early by `return` is reported at its first claim, with no help lines.

## 10. Recorded text this page changes

- docs/decisions M2-03 :267 (strings, `a + b`, `s += t`): the row said "never traps"; it traps
  `memory` past the cap (M2-06 §1.4 already says so).
- docs/decisions M2-03 :315-317 (§2.8, index and slice): the rule was written for one step, with
  the check before the next operand. It now reads: on a chain of steps, from a place or from a
  computed value, every index comes first, then the checks and reads step by step (M2-05 Rule D).
  This is a change, not a clarification: for `let y = xs[9][f()]` the old text checks `9` before
  `f()` runs, so a missing item 9 is reported first; now `f()` runs first, so what `f()` traps on
  comes first.
- docs/decisions M2-06 :29 (stack, position): adds a function value call's first token, the
  operator for a compare, the enclosing function's name for a copy.
- docs/decisions M2-06 :107-108 (§1.3): "each trap site returns its own constant" becomes: each
  trap returns its site's constant and its kind.
- docs/decisions M2-06 :128 (§1.4, host refusal): a literal or a copy reports at the enclosing
  function's name.
- docs/decisions M2-06 :268-271 (§3, trap sites): the table has no kind column; the kind travels
  with the trap, since one site may raise several kinds.
- docs/decisions M2-14 :265 (§1.15): `m[key] = v` on a new key traps `memory` at its last `[`.
- docs/decisions M2-06 §3 (copy and compare depth): "copy and compare count their depth on the same
  counter" becomes: they count their own nesting, apart from the call depth (section 9, item 13).
