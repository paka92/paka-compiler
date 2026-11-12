# N4 hand working

Not part of the statement. One entry per case: the checks, then each cost with `a`, `W`, the base
shares and remainders and who gets the cents left, then the totals and the percent as an exact
fraction of tenths. Second check: the shares of each cost add up to its amount, and the totals add
up to `G`.

## cases/1 (`split.txt`)

The worked example of the statement. rent: `a` 100000, `W` 6: 50000 r0, 33333 r2, 16666 r4; one
cent left, to carol. taxi: listed `carol bob`, printed in member order bob, carol. `a` 1000, `W` 3:
bob 666 r2, carol 333 r1; the cent to bob; both negated: -667, -333.

Totals 50000, 32666, 16334; `G` 99000. Tenths: 50000 x 1000 / 99000 = 505.05 -> 505; 32666000 /
99000 = 329.96 -> 330; 16334000 / 99000 = 164.99 -> 165. `50.5%`, `33.0%`, `16.5%`.

## cases/2 (`trip.txt`)

Line 1 good members. Line 2 blank. Line 3 `cost fuel 60.00` good. Line 4 `cost hotel 210.00 ann
dave`: `ann` is a member, `dave` is not: `error: line 4: unknown member dave`, status 20.

## hidden/1 (`ties.txt`)

x: only m, `W` 1: 127. y: only n: 1873. p shares nothing: 0. `G` 2000.

- m: 127 x 1000 / 2000 = 63.5 tenths, halfway: 63 is odd, 64 even: `6.4%`.
- n: 1873000 / 2000 = 936.5, halfway: 936 is even: `93.6%`.
- p: 0: `0.0%`.

Order by total: n 18.73, m 1.27, p 0.00. `total 20.00`.

## hidden/2 (`lunch.txt`)

lunch: listed `bob amy zed`, member order zed, amy, bob. `a` 100, `W` 3: each 33 r1; one cent
left; equal remainders, so the earliest in member order: zed 34. Printed `zed 0.34, amy 0.33,
bob 0.33`.

tip: amy and bob, `W` 2: 2 x 1 / 2 = 1 r0 each, nothing left: `amy 0.01, bob 0.01`.

Totals: zed 34, amy 34, bob 34; `G` 102. All equal, so ASCII name order: amy, bob, zed. Tenths:
34000 / 102 = 333.33 -> 333: `33.3%` for each.

## hidden/3 (`refund.txt`)

Weight `01` is 1. No line feed after the last line.

refund: all share. `A` -1234567, `a` 1234567, `W` 10. big 8641969 / 10 = 864196 r9; mid 2469134 /
10 = 246913 r4; low 1234567 / 10 = 123456 r7. Base sum 1234565, two cents left: big (r9), then low
(r7). Shares 864197, 246913, 123457, all negated: `-8,641.97`, `-2,469.13`, `-1,234.57`.

fee: mid only: 100000, `1,000.00`.

Totals: big -864197, mid -246913 + 100000 = -146913, low -123457. `G` -1234567 + 100000 =
-1134567. Order, largest first: low -123457, mid -146913, big -864197.

Percent with `G` negative: total / `G` is positive for all three. low 123457000 / 1134567 = 108.81
-> 109: `10.9%`. mid 146913000 / 1134567 = 129.49 -> 129: `12.9%`. big 864197000 / 1134567 =
761.69 -> 762: `76.2%`. `total -11,345.67`.

## hidden/4 (`even.txt`)

- in: a 100000000: `1,000,000.00`.
- out: `-01000000.00` is 8 digits, -100000000: b `-1,000,000.00`.
- z: `-0.00` is 0, not negative; `a` 0, every share 0: `a 0.00, b 0.00` with no `-`.
- small: b 99999 `999.99`; back: b -99999 `-999.99`.

Totals a 100000000, b -100000000. `G` 0, so both percents are `-`. Order a, b. `total 0.00`.

## hidden/5 (`weights.txt`)

Lines 1 and 2 are blank (empty, and three spaces). Line 3 is the members line: `ann:1` good;
`ann:x` holds one `:` and `ann` is a name, but `x` is not a weight. That is checked before the
repeated name: `error: line 3: bad weight ann:x`, status 20.

## hidden/6 (`again.txt`)

Line 1 members, line 2 a good cost. Line 3 `members x:1` is a later line, so a cost line; its first
word is not `cost`: `error: line 3: unknown command members`, status 20. Line 4 (unknown member y)
comes later.

## hidden/7 (`amount.txt`)

Line 2 `cost ok 0.10 bob` good. Line 3: three or more words, `big` a name; `123456789.00` has 9
digits before the `.`: `error: line 3: bad amount`, status 20, before the unknown `ghost`.

## hidden/8 (`dup.txt`)

Line 2 after `3.00`: `bob` a member, `ann` a member, `bob` again: `error: line 2: duplicate member
bob`, status 20. `Ann` (unknown, case differs) comes later.
