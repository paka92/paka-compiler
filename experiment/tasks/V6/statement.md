# Pack and unpack

Pack text files into one archive file, or unpack files from an archive.

## Arguments

The program is run with exactly three arguments: the mode, then two names.

```
pack LIST ARCHIVE
unpack ARCHIVE WHICH
```

- `pack` reads the file `LIST`, which names the files to pack, and writes the archive `ARCHIVE`.
- `unpack` reads the archive `ARCHIVE` and writes the files in it: every file when `WHICH` is `*`,
  else only the file named `WHICH`.

A name is 1 to 30 characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `.`,
`_` or `-`, and its first character is a letter or a digit. `LIST` and `ARCHIVE` are always 1 to 30
characters from the same set and `/`. `ARCHIVE` is never `LIST` and never a name in the list.

## Files

Every file read holds only the characters with codes 32 to 126 and the line feed (code 10). Where
a file is read as lines, each line ends at a line feed and the last line may lack one; an empty
file has no lines, and lines are numbered from 1.

The list has at most 20 lines. Each file to pack has at most 50 lines of at most 200 characters.
An archive to unpack has at most 200 lines and at most 20000 characters.

## Archive

An archive is one header line, then one entry per packed file, and nothing else:

- The header line is `archive N`, with `N` the number of entries.
- Each entry is a line `entry NAME L`, then the `L` characters of the file `NAME` exactly as they
  are, line feeds included, then one line feed.

Both lines are written exactly in these forms: the word, single spaces, and nothing else on the
line. `N` and `L` are written in decimal: `N` is 1 to 3 digits and `L` 1 to 5 digits, with no
leading zero unless the number is 0. `NAME` is a name. The single line feed after the content is always
there, so a file that ends with a line feed is followed by an empty line, and an empty file gives
an entry line followed by an empty line.

For example, packing `notes.txt`, which holds `hi` and a line feed, and then the empty file
`empty.txt` gives this archive, which ends with a line feed after its last, empty, line:

```
archive 2
entry notes.txt 3
hi

entry empty.txt 0

```

## Pack

1. Read `LIST`. Every line of it must be a name; an empty line is not a name. The same name must
   not be on two lines.
2. Read every named file, in list order.
3. Write the archive: the header, then one entry per file in list order. An empty list gives the
   archive `archive 0` with its line feed.
4. Print `packed N files into ARCHIVE`, with `N` in decimal; when `N` is 1 the word is `file`.

## Unpack

1. Read `ARCHIVE` and check its form from the start, as described under Errors.
2. When `WHICH` is not `*`, it must be the name of an entry.
3. Write the files: with `*` one file per entry in archive order, else the one entry named `WHICH`.
   Each file holds exactly the entry's content. A file of the same name that already exists is
   replaced entirely.
4. Print `unpacked N files from ARCHIVE`, with `N` the number of files written in decimal; when `N`
   is 1 the word is `file`.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. The checks are made in this order and only the first that fails is reported. No file is
written unless every read and check before the writing succeeded.

1. The mode is not exactly `pack` or `unpack`: `error: bad mode`.

For `pack`:

2. `LIST` cannot be read, for example because it does not exist: `error: cannot read LIST`.
3. The list lines are checked in order, and the first line that fails is reported, with `N` its
   line number in decimal. On that line, first: it is not a name, `error: list line N: bad name`;
   then: it is the same as an earlier line, `error: list line N: duplicate NAME`.
4. The first file in list order that cannot be read: `error: cannot read NAME`.
5. The archive cannot be written, for example because its folder does not exist:
   `error: cannot write ARCHIVE`.

For `unpack`:

2. `WHICH` is neither `*` nor a name: `error: bad name WHICH`. This is checked before the archive
   is read.
3. `ARCHIVE` cannot be read: `error: cannot read ARCHIVE`.
4. The archive is checked from its start. The first problem is reported as
   `error: archive line K: REASON`, where `K` is 1 plus the number of line feeds in the archive
   before the line named below, content line feeds included:
   1. The archive does not start with a header line ending in a line feed: `bad header`, with
      `K` = 1. For example `archive 02` is not a header line.
   2. For each of the `N` entries in turn, at the point where the entry starts:
      1. There is no entry line ending in a line feed there, also when the archive has ended:
         `bad entry`, with `K` that of the line starting there.
      2. `NAME` is the name of an earlier entry: `duplicate NAME`, with `K` that of the entry line.
      3. Fewer than `L` + 1 characters follow the entry line, or the character after the `L`
         content characters is not a line feed: `bad length`, with `K` that of the entry line.
   3. Characters follow the last entry: `trailing data`, with `K` that of the line where they
      start.
5. `WHICH` is not `*` and no entry has that name: `error: no entry WHICH`.
6. The first file in writing order that cannot be written: `error: cannot write NAME`.

For example, the archive `archive 1`, `entry a.txt 5`, `abc`, with a line feed after each line,
has only 4 characters after its entry line, and gives `error: archive line 2: bad length`. The
archive `archive 0`, a line feed and `x` gives `error: archive line 2: trailing data`.

Unpacking the example archive under Archive with `WHICH` = `empty.txt` writes only the empty file
`empty.txt` and prints `unpacked 1 file from ARCHIVE`, with the archive's name.
