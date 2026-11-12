# Falling sand

Let grains of sand fall on a grid until nothing moves or a step limit is reached.

## Arguments

The program is run with exactly two arguments: the name of the grid file, which always exists and
can be read, and the step limit `S`.

`S` must be one or more decimal digits (`0` to `9`) and nothing else, with a value from 0 to 1000;
leading zeros are allowed. So `-3`, `+7`, `5000`, `1.5` and the empty argument are not valid.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. The file has at most 100 lines of at most 100 characters.

Each line is a row of the grid, top row first, and each character a cell: `.` is empty, `#` is rock
and `o` is a grain. Rows are numbered from 1 at the top and columns from 1 at the left. All rows
must have as many cells as row 1.

## A step

A step visits every cell once: the bottom row first, then each row above it in turn, and in each
row from left to right. When a visited cell holds a grain, the grain may move one cell:

1. If it is in the bottom row, it stays.
2. Otherwise, if the cell below it is empty, it moves down.
3. Otherwise, if there is a column to its left and both the cell to its left and the cell below
   that are empty, it moves down and left.
4. Otherwise, if there is a column to its right and both the cell to its right and the cell below
   that are empty, it moves down and right.
5. Otherwise it stays.

Each move takes effect immediately: each check sees the grid as it is after every move made before
it in the same step. Rock never moves.

## Running

Steps are run one after another. Running stops after a step in which no grain moved, or after `S`
steps, whichever comes first. With `S` = 0 no step runs.

## Output

Print the grid, one row per line, then one more line:

- `steps: K, settled` if a step moved nothing, where `K` is the number of steps before it (the steps
  that moved something);
- `steps: S, limit` otherwise, with the value of `S` in decimal without leading zeros.

Every line ends with a line feed. The exit status is 0.

Example grid, with `S` = `10`:

```
....o.
....o.
......
......
......
....#.
```

Output:

```
......
......
......
......
......
...o#o
steps: 5, settled
```

In steps 1 to 3 both grains move down. In step 4 the lower grain moves down and left, then the
upper one moves down; in step 5 that grain moves down and right; step 6 moves nothing.

## Errors

An error prints exactly one line, ending with a line feed, and nothing else, and exits with status
20. The checks are made in this order, and only the first error found is printed:

1. `error: bad steps` if `S` is not valid. The file is not read.
2. `error: empty grid` if the file has no lines or line 1 is empty.
3. Then the lines in order from line 1. On line `N` (decimal):
   - `error: line N col C: bad cell` if a character is not `.`, `#` or `o`, where `C` is the column
     of the leftmost such character;
   - otherwise `error: line N: ragged row` if the line has a different number of characters from
     line 1.
