# T1 hand working

Not part of the statement. One entry per case. For each line: the words with their lengths, the
fill test, and for a justified line `L` (letters), `S = W - L`, `g` gaps and the gap sizes. The
second check is that every justified line adds up to `W`.

## cases/1 (`pangrams.txt 20`)

Paragraph 1: The 3, quick 5, brown 5, fox 3, jumps 5, over 4, the 3, lazy 4, dog. 4.

- `The quick brown fox` is 19; adding jumps gives 25 > 20. L = 16, S = 4, g = 3: 4/3 = 1 rem 1, gaps
  2 1 1: `The  quick brown fox` = 3+2+5+1+5+1+3 = 20.
- `jumps over the lazy` is 19; adding dog. gives 24. L = 16, S = 4, g = 3, gaps 2 1 1:
  `jumps  over the lazy` = 5+2+4+1+3+1+4 = 20.
- Last line `dog.`.

Paragraph 2: Pack 4, my 2, box 3, with 4, five 4, dozen 5, liquor 6, jugs. 5.

- `Pack my box with` is 16; adding five gives 21. L = 13, S = 7, g = 3: 2 rem 1, gaps 3 2 2:
  `Pack   my  box  with` = 4+3+2+2+3+2+4 = 20.
- `five dozen liquor` is 17; adding jugs. gives 23. L = 15, S = 5, g = 2: 2 rem 1, gaps 3 2:
  `five   dozen  liquor` = 4+3+5+2+6 = 20.
- Last line `jugs.`.

One empty line between the paragraphs.

## cases/2 (`note.txt 30`)

One paragraph over three input lines; leading and repeated spaces are ignored. Wrap 4, these 5,
words 5, into 4, lines 5, that 4, fit 3, thirty 6, columns 7, exactly, 8, please. 7.

- `Wrap these words into lines` is 27; adding that gives 32 > 30. L = 23, S = 7, g = 4: 1 rem 3,
  gaps 2 2 2 1: `Wrap  these  words  into lines` = 4+2+5+2+5+2+4+1+5 = 30.
- `that fit thirty columns` is 23; adding exactly, gives 32. L = 20, S = 10, g = 3: 3 rem 1, gaps
  4 3 3: `that    fit   thirty   columns` = 4+4+3+3+6+3+7 = 30.
- Last line `exactly, please.` (8 + 1 + 7 = 16).

## hidden/1 (`empty.txt 00`)

`00` is digits only, value 0, below 1. W is checked before the file, so the error wins over the
empty output: `error: bad width`, status 20.

## hidden/2 (`text.txt 0101`)

Digits only, leading zero dropped, value 101, above 100: `error: bad width`, status 20.

## hidden/3 (`empty.txt 10`)

Empty file, no word: nothing printed, status 0.

## hidden/4 (`long.txt 5`)

Paragraph 1: `abcdefghijkl` (12) is cut into `abcde`, `fghij`, `kl`; `xy` (2); `abcde` (5, exactly W,
not cut). Fill: `abcde` + `fghij` = 11 > 5, so `abcde` alone (single word, as is); `fghij` alone;
`kl xy` = 5 fits, adding `abcde` = 11 does not. `kl xy`: L = 4, S = 1, g = 1, gap 1, so `kl xy`.
Last line `abcde`.

Paragraph 2: `zz` (2), `0123456789` (10) cut into `01234` and `56789` (10 is a multiple of 5, so
no short piece). `zz` + `01234` = 8 > 5: `zz` alone; `01234` alone; last line `56789`.

## hidden/5 (`narrow.txt 1`)

No final line feed. Words a, bc, d; `bc` (2 > 1) is cut into `b`, `c`. W = 1: every line holds one
word: `a`, `b`, `c`, `d`. Single words are not padded.

## hidden/6 (`blanks.txt 10`)

Lines: empty, `   `, `one two`, empty, ` `, empty, `three`, `   `. Blank lines only separate, so
the paragraphs are `one two` and `three`. Each is a single last line: `one two`, then one empty line,
then `three`. No empty line before or after.

## hidden/7 (`spread.txt 16`)

I 1, am 2, a 1, fan 3, of 2, long 4, lines 5, of 2, text 4.

- `I am a fan of` is 13; adding long gives 18 > 16. L = 9, S = 7, g = 4: 1 rem 3, gaps 2 2 2 1:
  `I  am  a  fan of` = 1+2+2+2+1+2+3+1+2 = 16.
- `long lines of` is 13; adding text gives 18. L = 11, S = 5, g = 2: 2 rem 1, gaps 3 2:
  `long   lines  of` = 4+3+5+2+2 = 16.
- Last line `text`.

## hidden/8 (`exact.txt 07`)

`07` is valid, W = 7. abc 3, defg 4, hi 2, jklmnop 7 (exactly W, not cut), q 1.

- `abc` + `defg` = 8 > 7: `abc` alone, a single word on a non-last line, printed with no padding.
- `defg hi` = 7 exactly; adding jklmnop gives 15. L = 6, S = 1, g = 1: `defg hi`.
- `jklmnop` + `q` = 9 > 7: `jklmnop` alone.
- Last line `q`.
