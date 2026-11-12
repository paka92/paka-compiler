# F4 hand working

Not part of the statement. One entry per case: the map checks, the pages in list order, each link
with what it becomes, then the writes and the report.

## cases/1 (`map.txt`, `pages`)

The worked example. `[[Intro]]` matches `intro`: `[[start]]`. `[[setup|how to set up]]`:
`[[install|how to set up]]`. `[[faq]]` and `[[home]]` match nothing. Only `home.txt` is written.
2 links in 1 page.

## cases/2 (`map.txt`, `pages`)

Map good. `notes.txt` line 1 `[[old]]` good. Line 2 `Bad [[old page]] link.`: the `[[` is at
column 5; its inside `old page` has a space, not a key: `error: notes.txt line 2 col 5: bad link`,
status 20. Nothing is written.

## hidden/1 (`moves.map`, `list`)

Entries: a -> b, b -> c, x_1 -> Y-2, Y-2 -> x_1 (a swap; `Y-2` and `x_1` are different keys from
`x_1` and `Y-2`, so no duplicate).

- `a.md` line 1: `[[A]]` -> `[[b]]` (matched once, not again by `b c`); `[[b]]` -> `[[c]]`; `[[c]]`
  unchanged. Line 2: `[[X_1|first]]` -> `[[Y-2|first]]`; `[[y-2]]` matches `Y-2` -> `[[x_1]]`;
  `[[x_12]]` unchanged. 4 links.
- `b.md`: `[[B|bee]]` -> `[[c|bee]]`; `[[a]]` right after it -> `[[b]]`. 2 links.

Both written. `rewrote 6 links in 2 pages`.

## hidden/2 (`m`, `l`)

Map line 1 blank; line 2 `  old    New  ` is two words: old -> New.

- `p1` line 1: `[[old|]]` (empty label) -> `[[New|]]`; `[[old|[[x]]` is a link with label `[[x`
  -> `[[New|[[x]]`; ` then ]] ` is plain text; `[[ok]]` unchanged. Line 2 (no line feed):
  `[ [old] ]` is plain text; `[[OLD|a ] b]]` with label `a ] b` -> `[[New|a ] b]]`; the last `]`
  is plain text. 3 links. The missing line feed stays missing.
- `p2`: no link. `p3`: `[[new]]` is a NEW, not an OLD: unchanged.

Only `p1` written. `p1: 3 links`, `p2: 0 links`, `p3: 0 links`, `rewrote 3 links in 1 page`.

## hidden/3 (`map`, `list`)

`one.txt` good. `two.txt` line 1 good. Line 2 `x [[a|ok]] [[a|b|c]] [[`: `[[a|ok]]` takes columns
3 to 10; `[[a|b|c]]` starts at column 12 and has two `|`:
`error: two.txt line 2 col 12: bad link`. The unclosed `[[` later on that line and the one in
`three.txt` come after it.

## hidden/4 (`map`, `list`)

`page` line 1 `[[a]] [[a]`: the first link is good; the second `[[` at column 7 has no `]]` after
it on line 1 (the `]]` on line 2 does not count): `error: page line 1 col 7: unclosed link`.

## hidden/5 (`map`, `list`)

Line 1 `Intro`, line 2 `Intro-2` (a different key), line 3 blank, line 4 `intro` matches `Intro`:
`error: map line 4: duplicate intro`, as written on line 4. The bad key on line 5 is not reached.

## hidden/6 (`map`, `list`)

Line 1 good, line 2 blank, line 3 has three words: `error: map line 3: bad entry`. The missing page
is not reached.

## hidden/7 (`map`, `list`)

Map good. All pages are read before any link is checked: `first` reads (it has a bad link),
`second` does not exist: `error: cannot read second`.

## hidden/8 (`map`, `list`)

Both map lines are blank: `error: empty map`.
