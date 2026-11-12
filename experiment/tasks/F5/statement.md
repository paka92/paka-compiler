# Chapter split

Split a book into one file per chapter, named after the chapter titles, and write a table of
contents.

## Arguments

The program is run with exactly two arguments: the book name and the contents name. Each is 1 to 30
characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `.`, `_`, `-` or `/`. The
book name ends with `.md`, and the contents name never ends with `.txt`.

## Input

The book holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1. The book has at most 200 lines of at most 200 characters. A
line is blank when it holds no character other than the space.

A heading is a line that is `#` alone or starts with `# ` (a `#` and a space). Its title is what
follows the `#`, with the spaces at both ends removed: the title of `#   Two  Words ` is
`Two  Words`. Lines such as `#Title` or `## Part` are not headings.

Each heading starts a chapter. The lines of a chapter are the lines after its heading, up to the
next heading or the end of the book, without the blank lines at their start and at their end.
Blank lines between other lines of the chapter stay. Before the first heading, only blank lines may
appear.

## File names

The slug of a title is made in four steps:

1. Change every upper-case letter `A` to `Z` to lower case.
2. Replace every run of characters that are not `a` to `z` or `0` to `9`, as long as possible, by
   one `-`, and remove a `-` at the start and at the end.
3. Keep only the first 20 characters, and again remove a `-` at the end.
4. When nothing is left, the slug is `chapter`.

So `The Big Day!` gives `the-big-day`, and `abcdefghij klmnopqr stu` gives `abcdefghij-klmnopqr`.

The chapters get their file names in book order. The base of a chapter is its slug when no earlier
chapter has that base; otherwise it is the slug, `-` and the smallest whole number from 2 up for
which the result is not the base of an earlier chapter. The file name is the base and `.txt`. So
the titles `Intro`, `Intro` and `Intro 2` get `intro.txt`, `intro-2.txt` and `intro-2-2.txt`.

## Output

Write one file per chapter, in book order: a line `# TITLE`, with the chapter's title, then the
lines of the chapter. Then write the contents file, with one line per chapter in book order:

```
K. TITLE -> NAME (L lines)
```

`K` is the chapter number, counting from 1, `NAME` its file name and `L` the number of lines of
the chapter, without its heading, both in decimal. When `L` is 1 the word is `line`.

Every line of every written file ends with a line feed. A file of the same name that already exists
is replaced entirely. When every file is written, print one line and exit with status 0:

```
split BOOK into C chapters
```

`BOOK` is the book name and `C` the number of chapters in decimal. When `C` is 1 the word is
`chapter`.

Example: run with `story.md` and `toc`, where `story.md` holds

```
# Start
It was dark.

And cold.

# The End!
Bye.
```

The file `start.txt` is written with the lines `# Start`, `It was dark.`, an empty line and
`And cold.`; the file `the-end.txt` with the lines `# The End!` and `Bye.`; and `toc` with the
lines `1. Start -> start.txt (3 lines)` and `2. The End! -> the-end.txt (1 line)`. The output is:

```
split story.md into 2 chapters
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. The checks are made in this order and only the first that fails is reported:

1. The book cannot be read, for example because it does not exist: `error: cannot read NAME`, with
   the book name.
2. The lines are checked in order from line 1, and only the first line with an error is reported,
   with `N` its line number in decimal:
   1. A line before the first heading that is not blank:
      `error: line N: text before first chapter`
   2. A heading whose title is empty: `error: line N: empty title`
3. The book has no heading: `error: no chapters`
4. A file cannot be written, for example because the contents name names a folder that does not
   exist: `error: cannot write NAME`, with the name of the first file in writing order that cannot
   be written.

When a check of 1 to 3 fails, no file is written. When a write fails, the files written before it
stay as written. The program never creates a folder.
