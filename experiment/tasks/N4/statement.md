# Cost split

Split shared costs between members by weight, in whole cents, and report each member's total and
share.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 100 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A line that holds no word
is blank and is skipped. A name is 1 to 20 characters, each a letter `A` to `Z` or `a` to `z`, a
digit `0` to `9`, `_` or `-`. Names are compared exactly, case included.

The first line that is not blank is the members line. Its first word is `members`, and each later
word is a member: a name, then `:`, then the member's weight, which is 1 or 2 digits with a value
from 1 to 99. Leading zeros are allowed: `07` is 7. The order of the members on this line is the
member order.

Every later line that is not blank is a cost line:

```
cost NAME AMOUNT MEMBER MEMBER ...
```

`NAME` is a name; two costs may have the same name. `AMOUNT` is an optional `-`, then 1 to 8
digits, a `.` and exactly 2 digits: `12.05` is 1205 cents and `-0.50` is -50 cents. `-0.00` is 0,
and leading zeros are allowed. The words after `AMOUNT` name the members who share the cost, in any
order. With no such word, every member shares it.

## Splitting a cost

Let `A` be the cost's amount in cents and `a` its size: `A` without its sign. Let `W` be the sum of
the weights of the members who share it. For each such member with weight `w`:

- the base share is `a` times `w` divided by `W`, with any fraction dropped;
- the remainder is `a` times `w` minus the base share times `W`.

The base shares add up to `a` minus some `k`, with `k` smaller than the number of sharers. The `k`
sharers with the largest remainders each get one more cent. When remainders are equal, the member
earlier in the member order comes first. When `A` is negative, every share then gets a `-`. So the
shares always add up to `A` exactly.

## Output

An amount of `C` cents is written as a `-` when `C` is negative, then the whole units of its size
in decimal, with a `,` before each group of three digits counted from the right, then a `.` and the
remaining cents as exactly two digits. So 123456789 is `1,234,567.89`, 100000 is `1,000.00`, 99999
is `999.99`, -5 is `-0.05` and 0 is `0.00`.

First print one line per cost line, in input order:

```
NAME: MEMBER SHARE, MEMBER SHARE, ...
```

with one `MEMBER SHARE` pair for each member who shares the cost, in member order (not in the order
of the cost line), separated by a comma and a space.

Then print one line per member:

```
MEMBER TOTAL PERCENT
```

`TOTAL` is the sum of the member's shares over all costs, 0 when the member shares none. Let `G`
be the sum of the amounts of all costs. `PERCENT` is `-` when `G` is 0. Otherwise it is `TOTAL`
divided by `G`, times 100, rounded to one decimal: to the nearest multiple of 0.1, and when the
exact value is halfway between two of them, to the one whose last digit is even (so 6.25 becomes
6.2, 6.35 becomes 6.4 and -6.25 becomes -6.2). It is written as a `-` when the rounded value is below
zero, then the whole part with no leading zeros (`0` when it is below one), a `.`, the one decimal
digit and `%`. A value that rounds to zero is written `0.0%`. The whole part has no `,`.

These member lines are in order of `TOTAL`, largest first. Members with equal totals are in ASCII
order of their names: character by character from the left, by character code, and at the first
difference the smaller code comes first; when one name is the start of the other, the shorter one
comes first.

The last line is `total G`, with `G` written as an amount.

Every printed line ends with a line feed. The exit status is 0. With the limits above, every value
worked out, products included, is less than 2^53 in size, and the rounding must be done on the
exact value.

Example input:

```
members alice:3 bob:2 carol:1
cost rent 1000.00
cost taxi -10.00 carol bob
```

Output:

```
rent: alice 500.00, bob 333.33, carol 166.67
taxi: bob -6.67, carol -3.33
alice 500.00 50.5%
bob 326.66 33.0%
carol 163.34 16.5%
total 990.00
```

For rent, `a` is 100000 and `W` is 6. The base shares are 50000, 33333 and 16666, with remainders
0, 2 and 4, so carol gets the one cent left. For taxi, `a` is 1000 and `W` is 3: bob's base share
is 666 with remainder 2, carol's 333 with remainder 1, so bob gets the cent left, and both shares
get a `-`.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number in decimal, and `WORD` a word exactly as written.

The lines are checked in order from line 1, and only the first line with an error is reported. On
that line, the checks are made in the order listed and only the first that fails is reported.

On the members line:

1. The first word is not `members`: `error: line N: expected members`
2. There is no later word: `error: line N: no members`
3. The later words are checked from left to right. For each word, in this order:
   1. It does not hold exactly one `:`, or the part before the `:` is not a name:
      `error: line N: bad member WORD`
   2. The part after the `:` is not a weight: `error: line N: bad weight WORD`
   3. The name appeared earlier on the line: `error: line N: duplicate member NAME`, with `NAME`
      the name.

On a cost line:

1. The first word is not `cost`: `error: line N: unknown command WORD`, with `WORD` the first
   word. A second `members` line is such a line.
2. The line has fewer than three words: `error: line N: incomplete cost`
3. `NAME` is not a name: `error: line N: bad cost name`
4. `AMOUNT` is not an amount: `error: line N: bad amount`
5. The words after `AMOUNT` are checked from left to right. For each word, in this order:
   1. It is not the name of a member: `error: line N: unknown member WORD`
   2. It appeared earlier on the line: `error: line N: duplicate member WORD`

When every line is good: if the file holds no members line, the error is `error: no members`; if
it holds a members line but no cost line, the error is `error: no costs`.
