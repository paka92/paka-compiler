# Variable expansion

Read a file of variable definitions and print every variable with its references expanded.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. The file has at most 100 lines of at most 100 characters. Lines are numbered from 1,
skipped lines included, and columns from 1: the first character of a line is in column 1.

## Lines

A line that is empty, or whose first character is `#`, is skipped. Every other line is a
definition: a name, directly followed by `=`, directly followed by the value. The value is the rest
of the line after that first `=`, exactly as written: it may be empty, and spaces in it are kept.

A name is a lower-case letter `a` to `z` followed by any number of lower-case letters, digits `0`
to `9` and underscores `_`.

A name may be defined on more than one line. Its value is the one on its last definition, and it
is printed at the place of its first definition.

## Values

A value is read from left to right as a row of pieces. Every character other than `$` is a piece
that stands for itself. A `$` starts one of these pieces:

- `$$` stands for one `$`.
- `$NAME` is a reference to the variable `NAME`. `NAME` is the longest run of lower-case letters,
  digits and underscores after the `$`, and its first character must be a lower-case letter. So in
  `$ab_1.x` the name is `ab_1`, and `.x` is text after it.
- `${NAME}` is a reference to the variable `NAME`.
- `${#NAME}` stands for the length of the expanded value of `NAME`, counted in characters and
  written in decimal.
- `${NAME:-DEFAULT}` stands for the expanded value of `NAME`, except when `NAME` is not defined
  anywhere in the file or its expanded value is empty: then it stands for `DEFAULT`, expanded.
  `DEFAULT` is read by the same rules, so it may hold any pieces, references with defaults
  included, with one change: inside `DEFAULT`, a `}` that is not part of a piece inside it ends
  `DEFAULT` and the whole reference. `DEFAULT` may be empty.

Outside a default, `{` and `}` are plain characters that stand for themselves.

## Expansion

The expanded value of a variable is its value with every piece replaced by what it stands for. A
reference to a variable stands for that variable's expanded value.

Variables are expanded in the order they are printed. Each value is expanded piece by piece from
left to right; when a piece needs another variable, that variable is expanded completely before
the next piece. A variable already expanded keeps its expanded value and is not expanded again.
A default that is not used is not expanded, so the references in it are never looked at. A
`${NAME:-DEFAULT}` whose `NAME` is defined always expands `NAME` first, even if its value turns
out to be empty.

## Output

When there is no error, print one line `NAME=VALUE` for every defined variable, with `VALUE` its
expanded value, in the order of their first definitions. Every line ends with a line feed. A file
with no definitions prints nothing. The exit status is 0. No expanded value is longer than 1,000
characters.

Example input:

```
# paths
home=/home/$user
user=ann
log=${dir:-$home/log}/app-${#user}.txt
user=bob$$
```

Output:

```
home=/home/bob$
user=bob$
log=/home/bob$/log/app-4.txt
```

`dir` is not defined, so the default is used; the length of `bob$` is 4.

## Errors

An error prints exactly one line, ending with a line feed, and nothing else, and exits with status
20. First every definition line is checked, from line 1 down, and the first line with a syntax
error is reported. Only when no line has a syntax error are the variables expanded.

### Syntax errors

A syntax error is written as follows, with `L` the line number and `C` a column, both in decimal:

```
error: line L col C: MESSAGE
```

A definition line is read once from left to right, and the first error met is reported:

- `bad name`: the line does not start with a name directly followed by `=`. `C` is the column of
  the first character that does not fit: column 1 when the line does not start with a lower-case
  letter, else the column just after the longest run of name characters at its start, which is
  the length of the line plus one when the line ends there.
- `bad dollar`: a `$` that does not start a `${` reference is followed by a character that is not
  `$` or a lower-case letter, or by the end of the line. It is met at that character or line end.
  `C` is the column of the `$`.
- `bad reference`: after `${` comes something that does not fit one of `${NAME}`, `${#NAME}` or
  `${NAME:-`. It is met at the first character that does not fit. `C` is the column of the `$` of
  that `${`.
- `unclosed brace`: the line ends before a `${` reference is complete, whatever part of it is still
  missing. It is met at the end of the line. `C` is the column of the `$` of the leftmost `${` that
  is still open there.

When the line ends directly after a `$` inside an open `${` reference, both `bad dollar` and
`unclosed brace` are met at the end of the line; `bad dollar` is reported.

### Expansion errors

The first problem met during expansion, in the order described above, is reported:

- `error: undefined NAME in OWNER`: a `$NAME`, `${NAME}` or `${#NAME}` piece names a variable that
  is not defined anywhere in the file. `OWNER` is the variable on whose line the piece is written.
  A `${NAME:-DEFAULT}` piece is never this error for its own `NAME`.
- `error: cycle A -> B -> A`: a variable is needed while it is still being expanded. While
  expanding, there is a chain of variables: the one being printed, then the one it is waiting for,
  and so on, ending with the variable whose piece is being read. When a piece needs a variable that
  is in this chain, the message lists the chain from that variable to its end, then that variable
  again, with ` -> ` between names. A variable that refers to itself gives `cycle a -> a`.
