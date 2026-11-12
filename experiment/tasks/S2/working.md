# S2 hand working

Not part of the statement. One entry per case: each line's effect in cents, as balance / held per
account, with the first reason that applies. Second check: at the end, each balance equals its
deposits minus its captures, minus transfers out, plus transfers in; each held sum equals the active
holds left.

## cases/1 (`book.txt`)

Every line is well formed.

1. open ann. 2. open bob. 3. ann 10000 / 0.
4. hold ann t1 3050: available 10000, enough. ann 10000 / 3050.
5. transfer ann bob 8000: both open, not the same; available 10000 - 3050 = 6950 < 8000:
   `line 5: insufficient funds`.
6. capture t1 2025: t1 active, 2025 <= 3050. ann 7975 / 0; t1 ends.
7. transfer ann bob 6000: available 7975. ann 1975, bob 6000.
8. release t1: t1 has ended: `line 8: no hold t1`.
9. deposit cat: `line 9: no account cat`.

End: ann `19.75` / `0.00`, bob `60.00` / `0.00`; a < b. Check: ann 10000 - 2025 - 6000 = 1975.

## cases/2 (`typo.txt`)

Line 2 (`open dan` again) is well formed; it would be rejected, but nothing runs. Line 3
`deposit dan 12.5` has one digit after the point: `error: line 3: bad command`, status 20.

## hidden/1 (`empty.txt`)

No lines, no account: `no accounts`, status 0.

## hidden/2 (`zero.txt`)

Lines 1 to 3 are well formed, line 4 is empty and skipped. Line 5 `transfer eve eve 0.00`: the amount
`0.00` is below 0.01, so the line is malformed (the same-account rule never comes into play):
`error: line 5: bad command`. The rejection line 3 would print is not printed, because nothing runs.

## hidden/3 (`spaces.txt`)

Line 2 `deposit zed 0007.05` is well formed (four digits, leading zeros allowed). Line 3 holds three
spaces: it is not empty, so it must be a command, and it is not: `error: line 3: bad command`.

## hidden/4 (`holds.txt`)

1. open fay. 2. fay 1000 / 0. 3. hold a 600: available 1000. fay 1000 / 600.
4. hold fay a 100: account open; id a used by the hold of line 3: `line 4: hold exists a`.
5. hold fay b 401: b unused; available 1000 - 600 = 400 < 401: `line 5: insufficient funds`.
6. hold fay b 400: b still unused (the rejected hold used nothing); 400 <= 400. fay 1000 / 1000.
7. release a: fay 1000 / 400.
8. hold fay a 100: a was used by a hold that succeeded, though it has ended: `line 8: hold exists a`.
9. hold gus a 9900: gus is not open, checked before the id: `line 9: no account gus`.
10. capture b 400: fay 600 / 0.

End: `fay: balance 6.00, held 0.00`. Check: 1000 - 400 = 600.

## hidden/5 (`moves.txt`)

1, 2: open hal, ivy. 3: hal 5000.
4. transfer max hal: FROM max not open: `line 4: no account max`.
5. transfer hal max: FROM open, TO max not: `line 5: no account max`.
6. transfer zoe max: both missing, FROM checked first: `line 6: no account zoe`.
7. transfer hal hal 9900: same account is checked before funds (9900 > 5000 too): `line 7: same
   account`.
8. hold hal p 2000: hal 5000 / 2000.
9. transfer hal ivy 3001: available 3000 < 3001: `line 9: insufficient funds`.
10. transfer hal ivy 3000: hal 2000 / 2000, ivy 3000.

End: hal `20.00` held `20.00`; ivy `30.00` held `0.00`; h < i. Check: 5000 - 3000 = 2000.

## hidden/6 (`capture.txt`)

1. open jo. 2. jo 7. 3. jo 7 + 193 = 200. 4. hold k 150: jo 200 / 150.
5. capture k 151 > 150: `line 5: capture exceeds hold`.
6. capture q 9900: no hold q, checked before the amount: `line 6: no hold q`.
7. capture k 5: jo 195 / 0; k ends and the other 145 cents are no longer held.
8. capture k 1: k has ended: `line 8: no hold k`.
9. hold jo m 195: available 195. jo 195 / 195.
10. open jo: `line 10: account exists jo`.
11. capture m 190: jo 5 / 0.

End: 5 cents is `0.05`: `jo: balance 0.05, held 0.00`. Check: 200 - 5 - 190 = 5.

## hidden/7 (`big.txt`)

The last line has no line feed. Accounts zed, amy, b, al. amy 99999999999 + 99999999999 =
199999999998. transfer 99: amy 199999999899, zed 99. al 10. b 0.

Order: al < amy (`l` 108 < `m` 109) < b < zed. Money: 199999999899 cents is `1999999998.99`; 10 is
`0.10`; 0 is `0.00`; 99 is `0.99`.

## hidden/8 (`word.txt`)

Line 2 `release` has no id field: `error: line 2: bad command`. Line 3 (`withdraw`, no such
command) is malformed too, but only the first is reported.
