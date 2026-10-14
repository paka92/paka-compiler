# Calculator

Run a small program of integer assignments and expressions, one per line.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. Columns are numbered from 1: the first
character of a line is in column 1.

No number written in the file, no variable and no value worked out along the way is less than
-9007199254740991 or greater than 9007199254740991. Brackets and unary `-` nest at most 100 deep.

## Tokens

A line is read from left to right as tokens. At each point, spaces are skipped, and the next token
is the longest run that fits one of these:

- a number: one or more digits `0` to `9`, read in decimal (`007` is 7);
- a name: a letter `A` to `Z` or `a` to `z` or an underscore `_`, followed by any letters, digits
  and underscores (`x2` is one name; `2x` is the number `2` then the name `x`);
- one of the eight symbols `+ - * / % ( ) =`.

Any other character is a bad character. A token's column is the column of its first character.
Names are case-sensitive: `X` and `x` are different variables.

## Lines

A line that is empty or holds only spaces is blank and is skipped. Every other line is either an
assignment or an expression:

```
line        = assignment | expression
assignment  = name "=" expression
expression  = term { ("+" | "-") term }
term        = unary { ("*" | "/" | "%") unary }
unary       = "-" unary | primary
primary     = number | name | "(" expression ")"
```

`{ ... }` means repeated zero or more times. So `-` in front of a value binds tighter than `*`,
`/` and `%`, which bind tighter than `+` and `-`, and operators of one level group from the left:
`10 - 4 - 3` is `(10 - 4) - 3`, and `-7 / 2` is `(-7) / 2`.

- An assignment gives the name the value of the expression and prints nothing. A name may be
  assigned again; the new value replaces the old one.
- An expression line prints its value in decimal, with a `-` in front when it is negative, and a
  line feed.
- `/` divides and drops any fraction, rounding toward zero: `7 / 2` is 3 and `-7 / 2` is -3.
- `%` gives the remainder that goes with `/`, so that `a` equals `(a / b) * b + a % b`: `7 % -2`
  is 1 and `-7 % 2` is -1.

## Running

Lines run in order. The program stops at the first line with an error: the values printed for
earlier lines stay, the error line is printed, and the program exits with status 20. When no line
has an error, the exit status is 0. An empty file, or one with only blank lines and assignments,
prints nothing. Every printed line, the error line included, ends with a line feed.

## Errors

An error line is written as follows, with `L` the line number and `C` a column, both in decimal:

```
error: line L col C: MESSAGE
```

A line is checked in three stages, and the first stage that finds an error reports it:

1. **Characters.** If the line holds a bad character, the message is `bad character` and `C` is
   the column of the leftmost bad character.
2. **Syntax.** The tokens must form a line of the grammar above. If they do not, the message is
   `syntax error`. `C` is the column of the first token at which the tokens read so far are no
   longer the start of any valid line. When every token keeps them the start of a valid line but
   the line ends too soon, `C` is the length of the line plus one, trailing spaces included, not
   counting its line feed.
3. **Values.** The line is worked out from left to right: an operator's left side, then its right
   side, then the operator itself, and every part is worked out even when its value would not
   change the result. The first problem found is reported:
   - a name with no value yet: the message is `unknown name NAME`, with the name as written, and
     `C` is the name's column. In `n = n + 1` with no earlier `n`, the `n` on the right is unknown.
   - a `/` or `%` whose right side is 0: the message is `division by zero` and `C` is the column
     of that operator.

Example: when the first line of the file is `x = = 3 # 4`, the bad character `#` at column 9 is
reported, not the syntax error at column 5:

```
error: line 1 col 9: bad character
```
