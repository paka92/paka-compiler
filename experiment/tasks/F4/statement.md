# Link rename

Rewrite the links in a set of wiki pages after some pages were renamed, following a rename map.

## Arguments

The program is run with exactly two arguments: the map name and the list name. The list file
always exists and can be read. Each of its lines holds a page name and nothing else, and every
line, the last one included, ends with a line feed. It has one to nine lines, and the page names
are all different.

A file name, in the arguments or the list, is 1 to 30 characters, each a letter `A` to `Z` or `a`
to `z`, a digit `0` to `9`, `.`, `_`, `-` or `/`. The map name, the list name and the page names
are all different. The pages are taken in list order.

## Files

The map and the pages hold only the characters with codes 32 to 126 and the line feed (code 10).
Each is a sequence of lines: each line ends at a line feed, and the last line may lack one. An
empty file has no lines. Lines are numbered from 1, blank lines included. Each file has at most 100
lines of at most 200 characters.

A key is 1 to 20 characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `_` or
`-`. Two keys match when they are equal after every upper-case letter is changed to lower case: `Intro`
matches `intro` and `INTRO`.

## Map

A word is a run of characters other than the space, as long as possible. A map line that holds no
word is blank and is skipped. Every other map line is an entry: exactly two words, `OLD` and `NEW`,
each a key. It says that the page `OLD` is now called `NEW`.

## Links

In a page, a link starts at `[[` and ends at the first `]]` after it on the same line; the search
for the next link goes on after that `]]`. Text outside links is plain text, so a `]]` that ends
no link, or a single `[` or `]`, is plain text.

What is between the `[[` and the `]]` is the inside of the link. It is either `TARGET`, or `TARGET`,
then `|`, then `LABEL`. `TARGET` must be a key. `LABEL` is any characters but `|`, and may be empty.
So `[[a|]]` and `[[a|[[b]]` are good links (the second has the label `[[b`), and `[[]]`,
`[[a b]]`, `[[a|b|c]]` and `[[[a]]` are bad ones.

## Renaming

Every link whose `TARGET` matches the `OLD` of an entry is rewritten: its `TARGET` is replaced by
that entry's `NEW`, exactly as written in the map. Nothing else in the page changes: the `[[`, the
`|`, the label, the `]]`, all plain text and every line feed stay as they are. Each link is
rewritten at most once, with the map as given: with the entries `a b` and `b c`, `[[a]]` becomes
`[[b]]`, not `[[c]]`, and the entries `x y` and `y x` swap the two names.

## Output

All pages are read first, in list order. Then every page in which at least one link is
rewritten is written back under its own name, in list order. A page with no rewritten link is
not written. Then print one line per page, in list order:

```
PAGE: K links
```

with `K` the number of links rewritten in it, then one last line:

```
rewrote L links in P pages
```

with `L` the number of links rewritten in all and `P` the number of pages written. All numbers are
in decimal. When a number is 1 the word after it is `link` or `page`. Every printed line ends with
a line feed, and the exit status is 0.

Example: run with `map.txt` and `pages`, where `pages` holds the lines `home.txt` and `guide.txt`,
and `map.txt` holds

```
intro start
setup install
```

`home.txt` holds

```
See [[Intro]] and [[setup|how to set up]].
[[faq]]
```

and `guide.txt` holds `Back to [[home]].` and a line feed. `home.txt` is written back with the
lines `See [[start]] and [[install|how to set up]].` and `[[faq]]`, and the output is:

```
home.txt: 2 links
guide.txt: 0 links
rewrote 2 links in 1 page
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. The checks are made in this order and only the first that fails is reported:

1. The map cannot be read, for example because it does not exist: `error: cannot read NAME`, with
   the map name.
2. The map lines are checked in order from line 1, and only the first line with an error is
   reported, with `N` its line number in decimal. On that line, the checks are made in this order:
   1. It is not blank and not an entry: `error: map line N: bad entry`
   2. Its `OLD` matches the `OLD` of an entry on an earlier line:
      `error: map line N: duplicate OLD`, with `OLD` as written on this line.
3. The map has no entry: `error: empty map`
4. The pages are read in list order; the first that cannot be read is reported:
   `error: cannot read NAME`, with the page name.
5. The pages are checked in list order, each from line 1, each line from left to right, and
   only the first problem found is reported, with `PAGE` the page name, `N` the line number and
   `C` the column of the first `[` of the `[[` (the first character of a line is column 1), both
   in decimal:
   1. A `[[` that has no `]]` after it on its line: `error: PAGE line N col C: unclosed link`
   2. A bad link: `error: PAGE line N col C: bad link`
6. A page cannot be written: `error: cannot write NAME`, with the name of the first page in
   writing order that cannot be written.

So a page that cannot be read is reported even when an earlier page has a bad link. When a check
of 1 to 5 fails, no file is written. When a write fails, the pages written before it stay as
written.
