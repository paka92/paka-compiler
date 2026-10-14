# G3 hand working

Not part of the statement. One entry per case: the checks, then the best path with its cost
written as the tolls of the cells it visits plus 2 per `X`, and the paths it beats.

## cases/1 (`toll.txt`)

Paths of only `R` and `D` have 5 moves and visit 6 cells. `R R D R D`: 1+1+1+1+1+1 = 6;
`R R D D R`: 1+1+1+1+1+1 = 6. They first differ at move 4, `R` before `D`. `X` from (1,2) is not
allowed ((2,2) is a wall); `X` from (1,3) to (2,4) is allowed: `R R X D` costs 1+1+1+1+1 + 2 = 7.
`cost 6`, `moves 5: R2 D R D`.

## cases/2 (`diag.txt`)

`X X R`: (1,1), (2,2), (3,3), (3,4): 1+1+1+9 + 4 = 16. The corner cells are 9s, which are not walls.
Every other path visits at least one more 9 and costs at least 19. `cost 16`, `moves 3: X2 R`.

## hidden/1 (`order.txt`)

Line 2 `12` is short, but every line is checked for bad cells first: line 3 `1a3`,
`error: line 3 col 2: bad cell`.

## hidden/2 (`one.txt`, no last line feed)

One cell: start is end, no moves. `cost 7`, `moves 0`.

## hidden/3 (`walls.txt`)

Start and end are both walls: start is checked first, `error: start is a wall`.

## hidden/4 (`corner.txt`)

`R` and `D` from (1,1) hit walls; `X` needs (1,2) and (2,1) open, both walls. `error: no path`.

## hidden/5 (`tie.txt`)

`R D` and `D R`: 0+2+0 = 2 with 2 moves. `X`: 0+0 + 2 = 2 with 1 move. Same cost, fewer moves:
`cost 2`, `moves 1: X`.

## hidden/6 (`zero.txt`)

All tolls 0: every `R`/`D` path costs 0, every `X` costs 2. The 4-move paths tie; `R` first wins:
`R R D D`. `cost 0`, `moves 4: R2 D2`.

## hidden/7 (`blank.txt`)

Bytes `12\n12\n\n`: three lines, the last empty. No bad cell; line 3 has 0 cells:
`error: line 3: ragged row`.

## hidden/8 (`long.txt`, no last line feed)

Row 1 is all 0, the last column ends in 0s below. `R` x 11 then `D` x 2 costs 0; any other path
visits a 9 or makes an `X`. `cost 0`, `moves 13: R11 D2`.
