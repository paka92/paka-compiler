# Vending machine

Run a list of vending machine commands, with money in whole cents, and print the machine at the
end.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1. The file has at most 200 lines.

Every line is written with single spaces between its words and no space at the start or the end.
Line 1 is the coin box:

```
box N5 N10 N20 N50 N100 N200
```

Each `N` is 1 to 3 digits (`0` to `9`), leading zeros allowed: the number of coins of 5, 10, 20,
50, 100 and 200 cents that the box holds at the start. These six values are the only coins.

Every later line is a command, one of:

```
load SLOT PRICE COUNT
coin VALUE
select SLOT
cancel
```

- `SLOT` is a capital letter `A` to `Z` followed by a digit `1` to `9`, such as `B3`.
- `PRICE` is 1 or 2 digits, a `.`, then exactly 2 digits; its value in cents must be a multiple of
  5 and at least 5. `1.25` is 125 cents and `00.50` is 50 cents.
- `COUNT` is 1 or 2 digits with a value of at least 1; leading zeros allowed.
- `VALUE` is 1 to 3 digits, leading zeros allowed: an amount in cents. `050` is 50 cents.

## The machine

The machine has a credit, which starts at 0, and the coins inserted since the credit was last 0,
in the order they were inserted: the session coins. Slots start empty: no slot exists until it is
loaded. A slot has a price and a number of items.

The commands run in file order. Each prints at most one line, as given below.

- `load SLOT PRICE COUNT`: when the credit is not 0, prints `busy` and changes nothing. Otherwise,
  when the slot exists and its items plus `COUNT` would be more than 20, or when it does not exist
  and `COUNT` is more than 20, prints `slot full SLOT` and changes nothing. Otherwise the slot is
  made if it does not exist (with 0 items), its price becomes `PRICE` and `COUNT` is added to its
  items. A successful `load` prints nothing.
- `coin VALUE`: when `VALUE` is not one of the six coins, or when the credit plus `VALUE` would be
  more than 500, prints `rejected V`, with `V` the value written as money, and changes nothing.
  Otherwise the coin goes into the box, the credit grows by `VALUE`, the coin is added at the end
  of the session coins, and nothing is printed.
- `select SLOT`: these checks are made in this order and only the first that applies prints:
  1. the slot does not exist: `no slot SLOT`;
  2. the slot has 0 items: `sold out SLOT`;
  3. the credit is less than the price: `insert M more`, with `M` the price minus the credit,
     written as money;
  4. the change, which is the credit minus the price, cannot be paid by the rule below:
     `no change`.

  When one of them applies, nothing changes. Otherwise the sale happens: the slot loses one item,
  the change coins leave the box, the credit becomes 0, the session coins are forgotten, and the
  machine prints `sold SLOT` when the change is 0, or else
  `sold SLOT, change C: K1 K2 ...` with `C` the change and `K1 K2 ...` each coin paid, largest
  first, all written as money.
- `cancel`: when the credit is 0, prints `nothing to return`. Otherwise the session coins leave the
  box, the credit becomes 0, the session coins are forgotten, and the machine prints
  `returned C: K1 K2 ...` with `C` the credit before the cancel and `K1 K2 ...` the session coins in
  the order they were inserted, all written as money.

**Paying change.** The change is paid from the coins in the box at that moment, which include the
session coins. Take the coins from the largest value to the smallest: of each value, take as many
as the box holds without the coins taken so far going over the change. The change can be paid when
the coins taken add up to exactly the change; otherwise it cannot, even if another choice of coins
would add up.

## Output

Money is written as its whole units in decimal with no leading zeros (`0` when there are none), a
`.`, and the cents as two digits: 125 cents is `1.25` and 5 cents is `0.05`.

Print the lines of the commands, in their order. Then print three or more lines:

```
credit C
box N5 N10 N20 N50 N100 N200
SLOT: N at P
```

`C` is the credit at the end, written as money. The `box` line gives the number of coins of each
value now in the box, in decimal without leading zeros. Then one line per slot that exists, in
rising ASCII order of the slot names, with `N` its items in decimal without leading zeros and `P`
its price written as money. A slot with 0 items is printed too.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
box 0 1 1 0 2 0
load A1 0.70 2
coin 50
coin 50
select A1
select B2
```

Output:

```
sold A1, change 0.30: 0.20 0.10
no slot B2
credit 0.00
box 0 0 0 2 2 0
A1: 1 at 0.70
```

The credit is 100 and the change 30. No coin of 200, 100 or 50 fits in 30; the one 20 fits; then the
one 10 makes 30.

## Errors

Before any command runs, the whole file is checked. An error prints exactly one line, ending with a
line feed, and nothing else, and exits with status 20:

- `error: bad box` when the file has no lines or line 1 is not a box line written as above;
- otherwise `error: line N: bad command` for the first later line, with `N` its line number in
  decimal, that is not a command written exactly as above. An empty line is not a command.
