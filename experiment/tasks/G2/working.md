# G2 hand working

Not part of the statement. One entry per case: the checks, then each step with the grains in the
order they are visited, written (row, column). Second check: the grain count is the same before and
after, and the last step listed as moving nothing was re-checked grain by grain against rules 1-5.

## cases/1 (`pile.txt`, `S` = 100)

Grains (1,3), (2,3); 4 rows of 5.

1. Rows 4, 3 empty. (2,3): (3,3) empty, down. (1,3): (2,3) now empty, down. Grains (2,3), (3,3).
2. (3,3) down to (4,3). (2,3) down to (3,3).
3. (4,3) is in the bottom row. (3,3): below taken; left (3,2) and (4,2) empty: to (4,2).
4. (4,2), (4,3) bottom row. Nothing moves.

Steps 1-3 moved, step 4 did not: `steps: 3, settled`. Row 4 `.oo..`.

## cases/2 (`rain.txt`, `S` = 2)

1. Rows 4, 3, 2 hold no grain. (1,1), (1,2), (1,3) each move down. Row 2 `ooo`.
2. Rows 4, 3 hold no grain. (2,1): (3,1) empty, down. (2,2): below is rock; left (2,1) is now empty
   but (3,1) now holds a grain; right (2,3) holds a grain: stays. (2,3): (3,3) empty, down.

Step 2 moved and is step `S`: `steps: 2, limit`. Rows `...`, `.o.`, `o#o`, `...`.

## hidden/1 (`S` = `1001`)

Digits only, value 1001 > 1000: `error: bad steps`, status 20. The file (with a bad cell `x`) is
not read.

## hidden/2 (`S` = 5)

Line 1 `o...` fine, 4 cells. Line 2 `..O`: `O` (capital) at col 3 is not a cell, and the line is
also short; the bad cell comes first: `error: line 2 col 3: bad cell`.

## hidden/3 (`S` = 9)

Bytes `.o.\n...\n\n`: three lines, the last one empty. Lines 1 and 2 have 3 cells; line 3 has 0, no
bad cell: `error: line 3: ragged row`. It is not `empty grid`, which looks at line 1 only.

## hidden/4 (`S` = 0)

`0` is valid. The file is empty, no lines: `error: empty grid`.

## hidden/5 (`S` = 0)

`.o` and `..` (no last line feed) are valid. No step runs; the grain stays in the air:
`.o`, `..`, `steps: 0, limit`.

## hidden/6 (`S` = 50)

1. Row 3, row 2 hold no grain. (1,1): below rock; no left column; right (1,2) and (2,2) empty: to
   (2,2). (1,3): below rock; left (1,2) empty but (2,2) now taken; no right column: stays.
2. (2,2): (3,2) empty, down. (1,3): left (1,2) and (2,2), now empty: to (2,2).
3. (3,2) bottom row. (2,2): below taken; left (2,1) and right (2,3) are rock: stays. Nothing moves.

`steps: 2, settled`. Rows `...`, `#o#`, `.o.`.

## hidden/7 (`S` = 1000)

1. (1,2), (1,3), (1,4) each move down to row 2.
2. (2,2): (3,2) empty, down. (2,3): below rock; left (2,2) empty but (3,2) now taken; right (2,4)
   taken: stays. (2,4): (3,4) empty, down. Row 2 `..o..`, row 3 `.o#o.`.
3. Row 4 empty. (3,2): down to (4,2). (3,4): down to (4,4). (2,3): below rock; left (2,2) and (3,2),
   now empty: to (3,2).
4. (4,2), (4,4) bottom. (3,2): below taken; left (3,1) and (4,1) empty: to (4,1).
5. Row 4 all bottom; rows 3 to 1 hold no grain. Nothing moves.

`steps: 4, settled`. Row 3 `..#..`, row 4 `oo.o.`.

## hidden/8 (`S` = `002`)

`002` is valid with value 2.

1. Row 3 empty. (2,1): (3,1) empty, down. (1,2): below rock; left (1,1) and (2,1), now empty: to
   (2,1).
2. (3,1) bottom. (2,1): below taken; no left column; right (2,2) is rock: stays. Nothing moves.

Step 2 is within the limit and moved nothing: `steps: 1, settled`. Rows `..`, `o#`, `o.`.
