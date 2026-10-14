# P3 hand working

Not part of the statement. One entry per case: the stage that decides, the position counted
character by character, or the walk with the names in ASCII order. Second check: every column was
counted again by finding the character's offset in the line; every sorted list was checked pair by
pair against the codes `0`-`9` 48-57, `A`-`Z` 65-90, `_` 95, `a`-`z` 97-122.

## cases/1 (`app.conf`)

Stage 1: the comment line is skipped; `\x2d` has the hex digits 2 and d, value 2 * 16 + 13 = 45,
within 32 to 126, the character `-`; `\"` and `\\` are good escapes. No bad character. Stage 2: every
entry is `name = value`; the list holds two tables separated by one comma. Stage 3: no table repeats
a name.

Top names limits, server, users: l 108 < s 115 < u 117.

- `limits` is an empty table: `limits = {}`.
- server: host < port < tls (h 104 < p 112 < t 116). host's text is `web-01`. tls holds on.
- users[0] has name < role (n 110 < r 114); users[1] has name with text `b"o\b`, written back as
  `b\"o\\b`.

## cases/2 (`bad.conf`)

Stages 1 and 2 pass. In the table of `b`, the name `c` comes twice; the later one is on line 2:
`b`1 ` `2 `=`3 ` `4 `{`5 ` `6 `c`7 ` `8 `=`9 ` `10 `"`11 `2`12 `"`13 ` `14 `c`15.
`error: line 2 col 15: duplicate key c`, status 20.

## hidden/1 (`trail.conf`)

Stage 1 passes. Stage 2: line 3 `m = [ "x", ]`: `m`1 `=`3 `[`5 `"x"`7 `,`10 `]`12. After `,` a value
must follow; `]` cannot: `error: line 3 col 12: syntax error`. The duplicate `k` (stage 3) is never
reached.

## hidden/2 (`chars.conf`)

Stage 1 runs over the whole file before stage 2. Line 1 `x = }` holds only good characters (the
syntax error at `}` belongs to stage 2). Line 2: the `#` is inside the string, so it is text; the
second `#` starts a comment, so `@` there is skipped. Line 3 `z = "c" @`: `z`1 `=`3 `"`5 `c`6 `"`7
` `8 `@`9: `error: line 3 col 9: bad character`.

## hidden/3 (`hex.conf`)

Line 1: `\x7e` = 7 * 16 + 14 = 126, `\x20` = 32, `\x4a` = 4 * 16 + 10 = 74, all within 32 to 126.
Line 2 `bad = "tail\x7F"`: `b`1 `a`2 `d`3 ` `4 `=`5 ` `6 `"`7 `t`8 `a`9 `i`10 `l`11 `\`12. `7F` are
hex digits but 7 * 16 + 15 = 127 is above 126: `error: line 2 col 12: bad escape`.

## hidden/4 (`quote.conf`)

Line 2 `c = "x\"`: the string opens at col 5 (`c`1 ` `2 `=`3 ` `4 `"`5); `\"` is an escape, so the
last `"` does not close it, and the line ends: `error: line 2 col 5: unterminated string`.

## hidden/5 (`short.conf`)

Stage 1 passes. Tokens: `outer = { inner = [ "p" , { q = "r" } ]` and then the file ends. Each token
keeps a valid start (a `}` could still close the table), so the end of the file is the position.
The file has 2 lines and ends with a line feed: line 3 col 1. `error: line 3 col 1: syntax error`.

## hidden/6 (`order.conf`)

Stages pass (`B="up"` needs no spaces; the list of `n` runs over two lines). Top names and the first
code of each: `b` 98, `B` 66, `a1` 97, `a` 97, `_x` 95, `10` 49, `9` 57, `m` 109, `n` 110, `esc` 101.
Rising: `10` (49) < `9` (57) < `B` (66) < `_x` (95) < `a` < `a1` (`a` is the start of `a1`) <
`b` (98) < `esc` (101) < `m` (109) < `n` (110).

- `_x` is the empty string: `_x = ""`.
- `a` is a table with `z`: `a.z = "1"`, printed before `a1`.
- `esc`: `\x5C` = 5 * 16 + 12 = 92 = `\`; `\x22` = 34 = `"`; `q`; `\x4a` = 74 = `J`. Text `\"qJ`,
  written `\\` `\"` `q` `J`: `esc = "\\\"qJ"`.
- `m`: item 0 is a list whose item 0 is `"x"`: `m[0][0]`; item 1 is empty: `m[1] = []`; item 2 `"y"`.
- `n` has 11 items p to z: `n[0]` to `n[10]` in list order, so `n[10]` comes last, after `n[9]`.

22 lines, status 0.

## hidden/7 (`blank.conf`)

Only comments, an empty line and spaces: no token, the top table has no entries. Nothing printed,
status 0.

## hidden/8 (`dup.conf`)

Stages 1 and 2 pass. Repeated names in one table: line 3, `k` twice in the table of `y2`; line 4, `x`
again in the top table. The `id` names sit in four different tables, so they are not repeats. The
first in the file is line 3: `y`1 `2`2 ` `3 `=`4 ` `5 `{`6 ` `7 `k`8 ... `]`26 ` `27 `k`28.
`error: line 3 col 28: duplicate key k`.
