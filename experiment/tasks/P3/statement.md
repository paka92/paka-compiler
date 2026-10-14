# Nested config

Read a nested configuration file and print every value with its full path.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It has
at most 200 lines. Lines are numbered from 1 and columns from 1: the first character of a line is in
column 1. Tables and lists nest at most 100 deep.

The end of the file has a position too: the line and column that one more character would have if
it were added at the end. So an empty file ends at line 1 col 1, a file whose last line is `ab`
without a line feed ends at col 3 of that line, and a file that ends with a line feed ends at col 1
of the line after its last line.

## Tokens

The file is read from the start as tokens. Spaces, line feeds and comments between tokens are
skipped. A comment starts at a `#` outside a string and runs to the end of its line, not including
the line feed. A token's position is the position of its first character.

- A **name** is one or more of the characters `A` to `Z`, `a` to `z`, `0` to `9` and `_`, as many as
  follow one another. `8080` and `_x1` are names.
- A **string** starts with `"` and ends at the next `"` that is not part of an escape. It must end on
  the line where it starts. Inside it, every character other than `"` and `\` stands for itself, and
  `\` starts an escape:
  - `\"` stands for `"`;
  - `\\` stands for `\`;
  - `\x` followed by two hex digits (`0` to `9`, `a` to `f`, `A` to `F`) stands for the character
    with that code, which must be from 32 to 126: `\x21` is `!`.
- A **symbol** is one of `=`, `{`, `}`, `[`, `]` and `,`.

## Structure

The tokens must form a file of this grammar, where `{ ... }` means repeated zero or more times and
`[ ... ]` means optional:

```
file   = { entry }
entry  = name "=" value
value  = string | list | table
list   = "[" [ value { "," value } ] "]"
table  = "{" { entry } "}"
```

The file itself is the top table, and its entries are the top table's entries. Entries need no
separator. A list has no comma after its last item.

## Output

Every value has a path:

- an entry of the top table has its name as path;
- an entry of a table with path `P` has the path `P.name`;
- item `i` of a list with path `P`, counted from 0, has the path `P[i]`.

Print one line for each string, each empty list and each empty table, other than the top table
itself, written `PATH = VALUE`:

- a string is written as `"`, its text, then `"`. In the text, every `\` is written `\\` and every
  `"` is written `\"`; every other character is written as itself, also when the file gave it with
  `\x`.
- an empty list is written `[]` and an empty table `{}`.

A list or table that is not empty prints no line of its own. The lines come in this order:

- the entries of a table in rising ASCII order of their names: at the first character where two
  names differ, the one with the lower code comes first, and a name that is the start of a longer
  name comes before it;
- the items of a list in their order;
- everything below one entry or item comes before the next entry or item.

A file whose top table has no entries prints nothing. Every printed line ends with a line feed. The
exit status is 0.

Example input:

```
# sizes
box = { w = "2" h = "3" }
name = "kit\x21"
tags = ["red", [], {}]
```

Output:

```
box.h = "3"
box.w = "2"
name = "kit!"
tags[0] = "red"
tags[1] = []
tags[2] = {}
```

## Errors

An error prints exactly one line, ending with a line feed, and nothing else, and exits with status
20. `L` is a line number and `C` a column, both in decimal:

```
error: line L col C: MESSAGE
```

The file is checked in three stages, and the first stage that finds an error reports it:

1. **Characters.** The file is read from the start as above, and the first of these problems met is
   reported:
   - `bad character`: outside strings and comments, a character that is not a space, a line feed,
     `#`, `"`, a name character or a symbol. `C` is its column.
   - `bad escape`: inside a string, a `\` that is not followed by `"`, `\`, or `x` and two hex
     digits with a value from 32 to 126. A `\` followed by a line feed or by the end of the file is
     a bad escape too. The position is that of the `\`.
   - `unterminated string`: a string that reaches the end of its line or the end of the file
     before its closing `"`. The position is that of its opening `"`.

   A string is read from left to right, and an escape is checked when it is reached, so a string
   with a bad escape that is also not closed reports the bad escape.
2. **Syntax.** The tokens must form a file of the grammar above. If they do not, the message is
   `syntax error`, at the first token at which the tokens read so far are no longer the start of
   any valid file. When every token keeps them the start of a valid file but the file ends too
   soon, the position is the end of the file.
3. **Keys.** A name that is the same as the name of an earlier entry of the same table: the message
   is `duplicate key NAME`, with the name, at the position of the later name. When there are
   several, the one that comes first in the file is reported.

Example: when the first line of the file is `k = = "v" ;`, the bad character `;` at column 11 is
reported, not the syntax error at column 5:

```
error: line 1 col 11: bad character
```
