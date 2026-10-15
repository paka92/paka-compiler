# Comment stripper

Remove the comments from a C-like source file.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It has
at most 200 lines. Lines are numbered from 1 and columns from 1: the first character of a line is in
column 1. Line and column numbers always refer to the file as it was read, before the joins below.

## Step 1: joins

First, everywhere in the file, every `\` that is directly followed by a line feed is removed
together with that line feed, so the two lines become one. This is done before anything else, so
it also happens inside strings and comments. In a line ending with `\\`, only the last `\` is
directly followed by the line feed; it is removed with the line feed and the other `\` stays. Only
pairs found in the file as read are joined: a `\` that comes before a line feed only because of an
earlier join stays.

The joined text is a sequence of lines: each line ends at a line feed, and the last line may lack
one. An empty text has no lines.

## Step 2: comments

The joined text is read from left to right, starting in code.

- **In code**, `//` starts a line comment, `/*` starts a block comment at depth 1, and `"` or `'`
  starts a string. Every other character is code and is kept.
- **A line comment** runs up to, not including, the next line feed or the end of the text. It is
  removed; the line feed is kept and code goes on after it.
- **A block comment** is removed, the `/*` and `*/` that open and close it included, except that
  every line feed inside it is kept. Inside it, `/*` adds one to the depth and `*/` takes one away;
  when the depth reaches 0 the comment ends and code goes on. Quotes and `//` mean nothing inside a
  block comment.
- **A string** is kept as it is. It ends at the next quote of the same kind as the one that opened
  it (`"` or `'`), and that quote is kept too. Inside it, a `\` and the character after it both
  belong to the string, so `\"`, `\'` and `\\` do not end it. A `\` directly before a line feed or
  the end of the text does not escape it; the string is unterminated. Comment markers inside a
  string are text. A string must end on its line of the joined text.

The markers `//`, `/*` and `*/` are two characters read from left to right, and two markers never
share a character: after a marker, reading goes on after its second character. So in `/*/`, the
`/*` starts a comment and the `/` is inside it, and in `*//*` inside a block comment, `*/` takes
one away from the depth and then `/*` adds one. Outside a block comment, `*/` is plain code.

## Output

After the comments are removed, all spaces at the end of each line are removed. Removing comments
never removes a line, so the result has exactly as many lines as the joined text; a line that held
only a comment or only spaces is printed as an empty line. Print every line of the result, each
followed by a line feed. An empty file prints nothing. The exit status is 0.

Example input:

```
int a; // note
/* x /* y */ z */ int b;
```

Output:

```
int a;
 int b;
```

## Errors

An error prints exactly one line, ending with a line feed, and nothing else, and exits with status
20. `L` is a line number and `C` a column, both in decimal:

```
error: line L col C: MESSAGE
```

Reading stops at the first error met from left to right:

- `unterminated string`: a string reaches a line feed or the end of the text before its closing
  quote. The position is that of its opening quote.
- `unterminated comment`: the text ends inside a block comment. The position is that of the `/` of
  the `/*` that started the comment at depth 1.
