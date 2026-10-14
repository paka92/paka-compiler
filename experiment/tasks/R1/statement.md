# Build order

Read a file of build rules and print an order in which every name can be built.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 200 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A name is 1 to 20
characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `_` or `-`.

A line that holds no word is blank and is skipped. Every other line is a rule:

- its first word is a name followed directly by one `:`, and nothing else;
- each later word is a name.

The name in the first word is the rule's target; the later names are its dependencies. The rule
`app: lib util` says that `app` needs `lib` and `util`. A rule may have no dependencies (`lib:`).
A dependency listed more than once in one rule counts once. A target may list itself: it then needs itself, is never ready, and is a cycle on
its own.

The names of the file are all the names written in it, as targets or as dependencies. A name that
is never a target has no dependencies. The file holds at most 100 different names.

## Order

Names are compared in ASCII order: character by character from the left, by character code, and at
the first difference the smaller code comes first. When one name is the start of the other, the
shorter one comes first. So `-x` comes before `9a`, `9a` before `Zed`, `Zed` before `_a`, `_a`
before `a`, and `a` before `a-b`.

The order is built one name at a time. A name is ready when it is not in the order yet and every
one of its dependencies is. At each step, the ready name that comes first in ASCII order is added
to the order; then the ready names are looked at again. When every name is in the order, print the
order, one name per line, and exit with status 0. A file with no rule prints nothing.

Example input:

```
app: lib util
util: lib
docs:
test: app
```

Output:

```
docs
lib
util
app
test
```

## Cycles

When some names are not in the order but none of them is ready, the rules hold a cycle. The names
not in the order at that point are the names left. Find one cycle this way:

1. Start at the name left that comes first in ASCII order.
2. From a name, step to the dependency of it that is still left and comes first in ASCII order.
   Every name left has at least one such dependency.
3. Stop at the first name reached a second time. The cycle is the names from its first visit up
   to, not including, its second visit, in the order they were reached.

Then print one line, `cycle: `, then the cycle's names starting at the one that comes first in
ASCII order and going on in the order of the steps, then that first name again, all joined by
` -> `. Exit with status 20. Nothing else is printed: no name of the order is printed.

Example: for the rules `a: b`, `b: c` and `c: b`, no name is ready. The steps go from `a` to `b`,
`c` and `b` again, so the cycle is `b`, `c`:

```
cycle: b -> c -> b
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number in decimal.

- Line `N` is not blank and not a rule: `error: line N: bad line`
- Line `N` is a rule whose target is already the target of an earlier line:
  `error: line N: duplicate target NAME`, with the target as written.

The lines are checked in order from line 1, and only the first line with an error is reported.
All lines are checked before any order is worked out, so a file with an error line and a cycle
reports the error line.
