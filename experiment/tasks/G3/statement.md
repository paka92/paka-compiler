# Toll grid

Find the cheapest path across a grid of tolls, from the top-left cell to the bottom-right cell.

## Arguments

The program is run with exactly one argument: the name of the grid file, which always exists and
can be read.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. The file has at most 100 lines of at most 100 characters.

Each line is a row of the grid, top row first, and each character a cell: a digit `0` to `9` is a
cell with that toll, and `#` is a wall. Rows are numbered from 1 at the top and columns from 1 at
the left. All rows must have as many cells as row 1.

## Paths

A path starts on the start cell, row 1 column 1, and ends on the end cell, the last column of the
last row. It is a sequence of moves, each one of:

- `R`: one cell right, to the next column of the same row;
- `D`: one cell down, to the same column of the next row;
- `X`: one cell down and right, to the next column of the next row. An `X` move may only be made
  when two more cells are not walls: the cell just right of the cell the move starts from, and the
  cell just below the cell the move starts from.

A path never leaves the grid and never enters a wall. The start and end cells must not be walls.
When the start cell is the end cell, the only path is the one with no moves.

The cost of a path is the sum of the tolls of every cell it visits, the start and end cells
included, plus 2 for each `X` move.

The best path is the one with the lowest cost. When several paths have that cost, the best is the
one of them with the fewest moves. When several of those remain, compare their moves one by one
from the first, taking `R` before `D` and `D` before `X`: the best path is the one whose move is
earlier at the first place where they differ.

## Output

Print two lines:

```
cost C
moves M: RUNS
```

`C` is the cost of the best path and `M` its number of moves, both in decimal. `RUNS` writes its
moves in order as runs: a run is a longest sequence of equal moves next to each other. Each run is
written as its letter followed by its length in decimal, except that a run of length 1 is written as
the letter alone; runs are separated by single spaces. When `M` is 0 the second line is `moves 0`.

Every line ends with a line feed. The exit status is 0.

Example grid:

```
1119
9#11
9911
```

Output:

```
cost 6
moves 5: R2 D R D
```

Two paths cost 6 with 5 moves: `R R D R D` and `R R D D R`. They first differ at move 4, where `R`
comes before `D`. An `X` move from row 1 column 2 is not allowed, because row 2 column 2, just below
it, is a wall; every path with an `X` move costs at least 7.

## Errors

An error prints exactly one line, ending with a line feed, and nothing else, and exits with status
20. The checks are made in this order, and only the first error found is printed. `N` and `C` are
a line and a column number in decimal.

1. `error: empty grid` if the file has no lines or line 1 is empty.
2. `error: line N col C: bad cell` if a character is not a digit or `#`. Every line is checked,
   from line 1, before any row length is: the error names the first line holding such a character,
   and the column of the leftmost one on it.
3. `error: line N: ragged row` for the first line, from line 1, that has a different number of
   characters from line 1.
4. `error: start is a wall`, then `error: end is a wall`.
5. `error: no path` if no path exists.
