# S3 hand working

Not part of the statement. One entry per case: the joins, then each line of the joined text read
character by character with the state, then the end spaces removed. Second check: the number of
printed lines equals the number of lines of the joined text, and every error column was counted
again by finding the character's offset in the line as read.

## cases/1 (`main.c`)

No `\` before a line feed, so no join. 8 lines.

1. `#include <stdio.h> // io`: code up to the space before `//`; the comment goes. End space removed:
   `#include <stdio.h>`.
2. `/* header`: block comment at depth 1; the text goes, the line feed stays: empty line.
3. `   comment */`: still in the comment; `*/` ends it at depth 0: empty line.
4. `int main(void) {` unchanged.
5. The string `"// not a comment /* either */\n"` is kept whole (`\n` is a `\` and an `n`, both
   kept). `);` stays, ` /* trailing */` goes, then the end space: `  printf("// not a comment /*
   either */\n");` (one line).
6. `'"'` is a string opened by `'`; the `"` inside does not end it. ` // quote char` goes: `  char q =
   '"';`.
7. `/* outer /* inner */ still outer */`: depth 1, 2, 1, 0. Removed with the space before it:
   `  return 0;`.
8. `}`.

## cases/2 (`open.c`)

Line 1: `x`1 ` `2 `=`3 ` `4 `1`5 `;`6 ` `7 `/`8: a block comment starts at depth 1. Line 2:
`/* nested */` goes to depth 2 and back to 1; ` still open` and the end of the text are inside it:
`error: line 1 col 8: unterminated comment`, status 20.

## hidden/1 (`join.c`)

Joins: line 1 ends `/\` and line 3 ends `tail \` (a space, then `\`). Joined lines:

- `a = 1; /` + `/ comment made by a join` = `a = 1; // comment made by a join` -> `a = 1;`
- `c = 3; // tail ` + `d = 4;` = `c = 3; // tail d = 4;`; the comment holds `d = 4;` -> `c = 3;`
- `e = 5;`

3 joined lines, 3 printed.

## hidden/2 (`strings.c`)

Line 1 is `s = "one\\` then a line feed. Only the second `\` is directly before the line feed; it is
removed with it. Joined: `s = "one\` + `two";` = `s = "one\two";`. In the string, `\t` is a `\` and
a `t`, both kept, then `wo` and the closing `"`.

`t = 'x';` unchanged. `u = "a\"//b"; // c`: in the string `\"` does not end it and `//` is text;
the `"` after `b` closes it; ` // c` goes: `u = "a\"//b";`.

## hidden/3 (`bad.c`)

Line 1: `'a'` is a string; in `/* don't */` the `'` means nothing: `ok = 'a';`. Line 2: `b`1 `a`2
`d`3 ` `4 `=`5 ` `6 `"`7 opens a string; `//` inside is text; the line feed comes first:
`error: line 2 col 7: unterminated string`, status 20.

## hidden/4 (`long.c`)

Line 2 `msg = \` ends with `\`, line 3 `  "start \` ends with `\`: both join. Joined line 2 is
`msg =   "start middle`. The `"` was read at line 3, column 3 (two spaces before it). The string
reaches the line feed after `middle` (the end of line 4 as read):
`error: line 3 col 3: unterminated string`. Line 1 `p = 1;` would print but nothing is printed on an
error.

## hidden/5 (`marks.c`)

1. `a /*/ b */ c`: `/*` opens; the next `/` is followed by a space, so it is not a marker; ` b `
   inside; `*/` closes. Kept `a ` and ` c`: `a  c` (two spaces).
2. `d /**/e`: `/*` opens, then `*/` closes at once: `d e`.
3. `f */ g`: not in a comment, so `*/` is code: `f */ g`.
4. `h /* x *//* y */ i`: `/*` opens, ` x `, `*/` closes; the next two characters `/*` open again (the
   `/` of `*/` is not reused for `//`), ` y `, `*/` closes: `h ` + ` i` = `h  i`.

## hidden/6 (`nest.c`)

1. `x ` kept; `/*` depth 1; `"` means nothing; `/*` depth 2; `'` means nothing; `*/` depth 1;
   ` still `; `*/` depth 0; ` y` kept: `x  y`.
2. `/* a // b */`: `//` means nothing inside; `*/` closes; ` z` kept: ` z` (a start space stays).

## hidden/7 (`only.c`)

Bytes: three spaces, a line feed, `// only /* not open` with no line feed. Two lines. Line 1: end
spaces removed, empty. Line 2: a line comment to the end of the text; the `/*` in it starts nothing,
so no comment is left open. Empty. Output: two empty lines, `\n\n`, status 0.

## hidden/8 (`deep.c`)

Line 1: `"/*"` is a string, so `/*` is text. Line 2: `/*` at col 1 opens depth 1; `/* two */` goes to
2 and back to 1; ` three`, line 3 and the end of the text are inside:
`error: line 2 col 1: unterminated comment`.
