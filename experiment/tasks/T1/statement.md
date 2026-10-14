# Wrap and justify

Wrap the paragraphs of a text file to a given width and justify them.

## Arguments

The program is run with exactly two arguments:

1. the name of the input file, which always exists and can be read;
2. `W`, the line width.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines.

- A blank line is a line that is empty or holds only spaces.
- A paragraph is a group of non-blank lines that follow one another, as large as possible. Blank
  lines only separate paragraphs; how many there are, and where, does not matter.
- A word is a run of characters other than the space, as long as possible. A paragraph's words are
  taken in order across all its lines. Line breaks and spaces in the input are otherwise ignored.

## Wrapping

Each paragraph is wrapped on its own, in three steps.

1. **Cut long words.** A word longer than `W` characters is cut into pieces of `W` characters
   from its left end; the last piece holds what is left and may be shorter. Each piece is then a
   word of its own. A word of exactly `W` characters is not cut.
2. **Fill lines.** A line starts with the next word. The following word is added, after one space,
   as long as the line's length then stays at most `W`. When it would exceed `W`, a new line starts
   with that word.
3. **Justify.** Every line of the paragraph except its last is widened to exactly `W` characters:
   - Let the line have `n` words, `n` at least 2, of total length `L`. It has `g = n - 1` gaps
     between words and `S = W - L` spaces to place in them. Each gap gets `S / g` spaces, rounded
     down, and the first `S mod g` gaps, counted from the left, get one space more.
   - A line with a single word is printed as that word alone, with no spaces added.

   The last line of a paragraph is its words joined by single spaces.

So no printed line starts or ends with a space.

## Output

Print the wrapped paragraphs in order, with exactly one empty line between two paragraphs and no
empty line before the first or after the last. Every printed line ends with a line feed. When the
file holds no word, print nothing. The exit status is 0.

Example with `W` = 20, a paragraph whose first two lines are justified:

```
The  quick brown fox
jumps  over the lazy
dog.
```

## Errors

`W` is valid when it is made only of the digits `0` to `9` (at least one digit) and its value is
from 1 to 100. Leading zeros are allowed: `007` means 7. Anything else, such as `0`, `-5`, `+5`,
`101`, `2x` or an empty argument, is invalid.

When `W` is invalid, print exactly this line, ending with a line feed, and nothing else, and exit
with status 20:

```
error: bad width
```
