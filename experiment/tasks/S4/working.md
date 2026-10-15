# S4 hand working

Not part of the statement. One entry per case: the checks, then each command with the credit and
the box (counts of 5, 10, 20, 50, 100, 200) after it when they change.

## cases/1 (`vend.txt`)

Box `0 1 1 0 2 0`. `load A1 0.70 2`: A1 2 items at 70. Two `coin 50`: credit 100, box
`0 1 1 2 2 0`. `select A1`: change 30. 200, 100, 50 do not fit; one 20 (left 10); one 10 (left 0):
`sold A1, change 0.30: 0.20 0.10`, box `0 0 0 2 2 0`, credit 0, A1 1 item.
`select B2`: `no slot B2`.

## cases/2 (`snack.txt`)

`coin 100`: credit 100, box `2 2 2 2 3 2`. `coin 25`: not a coin, `rejected 0.25`. `select C4`
(price 150): `insert 0.50 more`. `cancel`: `returned 1.00: 1.00`, box back to `2 2 2 2 2 2`.
`cancel` again: credit 0, `nothing to return`.

## hidden/1 (`greedy.txt`)

Box `0 0 3 1 0 0`, A1 1 item at 40. `coin 100`: box `0 0 3 1 1 0`. `select A1`: change 60. 100 does
not fit; one 50 (left 10); 20 does not fit; no 10 or 5: left 10, `no change`, nothing changes, even
though three 20s would pay. `cancel`: `returned 1.00: 1.00`. Two `coin 20`: credit 40, box
`0 0 5 1 0 0`. `select A1`: change 0, `sold A1`. A1 has 0 items and is still printed.

## hidden/2 (`session.txt`, no last line feed)

Empty box. Coins 50, 20, 10: credit 80, box `0 1 1 1 0 0`. `select B2` (60): change 20, paid with
the 20 just inserted: `sold B2, change 0.20: 0.20`, box `0 1 0 1 0 0`.

## hidden/3 (`cap.txt`)

`000` is 0. Two `coin 200`: credit 400. Third: 600 > 500, `rejected 2.00`. `coin 100`: exactly 500,
taken. `coin 5`: 505, `rejected 0.05`. `coin 025` is 25, not a coin: `rejected 0.25`. `coin 0`:
`rejected 0.00`. Credit `5.00`, box `0 1 2 3 5 7`, no slot line.

## hidden/4 (`load.txt`)

A1 15 at 100. `load A1 1.20 6`: 21 > 20, `slot full A1`, price stays. `load A1 1.20 5`: 20 items,
price 120. `coin 50`: credit 50. `load B1 0.50 1`: credit not 0, `busy`. `select B1`: B1 was never
made, `no slot B1`. `cancel`: `returned 0.50: 0.50`. `load B1 0.50 25`: new slot but 25 > 20,
`slot full B1`. `load B1 0.50 20`: made. Box unchanged.

## hidden/5 (`order.txt`)

`00.50` is 50 and `01` is 1. `select C1` with credit 0: `insert 0.50 more`. `coin 50`, `select C1`:
change 0, `sold C1`; box `1 1 1 1 1 1` with the 50 in, so 50s = 2. `coin 100` (100s = 2),
`select C1`: 0 items, `sold out C1` comes before the change check. `select Z9`: `no slot Z9`.
`coin 100`: credit 200, 100s = 3. `select A2` (15): change 185 = 100 + 50 + 20 + 10 + 5:
`sold A2, change 1.85: 1.00 0.50 0.20 0.10 0.05`. Box `0 0 0 1 2 1`.
Slots in ASCII order A2, B1, C1.

## hidden/6 (`badbox.txt`)

Line 1 ends with a space: splitting gives an eighth, empty word. `error: bad box`, before line 2 is
looked at.

## hidden/7 (`price.txt`)

Line 2 `select A1` is a good command (no slot is checked before running). Line 3: `1.23` is 123, not
a multiple of 5: `error: line 3: bad command`. Line 4 (`0.00`, below 5) would also be bad.

## hidden/8 (`blank.txt`)

Line 4 is empty, not a command: `error: line 4: bad command`. Line 5 (`coin 2.00`) is also bad but
later.
