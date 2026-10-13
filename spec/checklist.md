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
| M2-01.1 | graphs by list position; no pools | 222 |
| M2-01.2 | built-in generic types only | 40 |
| M2-01.3 | whole files, several steps | 409, 417 |
| M2-01.4 | Float with basic maths | 62, 323 |
| M2-01.5 | ASCII test inputs only | not for the model: a rule for the test inputs of the experiment, not for programs |

## M2-02 bindings, control flow, names

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-02.1 | last line is the value; `return` only leaves early | 144 |
| M2-02.2 | variants bare, unique per program | 83 |
| M2-02.3 | case carries meaning | 46 |
| M2-02.4 | bare parameters, then `;`, then labelled ones | 141 |
| M2-02.5 | `let`, `var`, no rebinding | 146, 147 |
| M2-02.6 | a value is used or discarded with `_ =` | 160 |
| M2-02.7 | a match names every variant | 157 |
| M2-02.8 | comments anywhere, all kept | 50; which construct owns a kept comment is not for the model: a rule for the tools that print code |

## M2-03 primitive types and operations

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-03.1 | one Int, 64-bit signed, traps outside | 61, 185 |
| M2-03.2 | String positions are code points; one-character Strings | 193, 194 |
| M2-03.3 | division truncates toward zero | 186 |
| M2-03.4 | wrapping and bit operations are named functions | 325, 326, 327 |
| M2-03.5 | only `a..<b` | 153 |
| M2-03.6 | a Float becomes text through `fixed` only | 191, 324 |
| M2-03 changed | NaN bits cannot be observed; no canonical NaN | not for the model: a back-end rule; a program sees one NaN; NaN in `==` is line 188 |
| M2-03 changed | no `\u{hex}` escape | 189 |
| M2-03 changed | a bare `}` in a string is an error; write `\}` | 189 |
| M2-03 changed | no leading zero in a literal; `parse_int` accepts them | 73, 329 |
| M2-03 changed | an interpolation holds no `"`, `{` or `}` | 190 |
| M2-03 changed | five M1 lex cases stop holding | not for the model: test cases |

## M2-05 ownership

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-05.1 | copy in meaning | 201 |
| M2-05.2 | inout and read-only of the same data: refused, with a fix | 211 |
| M2-05.3 | fields apart, items not | 209 |
| M2-05.4 | bound names are values; the walked list is frozen | 207, 216, 219 |
| M2-05.5 | a slice is a value | 193, 208 |
| M2-05.6 | a type may contain itself | 86, 221 |
| M2-05 changed | an assignment inside a sibling operand counts as a write; the rule covers operators, literals and interpolation | 214 |
| M2-05 changed | `&` only as a whole call argument | 205 |
| M2-05 changed | String is not exempt | 213 |
| M2-05 changed | `for` takes no snapshot | 216 |

## M2-06 traps, holes, examples

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-06.1 | kind and position are meaning | 461 |
| M2-06.2 | the first trap in evaluation order | 441 |
| M2-06.3 | eight kinds | 444 |
| M2-06.4 | a hole is typed from its position, else an error | 92, 463 |
| M2-06.5 | an example is a block ending in a Bool | 466 |
| M2-06 changed | a failed comparison prints `left:` and `right:` on two lines | not for the model: output of `paka examples`, read by tools |
| M2-06 changed | the call-chain line is the same in every back end | not for the model: a back-end rule; the line itself is line 458 |
| M2-06 changed | `stack` and `memory` print the first line only | 461 |
| M2-06 changed | an untyped hole is `cannot-infer`; an example not ending in a Bool is `type-mismatch` | 92, 466 |
| M2-06 changed | example names share the one name pool | 37, 38 |
| M2-06 changed | the M1 case parse/example stays a reject | not for the model: a test case |

## M2-09 generics, interfaces, function values

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-09.1 | one closed built-in table | 174, 309 |
| M2-09.2 | `==` on every type, by structure | 176 |
| M2-09.3 | function values: named functions and variants | 292, 297 |
| M2-09.4 | function values equal when the same definition | 304 |
| M2-09.5 | order on Int, Float, String only | 177, 319 |
| M2-09.6 | `+` and `+=` join Strings | 178 |
| M2-09.7 | no automatic text | 192 |
| M2-09 changed | no user generics, interfaces or anonymous functions | 40, 297 |
| M2-09 changed | the interface hash covers every named type a signature reaches | not for the model: a rule of the incremental build |
| M2-09 changed | no app state may hold a function type | 305 |
| M2-09 changed | new code `not-a-value` | 295 |

## M2-10 shared data and pools

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-10.1 | no shared type; two uses are two copies | 202, 221 |
| M2-10.2 | pools not decided; graphs by list position | 222, 225 |
| M2-10 changed | nothing changed; the record adds samples, the accepted cost and what reopens the question | samples: 227; cost and reopening are not for the model: planning notes |

## M2-12 error handling

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-12.1 | failure is a value in the result | 244 |
| M2-12.2 | `Option` and `Result` | 67, 68 |
| M2-12.3 | bugs trap, outside data returns | 246 |
| M2-12.4 | postfix `?` | 249 |
| M2-12.5 | `unreachable()` | 255 |
| M2-12 changed | an `if` or `match` with no expected type takes the type of its first typed branch | 92 |
| M2-12 changed | `?` on an Option in a Result function is refused; write `ok_or(opt, text)?` | 252 |
| M2-12 changed | `Some`, `None`, `Ok`, `Err` are rows of the built-in table | 67, 68 |
| M2-12 changed | `unreachable-code` also fires after `unreachable()` | 164 |
| M2-12 changed | inout writes made before a `?` stay made | 253 |

## M2-14 built-in types and functions

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-14.1 | Map in key order; keys Int or String | 66, 361 |
| M2-14.2 | Map literal, index and `get` | 66, 247, 374 |
| M2-14.3 | walk `keys(m)` and `chars(text)` | 154 |
| M2-14.4 | List built-ins change in place, with `&` | 311, 347 |
| M2-14.5 | `sort_by` with a before function; one fixed stable sort | 355, 372 |
| M2-14.6 | about sixty rows, loops otherwise | 309, 379 |
| M2-14.7 | ASCII rules for case and white space | 345, 370 |
| M2-14 changed | 9 types, 4 constructors, 63 functions | 59, 309 |
| M2-14 changed | the sort is one fixed procedure | 372; the procedure itself is not for the model: a back-end rule; a correct `before` cannot tell |
| M2-14 changed | a Map key type other than Int or String is `type-mismatch` | 66 |
| M2-14 changed | `swap` checks both positions even when equal | not for the model: an edge result kept for the test cases (M2-14 section 1.17) |
| M2-14 changed | a pad width below zero changes nothing | not for the model: an edge result kept for the test cases (M2-14 section 1.17) |
| M2-14 changed | every growing row traps `memory` at the limit | 453 |
| M2-14 changed | `lines` splits on line feed minus a trailing empty piece | 336 |
| M2-14 changed | no wrong-name table is built yet | not for the model: a rule for the checker's messages |

## M2-15 headless apps

| Point | Decision | Line of spec/paka.md |
|---|---|---|
| M2-15.1 | a file failure is a second message, `or:` | 402, 403, 408 |
| M2-15.2 | a failure carries one of six texts | 408 |
| M2-15.3 | effects in list order, answers queued | 417, 418 |
| M2-15.4 | one String field per argument | 386, 413 |
| M2-15.5 | refused effect and failed Print are stops, not trap kinds | 425, 434 |
| M2-15.6 | `Exit` and `ExitWith(status)` | 404, 405 |
| M2-15.7 | no message left and no Exit: status 3 | 431, 434 |
| M2-15.8 | no standard input | 423 |
| M2-15 changed | the ExitWith range is 0 or 5 to 255 | 405, 430 |
| M2-15 changed | a literal status outside it is `literal-out-of-range` | 436 |
| M2-15 changed | the "ended without Exit" line names no message | 431 |
| M2-15 changed | a `then:` or `or:` function is applied when the answer reaches the front of the queue | not for the model: a run loop rule; a program sees only the order of its messages, line 418 |
| M2-15 changed | Print writes straight to standard output with no flush; the file-size signal is ignored | not for the model: a runtime rule |
| M2-15 changed | new code `bad-state` | 385 |
| M2-15 changed | nine M1 cases are re-expected | not for the model: test cases; the copies are under tests/cases-pending |

## What each record hands to the reference

| Record | Item | Line of spec/paka.md |
|---|---|---|
| M2-04 | `paka check <file> [--json]` and the field names | 476, 496 |
| M2-05 | the DFS idiom | 227 |
| M2-05 | the fix of each refusal | 88, 206, 211, 214, 216, 219 |
| M2-06 | the eight kinds; the hole rule in two lines; the example form and its four statuses | 444, 463, 466, 469 |
| M2-09 | the closed table, the one-line rules, the lambda repair | 174, 292, 300 |
| M2-10 | one line that graphs use list positions | 222 |
| M2-12 | `Option`, `Result`, where `?` may stand, "brackets claim, names ask", `unreachable()` | 67, 246, 250, 255 |
| M2-14 | the signature list, the count idiom, the sort in one sentence, the six white-space characters | 314, 370, 372, 376 |
| M2-15 | the app form, the handler shapes, the effect table, the loop in prose, the four stop lines, the usage line | 384, 395, 399, 411, 414, 425 |
| M2-15 | the range 0 or 5 to 255; no standard input; answers carry no path and arrive in order; an answer queued at Exit is dropped | 405, 418, 421, 423 |
