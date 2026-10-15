# N2 hand working

Not part of the statement. One entry per case: each group's values, sorted values, mean, median,
and the sum of squared differences `Q` with the standard deviation sqrt(Q / (n - 1)). Second check:
`Q` equals the sum of the squares minus n times the mean squared. Square roots to four decimals,
each at least 0.0001 from a rounding tie.

## cases/1 (`scores.txt`)

- art: 88, 92. Mean 90, median (88 + 92) / 2 = 90. Q = 4 + 4 = 8; sqrt(8) = 2.8284, so `2.83`.
- math: 70, 85, 90, 64; sorted 64 70 85 90. Mean 309 / 4 = 77.25, median (70 + 85) / 2 = 77.5.
  Q = 24321 - 309 x 309 / 4 = 24321 - 23870.25 = 450.75; Q / 3 = 150.25; sqrt = 12.2577, `12.26`.

`art` before `math`.

## cases/2 (`temps.txt`)

- mon: -3, 2, -5; sorted -5 -3 2. Mean -6 / 3 = -2, median -3. Differences -1, 4, -3: Q = 26;
  Q / 2 = 13; sqrt = 3.6056, `3.61`.
- tue: 0. One value: mean and median `0.00`, `sd=-`.

## hidden/1 (`bad.txt`)

Line 1 `fee: 3` is good. Line 2 `cost 5 6`: the first word `cost` has no `:`: `error: line 2: bad
line`, status 20.

## hidden/2 (`value.txt`)

Line 1 `a: 1 2.5 x`: `1` is a value, `2.5` is not and comes before `x`: `error: line 1: bad value
2.5`, status 20. Line 2 (a bad line) is never reached.

## hidden/3 (`none.txt`)

An empty line, three spaces, an empty line: all blank. `error: no groups`, status 20.

## hidden/4 (`text.txt`)

The medians need numeric sorting: as text, `100` would sort before `25`.

- j: 8, 12, 100, 7; sorted 7 8 12 100. Mean 127 / 4 = 31.75, median (8 + 12) / 2 = 10.
  Q = 10257 - 4032.25 = 6224.75; Q / 3 = 2074.9167; sqrt = 45.5513, `45.55`.
- k: 9, 10, 100, 25, 3; sorted 3 9 10 25 100. Mean 147 / 5 = 29.4, median 10. Q = 10815 - 4321.8
  = 6493.2; Q / 4 = 1623.3; sqrt = 40.2902, `40.29`.

`j` before `k`, though `k` comes first in the file.

## hidden/5 (`split.txt`)

Groups gathered over lines: b: 1, 2, 2; B: no value (two lines); _x: -7 (and an empty line);
c: 0, -1, -1. Codes: `B` 66, `_` 95, `b` 98, `c` 99.

- B: empty, `n=0 mean=- median=- sd=-`.
- _x: one value, mean and median `-7.00`, `sd=-`.
- b: sorted 1 2 2. Mean 5 / 3 = 1.6667, `1.67`; median 2. Q = 9 - 25 / 3 = 2 / 3; Q / 2 = 1 / 3;
  sqrt = 0.5774, `0.58`.
- c: sorted -1 -1 0. Mean -2 / 3 = -0.6667, `-0.67`; median -1. Q = 2 - 4 / 3 = 2 / 3, so `0.58`.

## hidden/6 (`zero.txt`)

Line 1 has leading and repeated spaces; line 2 has no line feed.

- w: `-00` is 0, `0007` is 7, `-03` is -3; sorted -3 0 7. Mean 4 / 3 = 1.3333, `1.33`; median 0,
  `0.00`. Q = 58 - 16 / 3 = 158 / 3; Q / 2 = 79 / 3 = 26.3333; sqrt = 5.1316, `5.13`.
- z: -4, 4, -1, 1; sorted -4 -1 1 4. Mean 0, median (-1 + 1) / 2 = 0: both `0.00`. Q = 34;
  Q / 3 = 11.3333; sqrt = 3.3665, `3.37`.

## hidden/7 (`wide.txt`)

- big: 99999, -99999, 99999; sorted -99999 99999 99999. Mean 99999 / 3 = 33333, median 99999.
  Differences 66666, -133332, 66666: Q = 2 x 4444355556 + 17777422224 = 26666133336; Q / 2 =
  13333066668; sqrt = 115468.8991, `115468.90`.
- same: four 5s. Mean and median 5, Q = 0, `sd=0.00`.

## hidden/8 (`mixed.txt`)

Line 1 `ok: 1 2` is good. Line 2 `9:: 7 seven`: the first word `9::` is `9:` and a `:`, and `9:` is
not a name; it is checked before the value `seven`: `error: line 2: bad line`, status 20.
