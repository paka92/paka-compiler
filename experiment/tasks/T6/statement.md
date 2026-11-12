# Canvas

Draw lines, boxes and text on a character canvas from a file of commands, and print the canvas.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1. The file has at most 200 lines of at most 200 characters.

Line 1 is the header, `W H`: the canvas width and height, two numbers from 1 to 80 separated by
one space, with nothing else on the line. The canvas has `H` rows of `W` cells. The cell in column
`x` of row `y` is written `(x, y)`; the top left cell is `(0, 0)`, `x` grows to the right and `y`
grows downward. Every cell starts as a space.

Every later line is a command, except that an empty line and a line whose first character is `#`
are skipped. A command's words are the parts of the line between spaces: the line is split at
every space, so a line with `k` spaces has `k + 1` words, some of which may be empty.

A number is written `0`, or a digit from 1 to 9 followed by at most three more digits, either one
optionally with a `-` in front, except that `-0` is not a number. So numbers run from -9999 to 9999,
and `007`, `+1`, `-0` and `12345` are not numbers.

## Commands

The first word names the command. The other words are given below; `X`, `Y`, `TX`, `TY`, `LEN`,
`W` and `H` are numbers.

- `hline X Y LEN` draws the mark `-` on the cells `(X, Y)` to `(X + LEN - 1, Y)`.
- `vline X Y LEN` draws the mark `|` on the cells `(X, Y)` to `(X, Y + LEN - 1)`.
- `box X Y W H` draws the edge of the rectangle of the cells `(X, Y)` to `(X + W - 1, Y + H - 1)`.
  A cell of the rectangle is in its top or bottom edge when its row is `Y` or `Y + H - 1`, and in
  its left or right edge when its column is `X` or `X + W - 1`. A cell in both kinds of edge gets
  the mark `+`, a cell only in the top or bottom edge the mark `-`, and a cell only in the left or
  right edge the mark `|`. Cells in no edge are not changed. So `box 0 0 1 1` draws one `+`,
  `box 0 0 4 1` draws `+--+`, and `box 0 0 1 3` draws `+`, `|` and `+` from the top down. An edge
  is found from the whole rectangle, even where part of it is off the canvas.
- `text X Y STRING` writes the characters of `STRING` on the cells `(X, Y)`, `(X + 1, Y)` and so
  on, one character per cell, putting the character in the cell whatever it held. `STRING` is
  everything after the third space of the line, spaces included, and may be empty. A space in
  `STRING` writes a space.
- `fill X Y W H C` puts the character `C` in every cell of the rectangle `(X, Y)` to
  `(X + W - 1, Y + H - 1)`, whatever it held. `C` is a word of exactly one character.
- `copy X Y W H TX TY` copies the rectangle `(X, Y)` to `(X + W - 1, Y + H - 1)` so that its top
  left cell lands on `(TX, TY)`: the cell `(X + i, Y + j)` is copied to `(TX + i, TY + j)`. Every
  cell is copied as it was before the command began, so the two rectangles may overlap. A cell is
  copied only when both it and its target are on the canvas; other targets are not changed.

Drawing a mark on a cell combines it with what the cell holds:

- `+` always gives `+`.
- `-` gives `+` when the cell holds `|` or `+`, and `-` otherwise.
- `|` gives `+` when the cell holds `-` or `+`, and `|` otherwise.

Cells that are not on the canvas are left out of every command; a command may lie partly or wholly
off the canvas. Commands run in the order of the file, each on the canvas the earlier ones left.

## Output

Print the `H` rows of the canvas from the top, each as its `W` characters with the spaces at its end
removed, each followed by a line feed. A row of spaces prints as an empty line. The exit status is
0.

Example: the input

```
6 4
box 0 0 4 3
hline 2 1 4
text 1 3 hi
```

gives

```
+--+
| -+--
+--+
 hi
```

The `-` drawn on the box's `|` at `(3, 1)` gives `+`.

## Errors

When the file is wrong, print exactly one line, ending with a line feed, and nothing else, and exit
with status 20. The whole file is checked before anything is drawn.

- When the file has no lines, or line 1 is not a header as described above:
  `error: line 1: bad header`.
- Otherwise, for the first command line that is wrong, `error: line L: WHAT`, where `L` is its line
  number in decimal and `WHAT` is the first of these that applies:
  - `unknown command`: the first word is not `hline`, `vline`, `box`, `text`, `fill` or `copy`.
  - `wrong count`: the line does not have 4 words for `hline` and `vline`, 5 for `box`, 6 for
    `fill`, 7 for `copy`, or at least 4 for `text`.
  - `bad number`: a word that stands for a number is not a number.
  - `bad char`: the `C` of `fill` is not exactly one character.
  - `bad size`: a `LEN`, `W` or `H` is less than 1.
