# Invoice

Work out the amounts of an invoice in whole cents, line by line, and its totals.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 100 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A line that holds no word
is blank and is skipped. Every other line is an item and must hold exactly five words, its fields,
in this order:

1. **Name**: 1 to 20 characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `_`
   or `-`. Two items may have the same name.
2. **Quantity**: an optional `-`, then 1 to 3 digits. `-0` is 0 and leading zeros are allowed:
   `007` is 7. A negative quantity is goods taken back.
3. **Price**: 1 to 5 digits, a `.`, then exactly 2 digits: the price of one unit. `12.05` is 1205
   cents. There is no sign.
4. **Discount**: 1 to 3 digits, then `%`, with a value from 0 to 100: a whole percent. `050%` is
   50%.
5. **Tax rate**: 1 or 2 digits, a `.`, exactly 2 digits, then `%`: a percent with two decimals,
   from `0.00%` to `99.99%`. `8.25%` is 825 hundredths of a percent.

## Amounts

Each item's amounts are whole cents, worked out in this order:

- gross = quantity times price;
- discount = gross times the discount percent, divided by 100, rounded;
- net = gross minus discount;
- tax = net times the tax rate in hundredths of a percent, divided by 10000, rounded;
- total = net plus tax.

Rounded means to the nearest whole cent. A value exactly halfway between two whole cents goes away
from zero: 12.5 becomes 13, and -12.5 becomes -13. In other words, a negative value is rounded as
its positive value would be, and the `-` is put back.

The invoice's sums are the sum of the items' nets, the sum of their taxes and the sum of their
totals. With the limits above, every value worked out, products before division included, is less
than 10^14 in size.

## Output

An amount of `C` cents is written as a `-` when `C` is negative, then the whole units of its size
in decimal with no leading zeros (`0` when the size is below 100 cents), a `.` and the remaining
cents as exactly two digits. So 1205 is `12.05`, -5 is `-0.05` and 0 is `0.00`.

Print one line per item, in input order:

```
NAME: gross G discount D net N tax T total L
```

Then three lines: `net` and the sum of nets, `tax` and the sum of taxes, and then `total` and the
sum of totals when that sum is 0 or more, or else `refund` and the sum of totals without its `-`.
Every printed line ends with a line feed. The exit status is 0.

Example input:

```
pen 3 1.25 10% 8.25%
cap -1 2.50 0% 10.00%
```

Output:

```
pen: gross 3.75 discount 0.38 net 3.37 tax 0.28 total 3.65
cap: gross -2.50 discount 0.00 net -2.50 tax -0.25 total -2.75
net 0.87
tax 0.03
total 0.90
```

The pen's discount is 37.5 cents, rounded to 38; its tax is 337 times 825 divided by 10000, which
is 27.8025 cents, rounded to 28.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. No item line is printed. `N` below is a line number in decimal.

The lines are checked in order from line 1, and only the first line with an error is reported.
On that line, the checks are made in this order and only the first that fails is reported:

1. The line has `F` words, and `F` is not 5: `error: line N: expected 5 fields, got F`
2. The name is not a name as above: `error: line N: bad name`
3. The quantity is not a quantity as above: `error: line N: bad quantity`
4. The price is not a price as above: `error: line N: bad price`
5. The discount is not a discount as above, its value over 100 included:
   `error: line N: bad discount`
6. The tax rate is not a tax rate as above: `error: line N: bad tax`

When every line is good but the file holds no item, the error is `error: no items`.
