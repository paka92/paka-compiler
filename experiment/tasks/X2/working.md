# X2 hand working

Not part of the statement. One entry per case: the input, how each expected result follows from the
statement, and a second check. `\n` is a line feed.

## cases/1 (`a.txt b.txt joined.txt`)

a.txt `alpha\nbeta\n` has 2 lines, b.txt `gamma\n` has 1. K = 3, W = 1. Written: `1 | alpha`,
`2 | beta`, `3 | gamma`, each with a line feed. Printed: `wrote 3 lines to joined.txt`.

## cases/2 (`notes.txt days.txt all.txt`)

notes.txt has 3 lines (`buy milk`, empty, `call bob`), days.txt 7 (mon to sun). K = 10, W = 2.
Numbers 1 to 9 get one space in front. Line 2 is empty, so it is ` 2 |` with nothing after.
Line 10 is `10 | sun`. Printed: `wrote 10 lines to all.txt`. Check: 3 + 7 = 10 written lines.

## hidden/1 (`missing.txt b.txt out.txt`)

missing.txt does not exist; check 1 fails first: `error: cannot read missing.txt`, status 20.

## hidden/2 (`a.txt nope.txt out.txt`)

a.txt reads; nope.txt does not exist: `error: cannot read nope.txt`, status 20.

## hidden/3 (`first.txt second.txt out.txt`)

Neither exists. Check 1 is made first: `error: cannot read first.txt`, status 20.

## hidden/4 (`a.txt b.txt out.txt`)

Both files empty, no lines, K = 0. out.txt is written empty. Printed `wrote 0 lines to out.txt`
(`lines`, since K is not 1).

## hidden/5 (`a.txt b.txt one.txt`)

a.txt empty (0 lines); b.txt `solo` with no line feed is 1 line. K = 1, W = 1. Written `1 | solo\n`
(every written line ends with a line feed). Printed `wrote 1 line to one.txt` (singular).

## hidden/6 (`s.txt s.txt twice.txt`)

s.txt `x\ny` has 2 lines (the last without a line feed); it is read twice. K = 4, W = 1. Written
`1 | x`, `2 | y`, `3 | x`, `4 | y`. Printed `wrote 4 lines to twice.txt`.

## hidden/7 (`a.txt b.txt out.txt`)

out.txt exists with older, longer text and is replaced entirely. a.txt `\n\n` is 2 empty lines;
b.txt ` lead` is 1 line whose text starts with a space. K = 3, W = 1. Written `1 |`, `2 |`, then
`3 | ` followed by ` lead`, which is `3 |  lead` (two spaces). Printed `wrote 3 lines to out.txt`.

## hidden/8 (`a.txt b.txt nodir/out.txt`)

Both inputs read. The folder `nodir` does not exist and is not created, so the write fails:
`error: cannot write nodir/out.txt`, status 20.
