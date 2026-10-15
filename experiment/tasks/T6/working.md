# T6 hand working

Not part of the statement. One entry per case: the commands in order and the cells each one
changes, then the rows with their end spaces removed.

## cases/1

The example. `box 0 0 4 3`: rows 0 and 2 `+--+`, row 1 `|  |`. `hline 2 1 4` on `(2..5, 1)`:
`(2, 1)` is a space, so `-`; `(3, 1)` holds `|`, so `+`; `(4, 1)` and `(5, 1)` `-`. Row 1 is
`| -+--`. `text 1 3 hi`: row 3 ` hi`.

## cases/2

Line 2 `# frame` is skipped, line 3 is a good `box`. Line 4 `hline 1 2` has 3 words, not 4:
`error: line 4: wrong count`.

## hidden/1 (copy)

- `text 0 0 abcdef`: row 0 `abcdef  `.
- `copy 0 0 6 1 1 0`: cells 0 to 5 of row 0 go to 1 to 6, all read first: `aabcdef `. (Copying
  cell by cell from the left would give `aaaaaaa`.)
- `text -2 1 xyzw`: `x` and `y` fall at -2 and -1, off the canvas; row 1 `zw`.
- `fill 0 2 8 1 .`: row 2 `........`.
- `copy 5 0 4 2 -1 1`: source columns 5 to 8, rows 0 to 1; targets columns -1 to 2, rows 1 to 2.
  - `(5, 0)` `e` to `(-1, 1)`: target off, nothing.
  - `(6, 0)` `f` to `(0, 1)`.
  - `(7, 0)` space to `(1, 1)`: the `w` becomes a space.
  - `(8, 0)` is off the canvas: `(2, 1)` is not changed.
  - Row 1 to row 2 the same way: `(0, 2)` and `(1, 2)` get spaces, `(2, 2)` keeps `.`.

Rows: `aabcdef`, `f`, `  ......`.

## hidden/2 (marks)

- `fill 0 0 7 1 =`: row 0 `=======`.
- `hline 0 0 4`: `-` on `=` gives `-`: `----===`.
- `vline 1 0 5`: `(1, 0)` holds `-`, so `+`; rows 1 to 4 get `|` in column 1.
- `hline 0 2 7`: `(1, 2)` holds `|`, so `+`: row 2 `-+-----`.
- `vline 1 1 1`: `|` on `|` stays `|`.
- `text 3 2 ab`: row 2 `-+-ab--`.
- `vline 4 1 3`: `(4, 1)` space, `(4, 2)` `b`, `(4, 3)` space: all become `|` (a letter is not `-`
  or `+`, so no `+`).
- `hline 5 4 9`: columns 5 to 13 cut to 5 and 6 of row 4.
- `hline 0 0 2`: `(0, 0)` `-` stays `-`, `(1, 0)` `+` stays `+`.

Rows: `-+--===`, ` |  |`, `-+-a|--`, ` |  |`, ` |   --`.

## hidden/3 (boxes)

- `box 0 0 1 1`: one cell in both kinds of edge: `(0, 0)` `+`.
- `box 2 0 4 1`: row 0 is top and bottom; columns 2 and 5 are side edges: `+--+` on columns 2 to 5.
- `box 7 0 1 3`: column 7 is both side edges; rows 0 and 2 are top and bottom: `+`, `|`, `+`.
- `box 0 2 3 2`: rows 2 and 3 both top and bottom: `+-+` on each.
- `box 4 3 9 9`: columns 4 to 12, rows 3 to 11, cut to columns 4 to 8, rows 3 to 5. The right
  edge (column 12) and bottom edge (row 11) are off the canvas, so `(8, 3)` is `-`, not `+`; row 3
  `+----` from column 4; rows 4 and 5 `|` in column 4.
- `box -1 -1 3 3`: columns -1 to 1, rows -1 to 1. On the canvas: `(0, 0)` is inside the box, not
  changed (`+` stays); `(1, 0)` is in the right edge only: `|`; `(0, 1)` in the bottom edge only:
  `-`; `(1, 1)` a corner: `+`.

Rows: `+|+--+ +`, `-+     |`, `+-+    +`, `+-+ +----`, `    |`, `    |`.

## hidden/4 (text)

- `text -2 0 abcdefgh`: `a` and `b` are off; row 0 `cdefgh`.
- `text 1 1   x  `: `STRING` is `  x  ` (after the third space); row 1 `   x  `, printed `   x`.
- `text 0 2 `: empty `STRING`, nothing.
- `fill 0 2 6 2 *`: rows 2 and 3 `******`.
- `text 1 2 a  b`: row 2 `*a  b*`; the spaces write spaces.
- `text 0 3 `: nothing.
- `text 5 3  Q`: `STRING` is ` Q`: `(5, 3)` becomes a space, `Q` falls off. Row 3 `***** `, printed
  `*****`. The file has no final line feed.

## hidden/5

Line 1 `4  3` splits into 3 words (`4`, empty, `3`): `error: line 1: bad header`.

## hidden/6

Lines 2 (`#c`) and 3 (empty) are skipped; line 4 is a good `text`. Line 5 ` # note` does not start
with `#`; its first word is empty: `error: line 5: unknown command`. Line 6 (a bad number) is not
reached, and line numbers count the skipped lines.

## hidden/7

Line 3 `box 2 -0 0 3`: the right count; `-0` is not a number, found before the 0 width is looked
at: `error: line 3: bad number`.

## hidden/8

Line 2 is a good `copy`. Line 3 `fill 0 0 0 2 ##`: the numbers are good, `##` is two characters,
found before the 0 width: `error: line 3: bad char`.
