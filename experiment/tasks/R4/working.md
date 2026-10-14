# R4 hand working

Not part of the statement. One entry per case: the checks, then each round with, for each service
still up, the number of its dependencies down at the start of the round against its threshold.

## cases/1 (`deps.txt`)

Thresholds: db none (no deps), cache 1, api 1, web 2. Round 0: db. Round 1: cache 1/1, api 1/1,
web 0/2: cache and api. Round 2: web 2/2. `up: none`.

## cases/2 (`limit.txt`)

Line 2 `b: a / 2`: one dependency, `K` = 2 > 1: `error: line 2: bad threshold`.

## hidden/1 (`late.txt`)

Line 1 names `ghost`, never defined, but names are checked only after every line passes. Line 3
`down a b` has three words: `error: line 3: bad line`.

## hidden/2 (`case.txt`)

Line 1 needs `api` and `db`, defined on lines 3 and 4: fine. Line 2 names `Web`; only `web` is
defined: `error: line 2: unknown service Web`.

## hidden/3 (`rounds.txt`)

Thresholds: b 1, c 2, d 1, e 2. Round 0: a. Round 1: b 1/1; c 0/2; d 0/1; e 0/2: b. Round 2: c 1/2
(d is not down yet); d 1/1; e 0/2: d. Round 3: c 2/2; e 1/2: c. Round 4: e 2/2. `up: none`.

## hidden/4 (`named.txt`)

`down: db` defines service `down`; `  down   down  ` is a down line naming it. Round 0: down.
Round 1: db and lone have no dependencies and never go down. `up: db lone`.

## hidden/5 (`twice.txt`)

Line 2 `y: z x z x / 9`: `z` repeats before `x` does, and duplicates are checked before the
threshold: `error: line 2: duplicate dependency z`.

## hidden/6 (`quiet.txt`)

Lines 1 and 3 are blank. Lines pass, names pass, no down line: `error: nothing down`.

## hidden/7 (`self.txt`)

ASCII order: `Zed alpha loop stuck zed`. Thresholds: loop 1, zed 1, alpha 2, stuck 2. Round 0: Zed
(named twice). Round 1: loop 1/1 (Zed); stuck 1/2 (itself is up); zed 0/1; alpha 1/2: loop.
Round 2: zed 1/1. Round 3: alpha 2/2. stuck needs itself down and stays up: `up: stuck`.

## hidden/8 (`chain.txt`, no last line feed)

Lines are in reverse order; every name is defined somewhere. `s02` needs s01; each later `sN` needs
the two before it (`/ 002` is 2). Round `r` takes down `s(r+1)` for r = 0 to 11. hub has threshold
10 of 12: at the start of round 10, s01 to s10 are down, so hub goes down with s11 in round 10.
`up: none`.
