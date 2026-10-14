# Word counts

Print the most frequent words of a text file.

## Arguments

The program is run with exactly two arguments:

1. the name of the input file, which always exists and can be read;
2. `N`, how many words to print.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It may
be empty.

A word is a run of one or more ASCII letters (`A` to `Z`, `a` to `z`) that is as long as possible.
Every other character, digits and apostrophes included, separates words: `don't` is the two words
`don` and `t`, and `x2y` is `x` and `y`. Words are compared without regard to case: `The`, `the`
and `THE` are one word, written `the`.

## Output

Count how often each word occurs. Print one line per word, written as its count, one space and the
word in lower case:

```
3 the
```

- A word with a higher count comes first.
- Words with the same count come in rising ASCII order of their lower-case form.
- Print the first `N` lines of that order. When there are fewer than `N` different words, print
  them all.
- When the file holds no word at all, print the single line `no words`.

Every printed line ends with a line feed. The exit status is 0.

## Errors

`N` is valid when it is made only of the digits `0` to `9` (at least one digit) and its value is
from 1 to 1000. Leading zeros are allowed: `007` means 7. Anything else, such as `0`, `-2`, `+3`,
`1001`, `2x` or an empty argument, is invalid. `N` is checked first; when it is invalid the file
is not read.

When `N` is invalid, print exactly this line, ending with a line feed, and nothing else, and exit
with status 20:

```
error: bad count
```
