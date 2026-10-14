# Car-park billing

Run a list of car-park entry and exit events, print the fee of every stay, and list the cars still
inside.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, empty lines included. The file has at most 200 lines.

An empty line, one with no characters at all, is skipped. A line of only spaces is not empty.
Every other line must be an event: two words and two fields, separated by single spaces, with no
space at the start or the end of the line:

```
in PLATE DAY TIME
out PLATE DAY TIME
```

- `PLATE` is 1 to 8 characters, each an upper-case letter `A` to `Z` or a digit `0` to `9`.
- `DAY` is 1 to 3 digits with a value from 1 to 366. Leading zeros are allowed: `007` is day 7.
- `TIME` is two digits, a `:` and two digits: the hour from `00` to `23` and the minute from `00`
  to `59`.

The minute of an event is `(DAY - 1) * 1440 + hour * 60 + minute`. A line that is not empty and is
not an event written exactly this way is malformed: `in ab1 1 08:00`, `in AB1 1 8:00` and
`in AB1 0 08:00` are malformed.

## Errors

Before any event runs, the file is checked in two passes. Each error prints exactly one line,
ending with a line feed, and nothing else, and exits with status 20. `N` is a line number in
decimal.

1. Every line is checked for form. For the first malformed line: `error: line N: bad event`.
2. Only when no line is malformed, every event is checked against the event before it, the nearest
   earlier line that is not empty. For the first event whose minute is smaller than that event's
   minute: `error: line N: time goes back`. Equal minutes are allowed.

So a malformed line is reported even when an earlier event goes back in time.

## Events

The events run in file order. A car is either inside or not; at the start no car is inside.

- `in`: when the car is inside, print `line N: PLATE is already in` and change nothing. Otherwise
  the car enters at the event's minute.
- `out`: when the car is not inside, print `line N: PLATE is not in` and change nothing.
  Otherwise the car leaves, and its stay runs from the minute it entered to this event's minute.
  Print `PLATE: M min, FEE`, with `M` the length of the stay in minutes and `FEE` its fee.

A car that has left may enter again later; that is a new stay.

## Fee

Money is counted in whole cents. For a stay of `M` minutes:

1. When `M` is at most 15, the fee is `0.00`.
2. Otherwise let `D` be the number of whole days in the stay, `M / 1440` rounded down, and `R` the
   rest, `M - D * 1440`. The fee is `D` times `12.00` plus the charge for `R`.

The charge for `R` is `0.00` when `R` is 0. Otherwise let `H` be the number of started hours in
`R`: `R / 60` rounded up, so 1 to 60 minutes is 1 hour and 61 minutes is 2 hours. The charge is
`2.00` for the first hour, `1.50` for each of the second and third hours and `1.00` for each hour
from the fourth on; when that is more than `12.00`, the charge is `12.00`.

Rule 1 looks at the whole stay only: a stay of 1445 minutes has `D` = 1 and `R` = 5, and its fee is
`12.00` plus `2.00`, which is `14.00`.

## Output

Print the lines of the events as they run. Then print one line for every car still inside, in
rising ASCII order of the plates (a plate that is the start of a longer plate comes first):

```
PLATE: still in since day D HH:MM
```

`D` is the day it entered in decimal with no leading zeros, and `HH:MM` the time it entered, as
in the input. Last, print `total: FEE`, with `FEE` the sum of the fees of all stays that ended.

Money is written as its whole units in decimal with no leading zeros (`0` when there are none), a
`.`, and the cents as two digits: 1550 cents is `15.50`. Every printed line ends with a line feed.
The exit status is 0.

Example input:

```
in AB12 1 08:00
in CD34 1 08:05
out AB12 1 08:10
out CD34 2 09:30
out AB12 2 10:00
```

`CD34` stays from minute 485 to minute 2010, 1525 minutes: `D` = 1, `R` = 85, `H` = 2, so the fee
is `12.00` plus `3.50`. Output:

```
AB12: 10 min, 0.00
CD34: 1525 min, 15.50
line 5: AB12 is not in
total: 15.50
```

With at most 200 lines and 366 days, no stay is longer than 527039 minutes and no sum is more than
1000000.00.
