# F5 hand working

Not part of the statement. One entry per case: the line checks, then each chapter's title, slug,
base and kept lines, then the files and the output.

## cases/1 (`story.md`, `toc`)

The worked example. Start: lines `It was dark.`, empty, `And cold.`, then a blank line that is
dropped at the end: 3 lines. `The End!`: slug `the-end` (the `!` becomes a `-` at the end, removed):
1 line. `split story.md into 2 chapters`.

## cases/2 (`notes.md`, `toc`)

Line 1 `Notes` is not blank and comes before the first heading:
`error: line 1: text before first chapter`, status 20.

## hidden/1 (`same.md`, `contents`)

Slugs: `intro`, `intro` (`!` dropped), `intro-2`, `intro`, `intro-2-2`.

1. `intro`: free: `intro.txt`.
2. `intro` taken; `intro-2` free: `intro-2.txt`.
3. `intro-2` taken (by chapter 2); `intro-2-2` free: `intro-2-2.txt`.
4. `intro`, `intro-2` taken; `intro-3`: `intro-3.txt`.
5. `intro-2-2` taken (by chapter 3); `intro-2-2-2`: `intro-2-2-2.txt`.

Each has 1 line. `split same.md into 5 chapters`.

## hidden/2 (`slugs.md`, `list`)

- `abcdefghij klmnopqr stu` -> `abcdefghij-klmnopqr-stu`; first 20 end in `-`, removed:
  `abcdefghij-klmnopqr`.
- `#   Spaced   Out  `: title `Spaced   Out`, kept as is in the file and the list; slug
  `spaced-out`.
- `--C++ & Rust: 2024--`: runs `--`, `++ & `, `: `, `--` become `-`; ends removed: `c-rust-2024`.
- `abcdefghijklmnopqrstuvwxyz`: first 20: `abcdefghijklmnopqrst`.

## hidden/3 (`trim.md`, `toc.list`)

Lines 1 and 2 are blank before the first heading: allowed.

- First (line 3): lines 4 to 11 are empty, spaces, `keep  `, empty, `## not a heading`,
  `#nor this`, spaces, empty. Drop the two blank at the start and the two at the end: `keep  `
  (trailing spaces kept), empty, `## not a heading`, `#nor this`: 4 lines.
- Empty (line 12): only blank lines: 0 lines, the file is just `# Empty`.
- Last (line 15): `  indented`, then an empty line and a last line of two spaces without a line
  feed, both dropped: 1 line.

## hidden/4 (`bad.md`, `toc`)

Line 1 blank, line 2 heading `Fine`, line 3 text. Line 4 `#    `: starts with `# `, title empty
after removing spaces: `error: line 4: empty title`. Line 5 is not reached.

## hidden/5 (`missing.md`, `toc`)

No book: `error: cannot read missing.md`.

## hidden/6 (`blank.md`, `toc`)

Four blank lines (the last of three spaces, no line feed) and no heading: `error: no chapters`.

## hidden/7 (`two.md`, `out/toc`)

Chapters `one.txt` and `two.txt` are written first; the folder `out` does not exist:
`error: cannot write out/toc`. The two chapter files stay written.

## hidden/8 (`odd.md`, `toc`)

Slugs: `???` -> empty -> `chapter`; `Chapter` -> `chapter`; `#  !!! ` -> title `!!!` -> `chapter`;
`chapter 2` -> `chapter-2`. Bases: `chapter`; `chapter-2`; `chapter-3` (`chapter-2` taken);
`chapter-2` taken, so `chapter-2-2`. `split odd.md into 4 chapters`.
