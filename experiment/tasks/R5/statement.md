# Cheapest trips

Read a file of roads between places and answer each trip question with the cheapest route.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 200 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A name is 1 to 20
characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `_` or `-`. A line that
holds no word is blank and is skipped. Every other line must hold exactly the words of one of:

```
one FROM TO COST
two FROM TO COST
trip FROM TO
```

`FROM` and `TO` are names and `COST` is 1 to 6 digits (`0` to `9`), leading zeros allowed: `007` is
7 and `0` is 0.

- `one FROM TO COST` is a road from `FROM` to `TO` that can be used only in that direction.
- `two FROM TO COST` is two roads: one from `FROM` to `TO` and one from `TO` to `FROM`, each with
  that cost.
- `trip FROM TO` asks for the best route from `FROM` to `TO`.

The places are the names written on `one` and `two` lines. Several roads may join the same two
places.

## Routes

A route from place `A` to place `B` is a sequence of roads in which the first starts at `A`, each
next one starts where the one before it ends, and the last ends at `B`. It may pass a place more
than once. Its cost is the sum of the costs of its roads. From a place to itself there is also the
route with no roads, of cost 0. A route is written as the places it visits, in order, `A` first and
`B` last: one more place than it has roads.

The best route is the one with the lowest cost. When several routes have that cost, the best is the
one of them with the fewest roads. When several of those remain, compare their places one by one
from the first: the best route is the one whose place comes first in ASCII order at the first place
where they differ. A name that is the start of a longer name comes first.

Each trip is answered with every road of the file, also the roads on lines after it.

## Output

Print one line per trip, in file order. The line starts with `FROM to TO: ` and goes on with the
first of these that applies:

1. `unknown FROM` when `FROM` is not a place;
2. `unknown TO` when `TO` is not a place;
3. `no route` when there is no route;
4. `cost C, R roads: P1 P2 ...` with `C` the cost and `R` the number of roads of the best route, in
   decimal, then its places separated by single spaces. When `R` is 1 the word is `road`.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
two home mill 4
one home bank 1
one bank mill 3
two mill park 2
one home yard 5
trip home park
trip park bank
trip park park
trip home zoo
two yard park 1
```

Output:

```
home to park: cost 6, 2 roads: home mill park
park to bank: cost 7, 3 roads: park mill home bank
park to park: cost 0, 0 roads: park
home to zoo: unknown zoo
```

From `home` to `park` three routes cost 6: `home mill park` and `home yard park` with 2 roads, and
`home bank mill park` with 3. Of the first two, `mill` comes before `yard`. The road from `yard` to
`park` is on the last line and still counts.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number in decimal.

The lines are checked in order from line 1, and only the first line with an error is reported:

- The line is not blank and does not hold exactly the words of one of the three forms above:
  `error: line N: bad line`
- It is a `one` or `two` line whose `FROM` and `TO` are the same name: `error: line N: loop road`

When every line is good but the file has no trip line, the error is `error: no trips`.
