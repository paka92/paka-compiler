# Natural sort

Sort the lines of a text file so that runs of digits compare as numbers and letter case is ignored.

## Arguments

The program is run with exactly two arguments: the name of the input file, then the order, which
must be exactly `up` or `down`. The file always exists and can be read.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines; a file holding only a line feed has one empty line. Lines are numbered from 1. The file
has at most 200 lines of at most 200 characters. An empty line, one with no characters at all, is
sorted and printed like any other line.

## Chunks

Each line is cut into chunks from left to right: every longest run of digits (`0` to `9`) is a
number chunk, and every longest run of other characters is a text chunk. So `ab12c-7` has the
chunks `ab`, `12`, `c-` and `7`, and an empty line has no chunk. The value of a number chunk is the
decimal number it writes: `007` and `7` both have the value 7.

## Comparing two lines

Compare the chunks of the two lines in pairs: the first chunk of each, then the second of each, and
so on. The first pair that is not equal decides:

- Two number chunks: the line with the smaller value comes first. Equal values are an equal pair.
- A number chunk and a text chunk: the line with the number chunk comes first.
- Two text chunks: change every upper-case letter `A` to `Z` into its lower-case letter in both,
  then compare them character by character by ASCII code. At the first place where they differ,
  the line with the smaller code comes first. When one chunk is the start of the other, the line
  with the shorter chunk comes first. Chunks that are the same after the change are an equal pair.

When one line runs out of chunks and every pair so far was equal, the line with fewer chunks comes
first. Two lines with the same number of chunks and only equal pairs are equal.

## Output

With `up`, print the lines in this order: every line that comes first in a comparison is printed
before the other. Equal lines keep their order from the input.

With `down`, the order is turned around: every line that comes first in a comparison is printed
after the other. Equal lines still keep their order from the input; they are not turned around.

Every line is printed exactly as it is in the input, with a line feed after it. The exit status is
0.

Example: the input lines are `item 10`, `Item 9`, `item 9` and `item`. The line `item` comes first
because its only chunk `item` is the start of the chunk `item ` of the others. `Item 9` and
`item 9` are equal. With `up` the output is:

```
item
Item 9
item 9
item 10
```

With `down` the output is:

```
item 10
Item 9
item 9
item
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. The checks are made in this order and only the first that fails is reported:

1. The order is not exactly `up` or `down`: `error: bad order`. This is checked before the file is
   read.
2. A number chunk has more than 15 digits, leading zeros included:
   `error: line N: number too long`, for the first such line, with `N` its line number in decimal.
   For example, a line `id 1234567890123456` has a chunk of 16 digits.

With at most 15 digits every value is below 10^15.
