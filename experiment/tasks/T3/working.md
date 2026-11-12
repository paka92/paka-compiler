# T3 hand working

Not part of the statement. One entry per case: the order check, the chunks of every line (L1, L2
and so on, number chunks as values), then the comparisons that decide the order. Text chunks are
shown after lowering. Second check: the output holds every input line once.

## cases/1 (`names.txt`, `up`)

- L1 `Report_2.txt`: `report_`, 2, `.txt`
- L2 `report10.txt`: `report`, 10, `.txt`
- L3 `report 2.txt`: `report `, 2, `.txt`
- L4 `REPORT2.txt`: `report`, 2, `.txt`
- L5 `report2.TXT`: `report`, 2, `.txt`
- L6 `r`: `r`
- L7 `REPORTB`: `reportb`

First chunks: `r` is the start of all others, so L6 first. `report` (L2, L4, L5) is the start of
`report `, `report_` and `reportb`, so those three come later. After `report`: space (32) < `_`
(95) < `b` (98), so L3, L1, L7. Among L2, L4, L5: values 10, 2, 2, so L4 and L5 before L2; L4 and
L5 have equal pairs (`.txt` after lowering) and three chunks each: equal, input order.

Order: L6, L4, L5, L2, L3, L1, L7. Raising instead of lowering would put L7 (`REPORTB`, `B` = 66)
before L1 (`_` = 95).

## cases/2 (`items.txt`, `down`)

- L1 `v2`: `v`, 2
- L2 `v02`: `v`, 2
- L3 `v`: `v`
- L4 `v-1`: `v-`, 1
- L5 (empty): no chunk
- L6 `V1a`: `v`, 1, `a`
- L7 `3v`: 3, `v`

Up order: L5 has no chunk, so it comes first against every line. L7 starts with a number chunk and
every other line with a text chunk: L7 next. `v` is the start of `v-`, so L4 is last. Among L1, L2,
L3, L6 (first chunk `v`): L3 runs out first; then values 1 (L6) and 2 (L1, L2); L1 and L2 are
equal. Up: L5, L7, L3, L6, L1, L2, L4.

Down turns it around, keeping L1 before L2: L4, L1, L2, L6, L3, L7, L5. The last line printed is
empty.

## hidden/1 (`in.txt`, `Up`)

The order must be exactly `up` or `down`: `Up` is not. `error: bad order`, status 20. The file's
16-digit chunk is never looked at.

## hidden/2 (`list.txt`, `down`)

L1 `x1` is fine. L2 `y0000000000000001`: the number chunk is 15 zeros and a 1, 16 digits, though
its value is 1. `error: line 2: number too long`, status 20. L3 (17 digits) is not reported.

## hidden/3 (`n.txt`, `up`)

All first chunks are `x`; second chunks: L1 100, L2 999999999999999 (15 digits, allowed), L3 10,
L4 9, L5 99 (`099`). Rising values: L4, L3, L5, L1, L2. Comparing the digits as text would put
`x099` and `x10` first.

## hidden/4 (`blank.txt`, `down`)

Lines: L1 empty, L2 one space (one text chunk ` `), L3 empty. Up: L1 and L3 (no chunk, equal) come
before L2. Down: L2, then L1 and L3 in input order. Output: a space, then two empty lines.

## hidden/5 (`sym.txt`, `up`)

One text chunk each, except L6:

- L1 `AB`: `ab`
- L2 `A_`: `a_`
- L3 `a[`: `a[`
- L4 `Ab`: `ab`
- L5 ``a` ``: ``a` ``
- L6 `A^1`: `a^`, 1

Second characters: `[` 91, `^` 94, `_` 95, `` ` `` 96, `b` 98. L1 and L4 are equal (one chunk each),
input order. Up: L3, L6, L2, L5, L1, L4.

## hidden/6 (`eq.txt`, `down`)

- L1 `a01b`: `a`, 1, `b`
- L2 `A1B`: `a`, 1, `b`
- L3 `b`: `b`
- L4 `a001b`: `a`, 1, `b`
- L5 `a1`: `a`, 1
- L6 `a2`: `a`, 2

Up: `a` before `b`, so L3 last. Values 1 before 2, so L6 after the 1s. Among L1, L2, L4, L5: L5
has fewer chunks, first; L1, L2, L4 are equal. Up: L5, L1, L2, L4, L6, L3.

Down: L3, L6, then L1, L2, L4 still in input order, then L5.

## hidden/7 (`mix.txt`, `up`)

- L1 `-5`: `-`, 5
- L2 `5`: 5
- L3 ` a`: ` a`
- L4 `a`: `a`
- L5 `10`: 10
- L6 `!`: `!`
- L7 `5a`: 5, `a`

Number first chunks (L2, L5, L7) come before text first chunks. Among them: L2 and L7 have 5, L2
runs out first; L5 has 10. Then text by first character: space 32 (L3), `!` 33 (L6), `-` 45 (L1),
`a` 97 (L4). Up: L2, L7, L5, L3, L6, L1, L4. `-5` is not a negative number.

## hidden/8 (`last.txt`, `down`)

The last line has no line feed; it is printed with one.

- L1 `Page 2 of 10`: `page `, 2, ` of `, 10
- L2 `page 2 of 9`: `page `, 2, ` of `, 9
- L3 `page 10 of 1`: `page `, 10, ` of `, 1
- L4 `Page`: `page`
- L5 `Page 2 of 09`: `page `, 2, ` of `, 9

Up: `page` is the start of `page `, so L4 first. Second chunk 2 (L1, L2, L5) before 10 (L3). Among
the 2s, fourth chunk 9 (L2, L5, equal) before 10 (L1). Up: L4, L2, L5, L1, L3.

Down: L3, L1, then L2 and L5 in input order, then L4.
