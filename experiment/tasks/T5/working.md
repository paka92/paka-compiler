# T5 hand working

Not part of the statement. One entry per case: the keys, each line's key texts (L1, L2 and so on),
then the order. Second check: the output holds every input line once.

## cases/1 (`fruit.txt`, `2n-,1i`)

Keys: field 2 numeric turned around; field 1 case ignored. Field 2: L1 3, L2 10, L3 `x` (not a
number), L4 10.0, L5 3. Rising by value: `x` first, then 3 (L1, L5), then 10 (L2, L4); turned
around: 10, then 3, then `x`. Among 10: `apple` and `apple` after lowering are equal, so input
order L2, L4. Among 3: `banana` < `pear`: L5, L1. Output as in the statement.

## cases/2 (`2,1x`)

Key 1 `2` is good. Key 2 `1x`: after the field `1`, `x` is not `.`, `n`, `i` or `-`:
`error: bad key 2`, status 20. The file is not read.

## hidden/1 (`names.txt`, `1-`)

Field 1: `bob`, `ann`, `bob`, `bo`, `ann`, `bobby`. Rising: `ann` (L2, L5) < `bo` < `bob` (L1, L3)
< `bobby` (`bo` is the start of `bob`, `bob` of `bobby`, so shorter first). Turned around:
`bobby`, `bob`, `bo`, `ann`. Equal lines keep input order even under `-`: L1 before L3, L2 before
L5. Output: `bobby,6`, `bob,1`, `bob,3`, `bo,4`, `ann,2`, `ann,5`.

## hidden/2 (`nums.txt`, `1n`)

Not numbers: `+1` (plus sign), `1e3` (letter), ` 5` (space), `1.` (no digit after the point),
`.5` (no digit before it), `1234567890` (10 digits), `0.1234567` (7 digits after the point), `-`
(no digit). They come first, all equal, in input order: c, f, h, j, k, m, o, p.

Numbers: `2.50` = 2.5, `-0` = 0, `2.5`, `0.000` = 0, `-1.25`, `-1.5`, `0`, `123456789`. By value:
-1.5 (i), -1.25 (g), then 0 for b, e, l in input order, then 2.5 for a, d in input order, then n.

## hidden/3 (`codes.txt`, `2.3n,1`)

Field 2 without its first 2 characters:

- L1 `ab12`: `12`, value 12.
- L2 `x`: 1 character, fewer than 3: empty, not a number.
- L3 `ab-3`: `-3`.
- L4 `zz12.0`: `12.0`, value 12.
- L5 `ab`: 2 characters, fewer than 3: empty, not a number.
- L6 `AB7`: `7`.

Not numbers first: L2 (`d`), L5 (`a`); key 2 (field 1 as text): `a` < `d`: L5, L2. Then -3 (L3),
7 (L6), then 12 for L1 (`e`) and L4 (`b`): key 2 puts `b` first: L4, L1.

Output: `a,ab`, `d,x`, `c,ab-3`, `f,AB7`, `b,zz12.0`, `e,ab12`.

## hidden/4 (`short.txt`, `3,1`)

Field 3: L1 `b`; L2 `y` has one field: empty; L3 is empty: empty; L4 `a`; L5 `w,3,` has an empty
third field; L6 `a` (the fourth field `extra` is not used). Empty first: L2, L3, L5, ordered by
field 1: `` (L3) < `w` (L5) < `y` (L2). Then `a`: field 1 `v` (L6) < `x` (L4). Then `b`: L1.

Output: the empty line, `w,3,`, `y`, `v,4,a,extra`, `x,2,a`, `z,1,b`.

## hidden/5 (`case.txt`, `1i,1`)

Lowered: `apple`, `zed`, `_x`, `apple`, `zed`, `apple`, `[`. By code: `[` 91 < `_` 95 < `a` 97 <
`z`. (Changing to upper case instead would put `[` and `_` after the letters.) Ties broken by the
plain text: `APPLE` < `Apple` < `apple` (`P` 80 < `p` 112, then `A` 65 < `a` 97); `Zed` < `zed`.

Output: `[`, `_x`, `APPLE`, `Apple`, `apple`, `Zed`, `zed`.

## hidden/6 (`1,2.5n-,1.0`)

Keys 1 and 2 are good. Key 3 `1.0`: `START` written `0` is not 1 to 99: `error: bad key 3`.

## hidden/7 (`1i,`)

Split at every comma: `1i` and the empty text. The empty key is not written as a key:
`error: bad key 2`.

## hidden/8 (`score.txt`, `2n-,1`)

No line feed at the end. Field 2: `dan` 7, `cat` `n/a` (not a number), `bea` 7.0, L4 empty line:
no field 2, empty, not a number; `alf` -2, `eve` empty, not a number; `abe` 7; `zed` 12.

Turned around: numbers from the largest, then the texts that are not numbers. 12 (`zed`); 7 for
`dan`, `bea`, `abe`, ordered by field 1 rising: `abe`, `bea`, `dan`; -2 (`alf`); then not numbers
ordered by field 1: `` (the empty line), `cat`, `eve`.

Output: `zed,12`, `abe,7`, `bea,7.0`, `dan,7`, `alf,-2`, the empty line, `cat,n/a`, `eve,`.
