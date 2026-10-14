# Column table

Turn a file of comma-separated rows into an aligned table, with a total line for the number
columns.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1.

Every line is one row, and an empty line is a row too. The first row is the header; every later row
is a data row. A row is split into fields at commas, read from left to right:

- A field whose first character is a double quote `"` is quoted. Its value is the text up to the
  next `"` that is not doubled. Inside it, a comma is part of the value and `""` stands for one `"`.
  The closing `"` must be followed by a comma or by the end of the line.
- Any other field is unquoted. Its value is all the text up to the next comma or the end of the
  line, exactly as written: spaces are kept, and the value may be empty. It must not contain `"`.
- An empty line is a row with one empty field. A line ending in a comma has an empty last field.

## Numbers

A value is a number when it is an optional `-` followed by 1 to 12 digits `0` to `9`, and nothing
else. So `007`, `-0` and `-12` are numbers; `+5`, `1e3`, `1.5`, ` 4`, an empty value and a run of
13 digits are not. Whether the value was quoted does not matter: `"12"` is the number 12.

A column is a number column when there is at least one data row and every data value in that
column is a number. Every other column is a text column. Header values are never looked at for
this.

## Output

Values are always printed exactly as they were read (after removing the quotes as above), never
rewritten.

There is a total line when at least one column is a number column. On it:

- a number column holds the sum of its values, written in decimal with a `-` in front when it is
  negative, no leading zeros, and `0` for zero;
- the first column, when it is a text column, holds the word `total`;
- every other text column holds an empty value.

The width of a column is the length of its longest value, counting the header, every data value,
and its value on the total line when there is one. A width is never less than 1.

The header line, each data line and the total line are table lines. A table line is made from
one value per column. Each value is padded with spaces to its column's
width, at the front in a number column and at the end in a text column, and the padded values are
joined with ` | ` (space, bar, space). Then all spaces at the end of the line are removed. A rule
line is made by writing, for each column, as many `-` as its width, and joining them with `-+-`.

Print, in this order:

1. the header line;
2. a rule line;
3. one table line per data row, in input order;
4. when there is a total line: a rule line, then the total line.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
item,qty
"fig, dried",12
kiwi,3
```

Output:

```
item       | qty
-----------+----
fig, dried |  12
kiwi       |   3
-----------+----
total      |  15
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. No part of the table is printed. `N` below is a line number in decimal.

- The file has no lines: `error: empty input`
- On line `N`, an unquoted value contains `"`, a closing `"` is followed by something other than
  a comma or the end of the line, or a quoted field is not closed before the end of the line:
  `error: line N: bad quote`
- Line `N` has `F` fields while the header has `H`: `error: line N: expected H fields, got F`

The lines are checked in order from line 1, and only the first line with an error is reported. When
a line has a quote error, that error is reported, not its number of fields.
