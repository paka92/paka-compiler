# R5 hand working

Not part of the statement. One entry per case: the checks, then for each trip the routes that tie
or nearly tie, as places with their cost and road count.

## cases/1 (`roads.txt`)

`home` to `park`: `home mill park` 4+2 = 6 (2), `home yard park` 5+1 = 6 (2, the road on line 10),
`home bank mill park` 1+3+2 = 6 (3). Fewest roads, then `mill` < `yard`.
`park` to `bank`: the only road into `bank` is from `home`: `park mill home bank` 2+4+1 = 7 (3).
`park` to `park`: no roads, cost 0. `home` to `zoo`: `zoo` is not a place.

## cases/2 (`long.txt`)

Line 1 is a good trip line. Line 2: `1234567` has 7 digits: `error: line 2: bad line`.

## hidden/1 (`order.txt`)

`s a z t` and `s b c t`, both 3 (3). First difference is place 2: `a` < `b`, so `s a z t`, although
`c` < `z` at place 3.

## hidden/2 (`free.txt`)

`a` to `c`: `a b c` 0 (2) and `a c` 0 (1): `cost 0, 1 road: a c`. `a` to `d`: `a c d` 5 (2) beats
`a b c d` 5 (3).

## hidden/3 (`ghost.txt`)

Places are `a` and `b`. `ghost phantom`: `FROM` first, `unknown ghost`. `a ghost`: `unknown ghost`.
`x x`: `x` is on no road line, so it is not a place: `unknown x`, not cost 0.
`b a`: `cost 3, 1 road`.

## hidden/4 (`oneway.txt`, no last line feed)

`b` to `a`: the road goes `a` to `b` only: `no route`. `a` to `c`: `a b c` 1+2 = 3 (2), using the
road on line 4.

## hidden/5 (`parallel.txt`)

`p` to `q`: roads 7 (`007`) and 5: `cost 5, 1 road`. `q` to `p`: roads 7 and 9 (`000009`):
`cost 7, 1 road`.

## hidden/6 (`loop.txt`)

Line 2 `two a a 1`: `error: line 2: loop road`. Line 4 (`trip a`) is bad too, but later.

## hidden/7 (`none.txt`)

Every line is good (line 2 is blank), no trip line: `error: no trips`.

## hidden/8 (`names.txt`)

`s` to `t`: `s ab t` and `s abc t`, both 2 (2); `ab` is the start of `abc`: `s ab t`. The trip
line has extra spaces, which only separate words. `u` to `v`: `u Ab v` and `u ab v`, both 2 (2);
`A` (65) < `a` (97): `u Ab v`.
