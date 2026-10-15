# N1 hand working

Not part of the statement. One entry per case: each item in cents (gross, discount, net, tax,
total) with every rounding step, then the sums. Second check: net plus tax equals total, for each
item and for the sums.

## cases/1 (`order.txt`)

Line 1 `book 2 19.99 5% 7.00%` is a good item. Line 2 `lamp 1 45.50 0% 20%`: five words, name,
quantity, price and discount good; the tax rate `20%` has no `.` and two decimals: `error: line 2:
bad tax`, status 20.

## cases/2 (`return.txt`)

- shoe: -2 x 3000 = -6000. Discount -6000 x 15 / 100 = -900. Net -5100. Tax -5100 x 550 / 10000 =
  -280.5, halfway, away from zero: -281. Total -5381.
- sock: 1 x 310 = 310. Discount 0. Net 310. Tax 310 x 550 / 10000 = 17.05, rounds to 17. Total 327.

Sums: net -4790, tax -264, total -5054. Negative: `refund 50.54`.

## hidden/1 (`price.txt`)

Line 1 is a good item. Line 2 `tea 4 2.5 0% 7.00%`: five words, name and quantity good, price
`2.5` has one digit after the `.`: `error: line 2: bad price`, status 20.

## hidden/2 (`count.txt`)

Line 1 `fig 1 1.00 0% 3.00%` is a good item. Line 2 `jam 1.5 3.00 7.00%` has 4 words; the count is
checked before the quantity `1.5`: `error: line 2: expected 5 fields, got 4`, status 20.

## hidden/3 (`blank.txt`)

An empty line and a line of two spaces: both blank, no item. `error: no items`, status 20.

## hidden/4 (`qty.txt`)

Line 1 is empty (blank). Line 2 `saw 12 8.40 5% 6.00%` is a good item. Line 3 `cup 1000 2.00 0% 5%`:
five words, name good; the quantity `1000` has four digits. The tax rate `5%` is bad too, but the
quantity is checked first: `error: line 3: bad quantity`, status 20.

## hidden/5 (`edge.txt`)

- gift: 2 x 1234 = 2468. Discount 100%: 2468. Net 0. Tax 0. Total 0.
- void: quantity `-00` is 0. Every amount is 0 and prints `0.00`.
- bolt: quantity `-06` is -6, discount `025%` is 25. Gross -6 x 99 = -594. Discount -594 x 25 / 100
  = -148.5, away from zero: -149. Net -594 + 149 = -445. Tax -445 x 150 / 10000 = -6.675, rounds
  to -7. Total -452.
- clip: -1 x 25 = -25. Discount -2.5, away from zero: -3. Net -22. Tax -22 x 40 / 10000 = -0.088,
  rounds to 0. Total -22.
- tag: 25. Discount 2.5 rounds to 3. Net 22. Tax 0.088 rounds to 0. Total 22.
- pin: -3 x 15 = -45. Discount 0. Net -45. Tax -45 x 5000 / 10000 = -22.5, away from zero: -23.
  Total -68.
- rug: 4 x 130 = 520. Discount 0. Net 520. Tax 520 x 3 / 10000 = 0.156, rounds to 0. Total 520.

Sums: net -445 - 22 + 22 - 45 + 520 = 30, tax -7 - 23 = -30, total -452 - 22 + 22 - 68 + 520 = 0.
Amounts below 100 cents print as `0.xx` with any `-` kept (`-0.30`); a total sum of 0 is not
negative: `total 0.00`.

## hidden/6 (`name.txt`)

Line 1 `tin 3 0.50 0% 2.00%` is a good item. Line 2 has five words; its name `abcdefghijklmnopqrstu`
has 21 characters, one too many. The quantity `x1` is bad too, but the name is checked first:
`error: line 2: bad name`, status 20.

## hidden/7 (`order.txt`)

Line 1 `nut 01 0.10 101% 100.00%`: five words, name, quantity `01` and price good. The discount
101 is over 100 and the tax rate `100.00%` has three digits before the `.`; the discount is checked
first: `error: line 1: bad discount`, status 20. Line 2 `zz` (one word) is never reached.

## hidden/8 (`big.txt`)

- crane 999: gross 999 x 9999999 = 9989999001. Discount 0. Net 9989999001. Tax 9989999001 x 9999
  / 10000 = 9989000001.0999, rounds to 9989000001. Total 19978999002.
- crane -999: gross -9989999001. Discount 1%: -99899990.01, rounds to -99899990. Net
  -9989999001 + 99899990 = -9890099011. Tax 0.01% (1): -989009.9011, rounds to -989010. Total
  -9891088021.

Line 3 has leading, doubled and tripled spaces; the words are the same. Line 6 has no line feed.

- mug 2: 800, discount 0, net 800, tax 800 x 1250 / 10000 = 100, total 900.
- mug -2: -800, 0, -800, -100, -900. The same name twice is allowed.
- nib 1: 1, 0, 1, tax 1 x 4999 / 10000 = 0.4999, rounds to 0, total 1.
- nib -1: -1, 0, -1, tax -0.4999 rounds to 0 and prints `0.00`, total -1.

The mug and nib lines add 0 to every sum. Sums: net 99899990, tax 9989000001 - 989010 =
9988010991, total 19978999002 - 9891088021 = 10087910981 (check 99899990 + 9988010991). The
largest product, 9989999001 x 9999, is about 9.99 x 10^13, under the stated 10^14.
