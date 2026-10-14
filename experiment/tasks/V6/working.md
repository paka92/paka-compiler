# V6 hand working

Not part of the statement. One entry per case: the mode and name checks, then for `pack` the list
and the length of every file, for `unpack` the archive walked line by line (LK for line K), and what
is written. Second check: every `L` equals the characters counted, line feeds included.

## cases/1 (`pack`, `files.lst`, `bundle.arc`)

List: L1 `one.txt`, L2 `two.txt`, both names, no repeat. `one.txt` is `first` (5), a line feed,
`second` (6), a line feed: 13. `two.txt` is `no newline`, 10, with no line feed. Archive: header
`archive 2`; `entry one.txt 13`, the 13 characters, one line feed (an empty line, as the content
ended with a line feed); `entry two.txt 10`, the 10 characters, one line feed. Output:
`packed 2 files into bundle.arc`.

## cases/2 (`unpack`, `box.arc`, `*`)

- L1 `archive 3`: N = 3.
- L2 `entry a.txt 2`: content `x` and a line feed (L3), then the line feed after it: L4 empty.
- L5 `entry b.txt 0`: no content, then the line feed: L6 empty.
- L7 `entry c.md 15`: content `entry z.txt 1` (13), a line feed (14), `q` (15); the next character
  is a line feed. L8 looks like an entry line but is content.
- The archive ends after the third entry: no trailing data.

Files: `a.txt` with `x` and a line feed, `b.txt` empty, `c.md` with the two lines, the last with no
line feed. Output: `unpacked 3 files from box.arc`.

## hidden/1 (`pack`, `l`, `out.arc`)

The list has no final line feed: L1 `e.txt`, L2 `mix.txt`, L3 `DATA-1`, all names (`DATA-1` starts
with a letter; `-` is allowed later).

- `e.txt` is empty: `entry e.txt 0`, then one line feed.
- `mix.txt` is `entry x 3` (9), a line feed (10), `abc` (13), a line feed (14), a line feed (15):
  `entry mix.txt 15`, the 15 characters, one line feed. The content looks like an entry; it is
  copied as it is.
- `DATA-1` is a space and `lead` (5), a line feed (6), `trail` and a space (12), no line feed:
  `entry DATA-1 12`, the 12 characters, one line feed. Spaces stay.

Output: `packed 3 files into out.arc`.

## hidden/2 (`pack`, `names`, `a.arc`)

L1 `p.txt`, L2 `q.txt` fine. L3 `p.txt` is a name, the same as L1: `error: list line 3: duplicate
p.txt`, status 20. L4 `-bad` (starts with `-`) is later, and `q.txt`, which does not exist, is
never read: the list is checked before any file is read.

## hidden/3 (`pack`, `names.txt`, `b.arc`)

L1 `a` is a name. L2 is empty, not a name: `error: list line 2: bad name`, status 20. Neither `a`
nor `b` exists, but no file is read.

## hidden/4 (`pack`, `set`, `dir/c.arc`)

The list is fine. `x1` is read; `x2` does not exist: `error: cannot read x2`, status 20. The archive
could not be written either (no folder `dir`), but reading comes first.

## hidden/5 (`unpack`, `t.arc`, `*`)

- L1 `archive 2`.
- L2 `entry r.txt 0`: no content, the line feed after it is L3, empty.
- L4 `entry s.txt 3`: content three line feeds (L5, L6, L7 empty), then the line feed after it
  (L8 empty). The archive ends there.

`r.txt` already holds `old`; it is replaced by the empty file. `s.txt` holds three line feeds.
Output: `unpacked 2 files from t.arc`.

## hidden/6 (`unpack`, `u.arc`, `b.txt`)

`b.txt` is a name. L1 `archive 2`. L2 `entry a.txt 4`: `AAA` and a line feed, then the line feed
(L4 empty). L5 `entry b.txt 5`: `B`, line feed, `B`, line feed, `B`, then the line feed. The archive
ends. Only `b.txt` is written, holding the three `B` lines with no final line feed. Output:
`unpacked 1 file from u.arc`, singular.

## hidden/7 (`unpack`, `v.arc`, `*`)

L1 `archive 2`. L2 `entry a.txt 4`: content `a`, line feed, `b`, line feed (L3, L4), then the line
feed (L5 empty). L6 `entry b.txt 2`: content `xy`, then `z`, not a line feed:
`error: archive line 6: bad length`, status 20. Before L6 there are 5 line feeds, so K = 6.

## hidden/8 (`unpack`, `w.arc`, `zz.txt`)

`zz.txt` is a name. L1 `archive 1`. L2 `entry y.txt 1`: content `Y`, then the line feed. The one
entry is done, but `extra` follows, starting after 3 line feeds: `error: archive line 4: trailing
data`, status 20. No entry is named `zz.txt` either, but the archive check comes first.
