# T4 hand working

Not part of the statement. One entry per case: the definitions in order of first definition, the
pieces of each value, then the expansion with the chain of variables being expanded. Second check:
every syntax column was counted again from the start of its line.

## cases/1 (`vars.txt`)

Line 1 is skipped (`#`). First definitions: `home` (line 2), `user` (line 3), `log` (line 4).
`user` is defined again on line 5, so its value is `bob$$`, printed in second place.

- `home`: text `/home/`, reference `user`. `user` is `bob` then `$$` = `bob$`. So `/home/bob$`.
- `user`: already expanded, `bob$`.
- `log`: `dir` is not defined, so the default `$home/log` is used: `/home/bob$/log`. Then
  `/app-`, then `${#user}`: `bob$` has 4 characters, then `.txt`.

Output as in the statement, status 0.

## cases/2 (`bad.txt`)

Line 1 is fine. Line 2 `home dir=...`: starts with the name `home`; column 5 is a space, not `=`:
`error: line 2 col 5: bad name`, status 20.

## hidden/1 (`names.txt`)

First definitions: `a`, `x`, `ab`, `a_1`, `c`; `x` again on line 6 (`second`); line 7 is skipped,
so `x` is `second`, printed in second place.

`c` = `$ab|$a_1.|$a-$a{b}|${a}b|$$x $$$x`:

- `$ab`: longest name run is `ab` (not `a`): `AB`. `|`.
- `$a_1`: run `a_1` stops at `.`: `U`. `.|`.
- `$a`: run stops at `-`: `A`. `-`.
- `$a`: run stops at `{`: `A`; `{b}` is plain text outside a default. `|`.
- `${a}`: `A`, then text `b`. `|`.
- `$$` is `$`, then `x`, space. `$$` is `$`, then `$x` is a reference: `second`.

`c=AB|U.|A-A{b}|Ab|$x $second`. Status 0.

## hidden/2 (`defaults.txt`)

Order: `full`, `d1`, `d2`, `d3`, `d4`, `empty`, `p`, `q`, `r`.

- `d1`: `empty` is defined (line 6), so it is expanded first: the empty value. Empty, so the
  default `was empty`.
- `d2`: `full` is `yes`, not empty: `yes`. The default holds `$nothing`, which is not defined, but
  the default is not used, so no error.
- `d3`: `missing` not defined: default `${also:-deep $full}`; `also` not defined: `deep yes`.
- `d4`: `[`, default empty, `]`: `[]`.
- `empty=` prints with nothing after `=`.
- `p` = `$$$$` = two `$$` pieces: `$$`.
- `q`: `${#p}` = 2, `${#empty}` = 0: `20`.
- `r`: `${#q}` = length of `20` = 2, then `:`, then `q` is `20`, not empty: `2:20`.

## hidden/3 (`ref.txt`)

Line 1 `a=$undefined` has no syntax error (the undefined name is an expansion error, and
expansion only happens when no line has a syntax error). Line 2 is empty, skipped. Line 3
`b=${x:-${Bad}`: `b`1 `=`2 `$`3 `{`4 `x`5 `:`6 `-`7 `$`8 `{`9 `B`10. After the inner `${` comes `B`,
not a lower-case letter: `bad reference`, met at column 10, before the line end where the outer
reference would be unclosed. Column of the inner `$`: `error: line 3 col 8: bad reference`.
Line 4 is not reached.

## hidden/4 (`self.txt`)

`ok` = `fine`. `a` = `${a:-x}`: `a` is defined, so it is expanded first, though the chain is `a`
already: `error: cycle a -> a`. The default is never reached, and `ok=fine` is not printed.

## hidden/5 (`chain.txt`)

Expand `top`, chain `top`. `$mid`: chain `top, mid`. `${#low}`: chain `top, mid, low`. `none` is
not defined, so the default `$back`: chain `top, mid, low, back`. `$mid`: `mid` is in the chain;
from `mid` to the end, then `mid`: `error: cycle mid -> low -> back -> mid`.

## hidden/6 (`undef.txt`)

`a` -> `$b` -> `b`: `x` not defined, default `$c` -> `c`: text `ok `, then `${#zz}`: `zz` is not
defined, and the piece is on the line of `c`: `error: undefined zz in c`.

## hidden/7 (`open.txt`)

Line 1 `a=$nothing`: no syntax error. Line 2 `b=${x:-${y}z`: `$`3 opens the outer reference,
`$`8 `{`9 `y`10 `}`11 closes the inner one, `z`12 is default text, then the line ends with the
outer reference open: `error: line 2 col 3: unclosed brace`. Line 3 is not reached.

## hidden/8 (`dollar.txt`)

No line feed at the end. Line 1 skipped, line 2 `fine=$$` fine. Line 3 `x=${y:-ab$`: `$`3 opens a
reference, default `ab`, then `$`10 is followed by the end of the line. Both `bad dollar` and
`unclosed brace` are met at the line end; `bad dollar` is reported, at the `$`:
`error: line 3 col 10: bad dollar`.
