# X1 hand working

Not part of the statement. One entry per case: the input, how each expected line follows from the
statement, and a second check.

## cases/1 (`text.txt 3`)

Words: the cat sat on the mat the dog sat too. Counts: the 3, sat 2, cat 1, dog 1, mat 1, on 1,
too 1. Order: 3 the; 2 sat; the 1s in ASCII order cat, dog, mat, on, too. First 3: `3 the`, `2 sat`,
`1 cat`. Check: 10 words in the file, counts add to 3 + 2 + 5 = 10.

## cases/2 (`notes.txt 10`)

`Apple banana, apple! Cherry? banana; APPLE cherry date.` Punctuation separates. Lower case: apple
banana apple cherry banana apple cherry date. Counts: apple 3, banana 2, cherry 2, date 1. The 2s:
banana before cherry. N = 10 is more than 4 words, so all four lines. Check: 8 words, 3 + 2 + 2 + 1 = 8.

## hidden/1 (`empty.txt 5`)

Empty file, no word: `no words`, status 0.

## hidden/2 (`marks.txt 1`)

`123 456 -- !!!` and `... ,` hold no letter: `no words`, status 0.

## hidden/3 (`text.txt 0`)

`0` is digits only but below 1: `error: bad count`, status 20.

## hidden/4 (`text.txt +3`)

`+` is not a digit: `error: bad count`, status 20.

## hidden/5 (`text.txt 1001`)

Digits only, value above 1000: `error: bad count`, status 20.

## hidden/6 (`split.txt 002`)

`002` is valid, N = 2. Line 1 `don't stop2stop DON'T`: don, t, stop, stop, don, t (the apostrophe and
the `2` separate). Line 2 `it's`: it, s. Counts: don 2, stop 2, t 2, it 1, s 1. The 2s in ASCII
order: don, stop, t. First 2: `2 don`, `2 stop`. Check: 8 words, 2 + 2 + 2 + 1 + 1 = 8.

## hidden/7 (`greek.txt 1000`)

No final line feed. Lower case: zeta alpha beta zeta alpha beta gamma. Counts: alpha 2, beta 2, zeta
2, gamma 1. The 2s in ASCII order: alpha, beta, zeta. Four words, N = 1000, so all. Check: 7 words,
2 + 2 + 2 + 1 = 7.

## hidden/8 (`cut.txt 2`)

d c b a e e: e 2; a, b, c, d 1 each. Order: 2 e, then 1 a, 1 b, 1 c, 1 d. First 2 cut the tie of 1s
after its first word: `2 e`, `1 a`.
