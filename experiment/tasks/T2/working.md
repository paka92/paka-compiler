# T2 hand working

Not part of the statement. One entry per case: the fields, the column kinds, the widths, then each
line. Second check: in every table the `|` of each line and the `+` of each rule sit in the same
columns.

## cases/1 (`sales.csv`)

Header item, qty, price. Rows: apple 3 120; `pear, green` (quoted, the comma is kept) 12 95; fig 1
1050. item is text; qty and price are number columns, so there is a total line, and the first
column (text) holds `total`. Totals: qty 3 + 12 + 1 = 16; price 120 + 95 + 1050 = 1265.

Widths: item max(4, 5, 11, 3, total 5) = 11; qty max(3, 1, 2, 1, 2) = 3; price max(5, 3, 2, 4, 4) =
5. Rule: 11 dashes, `-+-`, 3 dashes, `-+-`, 5 dashes = `------------+-----+------`.

- Header: `item` + 7 spaces, `qty`, `price` (right): `item        | qty | price`.
- `apple       |   3 |   120`, `pear, green |  12 |    95`, `fig         |   1 |  1050`.
- Rule, then `total       |  16 |  1265`.

## cases/2 (`people.csv`)

Header name, city, note. Row 2: Ann, Oslo, `"said ""hi"""` = `said "hi"`. Row 3: Bob, empty, `-`.
No number column (Ann, Oslo, `said "hi"`, an empty value and `-` are not numbers), so no total.

Widths: name max(4, 3, 3) = 4; city max(4, 4, 0) = 4; note max(4, 9, 1) = 9. Rule:
`-----+------+----------`.

- `name | city | note` (note padded to 9, then the end spaces removed).
- `Ann  | Oslo | said "hi"`.
- `Bob  |      | -`: city is 4 spaces, note `-` + 8 spaces removed.

## hidden/1 (`empty.csv`)

No lines: `error: empty input`, status 20.

## hidden/2 (`q.csv`)

Line 2 `x"y,1`: the first field is unquoted and contains `"`: `error: line 2: bad quote`.

## hidden/3 (`open.csv`)

Line 2 `1,"2,3`: the second field opens a quote that is not closed before the end of the line:
`error: line 2: bad quote`. Line 3 (`4,5`, 2 fields of 3) is never reached.

## hidden/4 (`gap.csv`)

Line 3 is empty: one empty field, header has 2: `error: line 3: expected 2 fields, got 1`. Lines 1
and 2 have 2 fields.

## hidden/5 (`after.csv`)

Line 2 `"ab"c,1,2`: the closing `"` is followed by `c`. The quote error is reported, not the 3
fields: `error: line 2: bad quote`.

## hidden/6 (`n.csv`)

| column | values | kind | total | width |
|---|---|---|---|---|
| code | 007, -0, 12 (quoted) | number | 7 + 0 + 12 = 19 | max(4, 3, 2, 2, 2) = 4 |
| amount | -5, 10, -20 | number | -15 | max(6, 2, 2, 3, 3) = 6 |
| plus | 1, +5, 2 | text (`+5`) | | max(4, 1, 2, 1) = 4 |
| exp | 1e3, 2, 3 | text (`1e3`) | | max(3, 3, 1, 1) = 3 |
| big | 1, 2, 1234567890123 | text (13 digits) | | max(3, 1, 1, 13) = 13 |
| gap | 4, empty, 6 | text (empty) | | max(3, 1, 0, 1) = 3 |

The first column is a number column, so no `total` word; the other text columns are empty on the
total line. Values are printed as read: `007` and `-0` stay as written. Rule: 4, 6, 4, 3, 13, 3
dashes joined by `-+-` = `-----+--------+------+-----+---------------+----`.

- Header: `code | amount | plus | exp | big           | gap` (`big` + 10 spaces).
- ` 007 |     -5 | 1    | 1e3 | 1             | 4`
- `  -0 |     10 | +5   | 2   | 2             |` (gap empty, end spaces removed)
- `  12 |    -20 | 2    | 3   | 1234567890123 | 6`
- Rule, then `  19 |    -15 |      |     |               |`.

## hidden/7 (`head.csv`)

`id,name` with no line feed: header only, no data row, so no number column and no total. Widths 2
and 4. `id | name`, rule `---+-----`.

## hidden/8 (`scores.csv`)

Rows: `a,b` -7 9; `x"y` 3 9; c -1 9. id is text; score totals -7 + 3 - 1 = -5; n totals 27.

Widths: id max(2, 3, 3, 1, total 5) = 5, so `total` widens it; score max(5, 2, 1, 2, 2) = 5; n
max(1, 1, 1, 1, 27 has 2) = 2, so the total widens it. Rule: `------+-------+---`.

- `id    | score |  n`
- `a,b   |    -7 |  9`, `x"y   |     3 |  9`, `c     |    -1 |  9`
- Rule, then `total |    -5 | 27`.
