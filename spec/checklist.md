# Reference checklist

Every decision point of the ten M2 decisions, mapped to the line of `spec/paka.md` that states
it. The points are the numbered questions on the M2 pages of DECISIONS.md and each item of a
page's "Changed in the final record" line. A point the reference leaves out is marked "not for
the model" with the reason. Line numbers are those of `spec/paka.md` as committed with this file;
whoever edits the reference updates them.

Built-in table: all 63 rows of docs/decisions/M2-14.md are in the reference. No row was dropped,
so the fallback order of its section 1.17 was not used.

## M2-01 task classes

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-01.1 | graphs by list position; no pools | 230 |
| M2-01.2 | built-in generic types only | 40 |
| M2-01.3 | whole files, several steps | 423, 431 |
| M2-01.4 | Float with basic maths | 67, 334 |
| M2-01.5 | ASCII test inputs only | not for the model: a rule for the test inputs of the experiment, not for programs |

## M2-02 bindings, control flow, names

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-02.1 | last line is the value; `return` only leaves early | 149 |
| M2-02.2 | variants bare, unique per program | 88 |
| M2-02.3 | case carries meaning | 46 |
| M2-02.4 | bare parameters, then `;`, then labelled ones | 146 |
| M2-02.5 | `let`, `var`, no rebinding | 152, 153 |
| M2-02.6 | a value is used or discarded with `_ =` | 166 |
| M2-02.7 | a match names every variant | 163 |
| M2-02.8 | comments anywhere, all kept | 50; which construct owns a kept comment is not for the model: a rule for the tools that print code |
| M2-02 changed | `;`, a trailing comma, a comma after an arm and `else` on a new line are errors | 53, 56, 161 |
| M2-02 changed | nesting limit 64 counts bracketless forms; chains at most 1,000 links; `nesting-too-deep` | 57 |

## M2-03 primitive types and operations

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-03.1 | one Int, 64-bit signed, traps outside | 66, 191 |
| M2-03.2 | String positions are code points; one-character Strings | 201, 202 |
| M2-03.3 | division truncates toward zero | 192 |
| M2-03.4 | wrapping and bit operations are named functions | 336, 337, 338 |
| M2-03.5 | only `a..<b` | 159 |
| M2-03.6 | a Float becomes text through `fixed` only | 199, 335 |
| M2-03 changed | NaN bits cannot be observed; no canonical NaN | not for the model: a back-end rule; a program sees one NaN; NaN in `==` is line 194 |
| M2-03 changed | no `\u{hex}` escape | 195 |
| M2-03 changed | a bare `}` in a string is an error; write `\}` | 195 |
| M2-03 changed | no leading zero in a literal; `parse_int` accepts them | 78, 340 |
| M2-03 changed | an interpolation holds no `"`, `{` or `}` | 198 |
| M2-03 changed | five M1 lex cases stop holding | not for the model: test cases |

## M2-05 ownership

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-05.1 | copy in meaning | 209 |
| M2-05.2 | inout and read-only of the same data: refused, with a fix | 219 |
| M2-05.3 | fields apart, items not | 217 |
| M2-05.4 | bound names are values; the walked list is frozen | 215, 224, 227 |
| M2-05.5 | a slice is a value | 201, 216 |
| M2-05.6 | a type may contain itself | 91, 229 |
| M2-05 changed | an assignment inside a sibling operand counts as a write; the rule covers operators, literals and interpolation | 222 |
| M2-05 changed | `&` only as a whole call argument | 213 |
| M2-05 changed | String is not exempt | 221 |
| M2-05 changed | `for` takes no snapshot | 224 |

## M2-06 traps, holes, examples

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-06.1 | kind and position are meaning | 475 |
| M2-06.2 | the first trap in evaluation order | 455 |
| M2-06.3 | eight kinds | 458 |
| M2-06.4 | a hole is typed from its position, else an error | 97, 477 |
| M2-06.5 | an example is a block ending in a Bool | 480 |
| M2-06 changed | a failed comparison prints `left:` and `right:` on two lines | not for the model: output of `paka examples`, read by tools |
| M2-06 changed | the call-chain line is the same in every back end | not for the model: a back-end rule; the line itself is line 472 |
| M2-06 changed | `stack` and `memory` print the first line only | 475 |
| M2-06 changed | an untyped hole is `cannot-infer`; an example not ending in a Bool is `type-mismatch` | 97, 480 |
| M2-06 changed | example names share the one name pool | 37, 38 |
| M2-06 changed | the M1 case parse/example stays a reject | not for the model: a test case |

## M2-09 generics, interfaces, function values

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-09.1 | one closed built-in table | 180, 319 |
| M2-09.2 | `==` on every type, by structure | 182 |
| M2-09.3 | function values: named functions and variants | 302, 307 |
| M2-09.4 | function values equal when the same definition | 314 |
| M2-09.5 | order on Int, Float, String only | 183, 330 |
| M2-09.6 | `+` and `+=` join Strings | 184 |
| M2-09.7 | no automatic text | 200 |
| M2-09 changed | no user generics, interfaces or anonymous functions | 40, 307 |
| M2-09 changed | the interface hash covers every named type a signature reaches | not for the model: a rule of the incremental build |
| M2-09 changed | no app state may hold a function type | 315 |
| M2-09 changed | new code `not-a-value` | 305 |

## M2-10 shared data and pools

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-10.1 | no shared type; two uses are two copies | 210, 229 |
| M2-10.2 | pools not decided; graphs by list position | 230, 234 |
| M2-10 changed | nothing changed; the record adds samples, the accepted cost and what reopens the question | samples: 236; cost and reopening are not for the model: planning notes |

## M2-12 error handling

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-12.1 | failure is a value in the result | 253 |
| M2-12.2 | `Option` and `Result` | 72, 73 |
| M2-12.3 | bugs trap, outside data returns | 255 |
| M2-12.4 | postfix `?` | 258 |
| M2-12.5 | `unreachable()` | 265 |
| M2-12 changed | an `if` or `match` with no expected type takes the type of its first typed branch | 97 |
| M2-12 changed | `?` on an Option in a Result function is refused; write `ok_or(opt, text)?` | 261 |
| M2-12 changed | `Some`, `None`, `Ok`, `Err` are rows of the built-in table | 72, 73 |
| M2-12 changed | `unreachable-code` also fires after `unreachable()` | 170 |
| M2-12 changed | inout writes made before a `?` stay made | 263 |

## M2-14 built-in types and functions

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-14.1 | Map in key order; keys Int or String | 71, 372 |
| M2-14.2 | Map literal, index and `get` | 71, 256, 385 |
| M2-14.3 | walk `keys(m)` and `chars(text)` | 160 |
| M2-14.4 | List built-ins change in place, with `&` | 322, 358 |
| M2-14.5 | `sort_by` with a before function; one fixed stable sort | 366, 383 |
| M2-14.6 | about sixty rows, loops otherwise | 319, 390 |
| M2-14.7 | ASCII rules for case and white space | 356, 381 |
| M2-14 changed | 9 types, 4 constructors, 63 functions | 64, 319 |
| M2-14 changed | the sort is one fixed procedure | 383; the procedure itself is not for the model: a back-end rule; a correct `before` cannot tell |
| M2-14 changed | a Map key type other than Int or String is `type-mismatch` | 71 |
| M2-14 changed | `swap` checks both positions even when equal | not for the model: an edge result kept for the test cases (M2-14 section 1.17) |
| M2-14 changed | a pad width below zero changes nothing | not for the model: an edge result kept for the test cases (M2-14 section 1.17) |
| M2-14 changed | every growing row traps `memory` at the limit | 467 |
| M2-14 changed | `lines` splits on line feed minus a trailing empty piece | 347 |
| M2-14 changed | no wrong-name table is built yet | not for the model: a rule for the checker's messages |

## M2-15 headless apps

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-15.1 | a file failure is a second message, `or:` | 413, 414, 422 |
| M2-15.2 | a failure carries one of six texts | 422 |
| M2-15.3 | effects in list order, answers queued | 431, 432 |
| M2-15.4 | one String field per argument | 397, 427 |
| M2-15.5 | refused effect and failed Print are stops, not trap kinds | 439, 448 |
| M2-15.6 | `Exit` and `ExitWith(status)` | 415, 416 |
| M2-15.7 | no message left and no Exit: status 3 | 445, 448 |
| M2-15.8 | no standard input | 437 |
| M2-15 changed | the ExitWith range is 0 or 5 to 255 | 416, 444 |
| M2-15 changed | a literal status outside it is `literal-out-of-range` | 450 |
| M2-15 changed | the "ended without Exit" line names no message | 445 |
| M2-15 changed | a `then:` or `or:` function is applied when the answer reaches the front of the queue | not for the model: a run loop rule; a program sees only the order of its messages, line 432 |
| M2-15 changed | Print writes straight to standard output with no flush; the file-size signal is ignored | not for the model: a runtime rule |
| M2-15 changed | new code `bad-state` | 396 |
| M2-15 changed | nine M1 cases are re-expected | not for the model: test cases; the copies are under tests/cases-pending |

## What each record hands to the reference

| Record | Item | Line of spec/paka.md |
|---|---|---|
| M2-04 | `paka check <file> [--json]` and the field names | 490, 511 |
| M2-05 | the DFS idiom | 236 |
| M2-05 | the fix of each refusal | 93, 214, 219, 222, 224, 227 |
| M2-06 | the eight kinds; the hole rule in two lines; the example form and its four statuses | 458, 477, 480, 483 |
| M2-09 | the closed table, the one-line rules, the lambda repair | 180, 302, 310 |
| M2-10 | one line that graphs use list positions | 230 |
| M2-12 | `Option`, `Result`, where `?` may stand, "brackets claim, names ask", `unreachable()` | 72, 255, 259, 265 |
| M2-14 | the signature list, the count idiom, the sort in one sentence, the six white-space characters | 325, 381, 383, 387 |
| M2-15 | the app form, the handler shapes, the effect table, the loop in prose, the four stop lines, the usage line | 395, 406, 410, 425, 428, 439 |
| M2-15 | the range 0 or 5 to 255; no standard input; answers carry no path and arrive in order; an answer queued at Exit is dropped | 416, 432, 435, 437 |

## M5 views, elements, keys, UI examples, migrate

Every point of docs/decisions/M5-01.md to M5-07.md, mapped the same way (M5-41), to the line of
`spec/screens.md` that states it. Sections 13 to 15 live on that page with a budget of their own,
2,500 tokens, so `spec/paka.md` keeps its 8,000 and the M2 line numbers above still hold. All five
rows of the M5-04 element table and all six `ui_` functions appear in the sample at line 9, which
checks and whose example passes.

## M5-01 what a view is

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-01.1 | children in a block after the element | 77, 32 |
| M5-01.2 | child blocks hold `let`, `if`, `for`, `match`; no `var`, assignment, loops or jumps | 78, 79, 80 |
| M5-01.3 | elements only in views, only in place | 82, 83 |
| M5-01.4 | one root | 75 |
| M5-01.5 | view calls: plain call, same messages | 83, 44 |
| M5-01.6 | launch: defaults, then optional start, then the view | 66, 67 |
| M5-01.7 | closing the window ends the app | not for the model yet: no window is built; `paka run` refuses a visual app, line 69 |
| M5-01 rules | full signature; read-only parameters; the app's view takes the state | 72, 73, 74 |
| M5-01 rules | a placeholder in element place; a hole elsewhere traps | 84 |
| M5-01 rules | a trap in a view keeps the last frame; a first trap shows one error node | 85 |
| M5-01 rules | `update` may give no result; the view runs after every handler that did not trap | 66, 68 |

## M5-02 keys

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-02.1 | an element holding state needs `key:`, written last | 95, 104 |
| M5-02.2 | every child a `for` adds is keyed, through `if` and `match`; a hole is exempt | 104, 105 |
| M5-02.3 | keys differ among siblings; the id is the path | 108, 110 |
| M5-02.4 | an unkeyed step is the line number in the child block | 110, 111 |
| M5-02.5 | a key on a view call names its root; the root carries none and needs none | 106, 107 |
| M5-02.6 | keys are Int or String; `3` and `"3"` differ | 103 |
| M5-02.7 | duplicates draw one error node | 109 |
| M5-02.8 | element state kept only while matched | 111, 112 |
| M5-02 rules | the same literal key twice in one block is a check-time error | 108, 109 |
| M5-02 rules | focus, swap matching, reset dropping element state, shared cursors, nested loops | not for the model: runtime rules a program cannot read; reset is line 152 |

## M5-03 text input

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-03.1 | the app holds the text | 97, 98 |
| M5-03.2 | an edit sends the whole new text through a one-String variant | 96, 97 |
| M5-03.3 | the app wins; the cursor follows | 98; the cursor rule is not for the model: the app cannot read the cursor |
| M5-03.4 | preedit stays in the element | not for the model: composing text sends nothing a program sees |
| M5-03.5 | cursor, selection and focus belong to the runtime | 91, 99 |
| M5-03.6 | one cursor step is one code point | 119 |
| M5-03 rules | one line; arrows and Enter send nothing; nothing focused at launch | not for the model: key handling a program does not see; no `ui_` function sends those keys |

## M5-04 elements

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-04.1 | five elements | 87 to 93 |
| M5-04.2 | only table defaults may be left out; table order, `key:` last | 95, 96 |
| M5-04.3 | no look arguments; `gap:` is the only layout argument | 92, 93; the looks are not for the model: no program reads them |
| M5-04.4 | as M5-08 rewrites it: a click moves focus only to an Input; Tab also reaches Buttons | 99; Tab and a focused Button are not for the model: no `ui_` function sends Tab |
| M5-04.5 | other elements open | not for the model: a planning note |
| M5-04 rules | `Text` takes a String; `on_change:` a variant with one String field | 89, 96 |
| M5-04 rules | a negative literal gap is an error; a computed one traps | 98, 99 |
| M5-04 rules | placeholder and error node looks, click rules, drawn line breaks | not for the model: looks and pointer details; the click result is line 130 |

## M5-05 layout

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-05.1 | whole units, scaled | not for the model: no program or UI example sees a size (M5-05 section 2, last rule) |
| M5-05.2 | width down, height up; Button hugs, the rest fill | 89, 90, 91 |
| M5-05.3 | left in a Col, centred in a Row | 92, 93; the alignment is not for the model |
| M5-05.4 | text wraps to its width | 89 |
| M5-05.5 | content that does not fit sticks out | not for the model: drawing only |

## M5-06 UI examples

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-06.1 | a `Ui` changed in place; actions take `&` | 117, 118, 119, 57 |
| M5-06.2 | a path is a String of steps; a bad literal path is an error | 127, 128 |
| M5-06.3 | tree text, one line per node | 135, 136, 138 |
| M5-06.4 | the tree shows what the view passed; defaults not printed | 139; the field list in full is not for the model: the sample shows it |
| M5-06.5 | three readers; state not readable | 120, 121, 122 |
| M5-06.6 | a hole counts only if touched | 132 |
| M5-06.7 | a trap ends the example; an error node is fail | 132, 133 |
| M5-06.8 | typing goes to the focused input; none is a trap | 119, 131 |
| M5-06 rules | only in example bodies; `ui_start` takes a visual app's name; `Ui` not stored or compared | 125, 126 |
| M5-06 rules | click and read traps; effects not performed; after `Exit` an action traps | 129, 130, 131 |
| M5-06 rules | loop placeholder steps `#n.r`; escapes in keys and texts | 111; escapes are not for the model: the path printer gives them |

## M5-07 migrate and reset

| Point | Decision | Line of spec/screens.md |
|---|---|---|
| M5-07.1 | carried by the edit, never written to the source | 152, 153 |
| M5-07.2 | any change but defaults counts | 150, 151 |
| M5-07.3 | one migrate for the whole state; a type holding itself takes a reset | 157, 164, 165 |
| M5-07.4 | parameter old, result new | 162, 163 |
| M5-07.5 | refused at the edit unless a choice is carried or waiting | 151, 152 |
| M5-07.6 | a failed migrate keeps the old version | 165, 166 |
| M5-07.7 | reset is a launch again | 152 |
| M5-07 rules | an old value of a changed type is only taken apart; no effects | 163, 164 |
| M5-07 rules | edit replies, queue handling, release mode | not for the model: tool results and runtime rules; the reference lists no tool calls |
