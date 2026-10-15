# Line diff

Compare two text files line by line and print the changes in hunks with context lines.

## Arguments

The program is run with exactly three arguments: the name of the old file, the name of the new
file, and the context size `C`. Both files always exist and can be read.

`C` must be one or more decimal digits (`0` to `9`) and nothing else, with a value from 0 to 10;
leading zeros are allowed. So `-1`, `+2`, `12`, `1.0` and the empty argument are not valid.

## Input

Each file holds only the characters with codes 32 to 126 and the line feed (code 10). A file is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. So whether the last line ends with a line feed makes no difference. Each file has at most
200 lines of at most 200 characters. Two lines are equal when they hold exactly the same
characters, spaces included.

## The common table

Let the old file have lines `a1` to `an` and the new file lines `b1` to `bm`. For `i` from 0 to `n`
and `j` from 0 to `m`, `L(i, j)` is the length of the longest common subsequence of the old lines
after the first `i` and the new lines after the first `j`:

- `L(i, j) = 0` when `i = n` or `j = m`;
- otherwise `L(i, j) = L(i + 1, j + 1) + 1` when `a(i+1)` equals `b(j+1)`;
- otherwise `L(i, j)` is the larger of `L(i + 1, j)` and `L(i, j + 1)`.

## The walk

Start at `i = 0`, `j = 0` and repeat until `i = n` and `j = m`, each time adding one entry to a
list:

1. If `i < n`, `j < m` and `a(i+1)` equals `b(j+1)`: a **keep** of that line; add one to both.
2. Otherwise, if `j = m`, or if `i < n` and `L(i + 1, j)` is at least `L(i, j + 1)`: a **delete**
   of `a(i+1)`; add one to `i`.
3. Otherwise: an **insert** of `b(j+1)`; add one to `j`.

Deletes and inserts are **changes**. Each keep uses one old and one new line, a delete one old line
and an insert one new line, in order.

## Hunks

If the list has no change, print `no differences` and a line feed.

Otherwise, the changes are put in groups in list order: two changes with no other change between
them belong to the same group when at most `2 * C` keeps lie between them in the list. A hunk is
the part of the list from a group's first change to its last, plus up to `C` keeps directly before
it and up to `C` keeps directly after it (fewer where the list starts or ends first). Hunks never
overlap.

Each hunk prints a header and then one line per entry, in list order: a keep as a space and the
line, a delete as `-` and the line, an insert as `+` and the line. The header is

```
@@ -A,B +D,E @@
```

where `B` is the number of entries in the hunk that use an old line and `A` the number of the first
such old line; when `B` is 0, `A` is instead the number of old lines used before the hunk (0 if
none). `D` and `E` are the same for new lines. All numbers are decimal, lines count from 1, and
every printed line ends with a line feed. The exit status is 0.

Example: old file `one`, `two`, `three`, `four`; new file `one`, `2`, `three`, `four`; `C` is `4`.
Output:

```
@@ -1,4 +1,4 @@
 one
-two
+2
 three
 four
```

## Errors

The only error is a context argument that is not valid. It is checked first, before any file is
read. It prints exactly this line, ending with a line feed, and nothing else, and exits with
status 20:

```
error: bad context
```
