# Calendar

Answer date questions from a file: days between two dates, the weekday of a date, and a date a
number of working days away.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, empty lines included. The file has at most 200 lines of at most
200 characters.

An empty line, one with no characters at all, is skipped. A line of only spaces is not empty.
Every other line must be a command: a command word and its fields, separated by single spaces,
with no space at the start or the end of the line. There are four commands:

```
days DATE DATE
weekday DATE
add DATE COUNT
holiday DATE
```

- `DATE` is written `YYYY-MM-DD`: four digits, `-`, two digits, `-`, two digits.
- `COUNT` is 1 to 3 digits, with an optional `-` in front. `-0` is the same as `0`.

A date written this way is valid when the year is from 1900 to 2100, the month from 1 to 12, and
the day from 1 to the length of the month: 31 for months 1, 3, 5, 7, 8, 10 and 12, 30 for months
4, 6, 9 and 11, and for month 2, 29 in a leap year and 28 otherwise. A year is a leap year when it
divides by 4, except that a year that divides by 100 and not by 400 is not: 2000 is a leap year,
1900 and 2100 are not.

## Errors

Before anything is printed, every line is checked in file order. The first line with a problem is
reported, and on that line only the first of these two checks that fails:

1. The line is not empty and is not one of the commands written exactly as above:
   `error: line N: bad command`. For example `weekday 2023-2-28`, `add 2024-01-05 1000` and
   `Weekday 2024-01-05` are bad commands.
2. A date on the line is not valid: `error: line N: bad date`. For example `weekday 2023-02-29`
   and `weekday 2101-01-01` have a bad date.

So `days 2101-01-01 2024-1-01` is a bad command, not a bad date. `N` is the line number in
decimal. The error is the only line printed, it ends with a line feed, and the exit status is 20.

## Commands

Every date has a weekday; 1900-01-01 is a Monday. A working day is a Monday, Tuesday, Wednesday,
Thursday or Friday that is not a holiday. The holidays are the dates of all `holiday` lines in the
file, wherever they stand: a holiday line also counts for an `add` line above it. A holiday on a
Saturday or Sunday changes nothing, and the same holiday may be listed more than once.

The commands are answered in file order. Each `days`, `weekday` and `add` line prints one line;
a `holiday` line prints nothing.

- `days A B`: the number of days from `A` to `B` in decimal: 0 when they are the same date,
  negative with a `-` in front when `B` is before `A`.
- `weekday A`: the English name of the weekday of `A`, written `Monday`, `Tuesday`, `Wednesday`,
  `Thursday`, `Friday`, `Saturday` or `Sunday`.
- `add A COUNT`: start at `A`. When `COUNT` is more than 0, move `COUNT` times to the first working
  day after the current date. When `COUNT` is less than 0, move `-COUNT` times to the last working
  day before the current date. When `COUNT` is 0, the result is `A` itself, even when `A` is not a
  working day. Print the result as `YYYY-MM-DD`. When the result would be before 1900-01-01 or
  after 2100-12-31, print `line N: out of range` instead, with `N` the line number in decimal.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
holiday 2024-12-25
days 2024-02-28 2024-03-01
weekday 2024-12-25
add 2024-12-24 1
add 2024-12-21 0
```

Output:

```
2
Wednesday
2024-12-26
2024-12-21
```

The first working day after Tuesday 2024-12-24 is not 2024-12-25, a holiday, but Thursday
2024-12-26. 2024-12-21 is a Saturday.

With dates from 1900 to 2100 every day count is below 74000 in size, and no `add` takes more than
999 steps.
