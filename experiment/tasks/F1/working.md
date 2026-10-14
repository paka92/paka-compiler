# F1 hand working

Not part of the statement. One entry per case: the merge step by step, with the chain and the set
of finished files, then the output. Second check: `F` is the number of different files named in
the steps, and `K` the number of lines in `out/`.

## cases/1 (`doc.txt merged.txt`)

Chain doc.txt. Line 1 `# Guide` and line 2 (empty) are text. Line 3 merges intro.txt: `Hello.`,
`Welcome.`; intro.txt finished. Line 4 merges steps.txt: `1. open`, `2. close` (the last line has
no line feed; it is written with one); steps.txt finished. Line 5 `bye`.

Seven lines written. Files doc, intro, steps: `read 3 files, wrote 7 lines to merged.txt`.

## cases/2 (`site.txt page.txt`)

Chain site.txt. Line 1 merges head.txt (chain site, head): `<head>`, then head's line 2 merges
nav.txt: `<nav/>`; nav finished. `</head>`; head finished. site line 2 `body`. Line 3 names nav.txt:
not on the chain, finished, adds nothing. Line 4 names head.txt: the same.

Lines `<head>`, `<nav/>`, `</head>`, `body`: `read 3 files, wrote 4 lines to page.txt`.

## hidden/1 (`top.txt all.txt`)

`in/` holds `Top.txt` only; names differ in case, so `top.txt` cannot be read: `error: cannot read
top.txt`, status 20.

## hidden/2 (`r.txt r.out`)

Line 1 `start` is text. Line 2 merges b.txt (chain r, b): `b1`, then b's line 2 `#include c.txt` is
of the valid form, c.txt is not on the chain and not finished, and cannot be read: `error: b.txt
line 2: cannot read c.txt`, status 20. r's line 3 `#include  extra.txt` (two spaces, a bad include)
comes later in merge order and is never reached.

## hidden/3 (`r.txt joined.out`)

r line 1 merges a.txt (chain r, a): `alpha`, then a's line 2 merges b.txt (chain r, a, b). b's line
1 `#include a.txt`: a.txt is on the chain: `error: b.txt line 1: include cycle: r.txt -> a.txt ->
b.txt -> a.txt`, status 20. r's line 2 `#include` (a bad include) is never reached.

## hidden/4 (`x.txt y.txt`)

Lines 1 and 2 are text. Line 3 includes x.txt, the root itself, on the chain (x): `error: x.txt
line 3: include cycle: x.txt -> x.txt`, status 20.

## hidden/5 (`p.txt q.txt`)

Line 1 starts with a space and line 2 with `#Include`: their first 8 characters are not `#include`,
so both are text. Line 3 `#include-a.txt` starts with `#include` but has `-` where the space must
be: `error: p.txt line 3: bad include`, status 20.

## hidden/6 (`e.txt old.log`)

e.txt is empty: no line. One file read, nothing copied. `old.log` exists with `stale`; writing
replaces it with an empty file (`out/old.log` is empty). `read 1 file, wrote 0 lines to old.log`.

## hidden/7 (`t.txt all.txt`)

Line 1 merges e.txt (empty); e finished. Line 2 merges n.txt: `last` (no line feed); n finished.
Line 3 (no line feed) names e.txt, finished: adds nothing. Files t, e, n; one line written, `last`
with a line feed: `read 3 files, wrote 1 line to all.txt`.

## hidden/8 (`w.txt nodir/w.out`)

The merge works: `x`, `y`; no merge error. No directory `nodir` exists, so the write fails:
`error: cannot write nodir/w.out`, status 20.
