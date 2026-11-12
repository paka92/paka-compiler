# F3 hand working

Not part of the statement. One entry per case: the script checks, the places each command takes
(gap g is 2g, line l is 2l - 1), then the result and the summary.

## cases/1 (`notes.txt`, `fix.txt`, `new.txt`)

The worked example. Places: `d 2` 3; `c 3` 5; `a 4` 8. Result `one THREE 3.5 four five`, 5 lines,
`+3 -2`.

## cases/2 (`list.txt`, `s.txt`, `out/list.txt`)

Script and target good; `a 2` adds `c` after the last line. The folder `out` does not exist:
`error: cannot write out/list.txt`, status 20.

## hidden/1 (`page.txt`, `p.txt`, `page2.txt`)

The target's last line `end` lacks a line feed. `c 2` takes 3 with three text lines: `>` is an
empty line, `>   indented` is `  indented` (two spaces), `> > quoted` is `> quoted`. `d 4` takes 7.
Result: `alpha`, empty, `  indented`, `> quoted`, `gamma`, `end`. `end` is kept and nothing is
added after it, so it still lacks a line feed. 6 lines, `+3 -2`.

## hidden/2 (`mid.txt`, `ends.txt`, `out.txt`)

Target `b`, `c` (no final line feed), `n` 2. The script's last line `> e` lacks a line feed; it is
still a text line. `a 0` takes 0, `a 2` takes 4. Result `a b c d e`: lines are added after `c`, so
every line ends with a line feed. 5 lines, `+3 -0`.

## hidden/3 (`doc.txt`, `edits.txt`, `doc.txt`)

The output is the target. `d 2-3` takes 3 to 5; `a 3` takes 6, after 5; `c 4-5` takes 7 to 9.
Result `l1`, `new`, `last`. `l5` lacked a line feed but is changed, so every line ends with one.
`doc.txt` is replaced. 3 lines, `+2 -4`.

## hidden/4 (`six.txt`, `s.txt`, `o.txt`)

`d 1` takes 1. `a 5` takes 10. `c 5` takes 9, not after 10: `error: script line 4: out of order`.

## hidden/5 (`five.txt`, `s.txt`, `o.txt`)

`n` 5. `a 5` is good (gap 5 exists) and takes 10. `d 2-6` names line 6: past end is checked before
order (it would also be out of order): `error: script line 3: past end`.

## hidden/6 (`gone.txt`, `s.txt`, `o.txt`)

Line 1 `d 1` good. Line 2 `a 1`: the next line `>x` is not a text line, so line 2 fails first:
`error: script line 2: missing text`. Line 3 is never reported, and the missing target is never
read.

## hidden/7 (`t.txt`, `s.txt`, `o.txt`)

`a 0` with `> top`, then `d 3`, then `> stray` follows a `d` command:
`error: script line 4: unexpected text`. Line 5 `d 9-2` is never checked.

## hidden/8 (`nofile.txt`, `s.txt`, `o.txt`)

`c 4-4` with `> four` good. Line 3 `d 3-1`: `B` less than `A`:
`error: script line 3: backward range`. The script is checked before the target is read, so the
missing target is not reported.
