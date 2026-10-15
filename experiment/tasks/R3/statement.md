# Islands and bridges

Read a file of links between names, print the groups of linked names and the links whose loss
would split a group.

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

A line that holds no word is blank and is skipped. Every other line is a link: exactly two words,
each a name. A link joins its two names both ways; `a b` and `b a` mean the same. The same two
names may be linked by more than one line, and a name may be linked to itself. The names of the
file are all the names written on its links; there are at most 100 different names.

## Groups and bridges

Two names are in the same group when a chain of links leads from one to the other. Every name is
in exactly one group, and a name linked only to itself is a group of its own.

A link is a bridge when taking away that one line, and keeping every other line, leaves its two
names in different groups. So a link of a name to itself is never a bridge, and neither is a link
whose two names are also joined by another line.

## Output

Names are compared in ASCII order: character by character from the left, by character code, and at
the first difference the smaller code comes first. When one name is the start of the other, the
shorter one comes first.

Print, in this order:

1. `groups: G`, with `G` the number of groups in decimal;
2. one line per group: its names in ASCII order, joined by one space. The groups come in the
   ASCII order of their first names;
3. `bridges: B`, with `B` the number of bridges in decimal;
4. one line per bridge: its two names, the one first in ASCII order first, joined by one space.
   The bridges come in the ASCII order of their first names, and those with the same first name
   in the ASCII order of their second names.

Every printed line ends with a line feed. The exit status is 0. A file with no link prints
`groups: 0` and `bridges: 0`.

Example input:

```
a b
b c
c a
c d
x y
```

Output:

```
groups: 2
a b c d
x y
bridges: 2
c d
x y
```

## Errors

When a line is not blank and not a link, print exactly one line, ending with a line feed, and
nothing else, and exit with status 20:

```
error: line N: bad link
```

`N` is the line number in decimal. The lines are checked in order from line 1, and only the first
line with an error is reported.
