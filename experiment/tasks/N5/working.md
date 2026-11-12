# N5 hand working

Not part of the statement. One entry per case: the checks, then each student's `T` and `S`, the
exact score, its printed form by halves up, the grade, the rank and the median.

## cases/1 (`grades.txt`)

The worked example of the statement. ann 8350 / 100 = 83.5, B. bob 2000 / 20 = 100, A. cy 4250 /
50 = 85, B. Ranks: bob 1, cy 2, ann 3. Median of three: 85.0.

## cases/2 (`short.txt`)

Line 1 parts, `P` 2, weights 50 + 50. Line 2 ann good. Line 3 `ben 90`: one word after the name:
`error: line 3: expected 2 scores, got 1`, status 20.

## hidden/1 (`edge.txt`)

Weights 7, 13, 80.

- tie: 7 x 60 + 13 x 61 = 1213, `S` 20: 60.65, exactly halfway, up: `60.7`. At least 60: D.
- edge: 7 x 88 + 13 x 91 = 1799, `S` 20: 89.95, halfway, up: `90.0`. 1799 < 90 x 20 = 1800: B.
- low: 0 / 100 = 0: `0.0`, F.

Ranks 1, 2, 3. Median: the middle score 60.65: `60.7`. Grades B 1, D 1, F 1.

## hidden/2 (`late.txt`)

Weights `040` = 40 and `060` = 60.

- amy: `100L` is 90, `5L` is 0: 3600 / 100 = 36.0, F.
- bo: `001` is 1, `0L` is 0: 40 / 100 = 0.4, F.
- cal: 95, 100: 3800 + 6000 = 9800 / 100 = 98.0, A.
- `parts x 85L` is a student line: hw excused, lab 75: 4500 / 60 = 75.0, C.

Order 98, 75, 36, 0.4. Median of four: (75 + 36) / 2 = 55.5.

## hidden/3 (`rank.txt`)

Weights 25, 75.

- adam: 2000 + 6750 = 8750 / 100 = 87.5. Zoe: 2375 + 6375 = 8750 / 100 = 87.5.
- ned: 88. lee: q excused, 6600 / 75 = 88. kim: r excused, 2200 / 25 = 88. top: 100.

Ranks: top 1; kim, lee, ned 2 (one higher); Zoe and adam 5 (four higher). `Z` (90) comes before
`a` (97). All 87.5 and up are B except top. Median of six: middles 88 and 88: `88.0`.

## hidden/4 (`median.txt`)

Weights 7, 13, 80. Line 2 blank. zed and abe have every part excused: `S` 0, no score, I, listed
by name: abe, zed. q: 476 + 754 + 4800 = 6030 / 100 = 60.3, D. p: 61, D. The last line has no
line feed. Median of two: (61 + 60.3) / 2 = 60.65, halfway, up: `60.7`.

## hidden/5 (`zero.txt`)

Line 1 blank. Line 2 is the parts line: `hw:50` and `lab:50` good; `exam:000` has value 0, not
from 1 to 100: `error: line 2: bad weight exam:000`. The sum check is never reached.

## hidden/6 (`case.txt`)

`A` and `a` are different names, so no duplicate. 100 + 1: `error: line 1: weights sum to 101`.

## hidden/7 (`again.txt`)

`P` 1. ann and bob good. Line 4 `ann 90 80`: the duplicate check comes before the score count:
`error: line 4: duplicate student ann`.

## hidden/8 (`score.txt`)

Line 2 `cy 100L x` good. Line 3 `dee 90l 101`: `90l` has a lower-case `l`, not a score; it is the
first word checked: `error: line 3: bad score 90l` (`101` is never reached).
