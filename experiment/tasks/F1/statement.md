# Include resolver

Merge a text file and the files it includes into one output file.

## Arguments

The program is run with exactly two arguments: the root name, then the output name.

A name is 1 to 30 characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `.`,
`_` or `-`, and its first character is a letter or a digit. The root name is always a name. The
output name is 1 to 30 characters from the same set and `/`; it is never the name of a file that
the merge reads. A file of that name may already exist; writing replaces it.

## Files

Every file read holds only the characters with codes 32 to 126 and the line feed (code 10). It is
a sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file
has no lines. Lines are numbered from 1. A file has at most 100 lines, no line is longer than 200
characters, and the merge reads at most 30 different files.

A line whose first 8 characters are `#include` is an include line. Its valid form is exactly
`#include`, one space, and a name, with nothing before or after. Every other line is a text line:
` #include a.txt`, with a space first, and `#Include a.txt` are text lines.

## Merge

To merge a file, go through its lines in order:

- a text line is copied to the output as it is, blank lines included;
- an include line merges the named file at that point, then the merge goes on with the next line.

A file whose merge has finished is merged only once: a later include line naming it adds nothing.
A file whose merge has begun but not finished is on the chain; including it again is a cycle,
which is an error. The chain is the root, then each file included on the way down to the file
being merged, in that order.

Merge the root file, then write every copied line, each ending with a line feed, to the output
name. With no copied line, the output is an empty file. Then print one line and exit with status 0:

```
read F files, wrote K lines to OUT
```

`F` is the number of different files read, `K` the number of lines written, both in decimal, and
`OUT` the output name. When `F` is 1, `files` is written `file`; when `K` is 1, `lines` is written
`line`.

Example: run with `main.txt result.txt`, where `main.txt` holds

```
title
#include body.txt
end
```

and `body.txt` holds the one line `middle`. The file `result.txt` is written with the lines
`title`, `middle` and `end`, and the output is:

```
read 2 files, wrote 3 lines to result.txt
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, exits with status 20
and writes no file. `FILE` below is the name of the file holding the include line, as given in the
arguments or in the include line that named it, `N` is the line number in that file in decimal,
and `NAME` the name the include line holds.

1. The root file cannot be read: `error: cannot read ROOT`, with `ROOT` the root name.
2. While merging, the first include line that fails one of these checks is reported. The checks
   are made in this order, and only the first that fails is reported:
   1. The line is not of the valid form: `error: FILE line N: bad include`
   2. `NAME` is on the chain: `error: FILE line N: include cycle: CHAIN -> NAME`, with `CHAIN` the
      names of the chain from the root to `FILE`, joined by ` -> `.
   3. `NAME` has been merged already: no error, the line adds nothing.
   4. The file `NAME` cannot be read: `error: FILE line N: cannot read NAME`
3. The output cannot be written: `error: cannot write OUT`

Lines are met in merge order, so an error inside an included file is reported before any error
on a later line of the file that includes it. For example, when `book.txt` includes `part1.txt`,
which includes `part2.txt`, whose line 7 is `#include book.txt`:

```
error: part2.txt line 7: include cycle: book.txt -> part1.txt -> part2.txt -> book.txt
```
