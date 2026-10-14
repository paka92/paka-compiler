# Group statistics

Read whole numbers sorted into named groups and print the count, mean, median and standard
deviation of each group.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 200 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A name is 1 to 20
characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `_` or `-`. A value is an
optional `-`, then 1 to 5 digits: `-0` is 0 and leading zeros are allowed, so `007` is 7.

A line that holds no word is blank and is skipped. Every other line is a group line:

- its first word is a name followed directly by one `:`, and nothing else;
- each later word is a value.

The name in the first word is the group; the values on the line belong to it. A group may appear
on several lines, and its values are all the values of all its lines. A group line may hold no
value (`green:`), and a group whose lines hold no value is empty. The file holds at most 50
different groups and at most 200 values in all.

## Statistics

For a group with `n` values:

- the mean is the sum of the values divided by `n`;
- the median is the middle value once the values are sorted from smallest to largest when `n` is
  odd, and the mean of the two middle values when `n` is even;
- the standard deviation is the square root of Q / (n - 1), with Q the sum of the squares of each
  value's difference from the mean.

The mean and median need `n` of at least 1, and the standard deviation `n` of at least 2.

## Output

Names are compared in ASCII order: character by character from the left, by character code, and at
the first difference the smaller code comes first. When one name is the start of the other, the
shorter one comes first.

Print one line per group, in the ASCII order of the names:

```
NAME: n=N mean=M median=D sd=S
```

`N` is the number of values in decimal. `M`, `D` and `S` are the mean, median and standard
deviation rounded to two decimals: a `-` when the rounded value is below zero, then the whole part
with no leading zeros (`0` when it is below one), a `.` and exactly two digits. A value that rounds
to zero prints `0.00`, with no `-`. When a statistic needs more values than the group has, it
prints a single `-` instead: an empty group prints `mean=- median=- sd=-`, and a group with one
value prints `sd=-`.

Every exact mean, median and standard deviation is at least 0.0001 away from any value halfway
between two numbers with two decimals, so the rounding never meets a tie, and computing with
64-bit floating point numbers is precise enough.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
red: 4 8
blue: 5
red: 6
green:
```

Output:

```
blue: n=1 mean=5.00 median=5.00 sd=-
green: n=0 mean=- median=- sd=-
red: n=3 mean=6.00 median=6.00 sd=2.00
```

The values of `red` are 4, 8 and 6. Their mean is 6, and their differences from it are -2, 2 and
0, so the standard deviation is the square root of 8 divided by 2, which is 2.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number in decimal.

The lines are checked in order from line 1, and only the first line with an error is reported.
On a line that is not blank, the first word is checked first, then the later words from left to
right:

1. The first word is not a name followed directly by one `:`: `error: line N: bad line`
2. A later word is not a value: `error: line N: bad value WORD`, with `WORD` the first such word
   as written.

When every line is good but the file holds no group line, the error is `error: no groups`.
