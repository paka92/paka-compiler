# Numbered join

Join two text files into one written file with line numbers.

## Arguments

The program is run with exactly three arguments:

1. the name of the first input file;
2. the name of the second input file;
3. the name of the output file.

Names are used exactly as given, relative to the working directory. The output name always differs
from both input names. The two input names may be the same; the file is then read and joined twice.

## Input

An input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. A file holding only a line feed has one empty line.

## Output file

Take every line of the first file, then every line of the second file, in order. Number them 1,
2, 3 and so on across both files. Let `K` be the number of lines in all and `W` the number of digits
of `K`.

Each line is written as its number, right-aligned with spaces in front to width `W`, then a space,
a `|`, a space and the line's text. When the text is empty, the line is the right-aligned number,
a space and a `|`, with nothing after it. Every written line ends with a line feed.

With `K` = 10, the first and tenth lines of text `buy milk` and `sun`, and an empty second line:

```
 1 | buy milk
 2 |
...
10 | sun
```

When `K` is 0 the output file is written empty. An existing file of that name is replaced
entirely.

## Standard output

After the file is written, print one line and exit with status 0:

```
wrote K lines to NAME
```

`K` is written in decimal and `NAME` is the output name exactly as given. When `K` is 1, the
word is `line` instead of `lines`.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. The checks are made in this order and only the first that fails is reported:

1. The first file cannot be read, for example because it does not exist:
   `error: cannot read NAME`, with the first input name.
2. The second file cannot be read: `error: cannot read NAME`, with the second input name.
3. The output file cannot be written, for example because its folder does not exist:
   `error: cannot write NAME`, with the output name.

When either input file cannot be read, no file is written. The program never creates a folder.
