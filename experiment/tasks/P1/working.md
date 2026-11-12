# P1 hand working

Not part of the statement. One entry per case: each line's value or error, with columns counted
character by character. Second check: each division is checked as `a = (a / b) * b + a % b`.

## cases/1 (`prog.txt`)

1. `x = 7`: x is 7.
2. `y = x * 2 + 1`: (7 * 2) + 1 = 15.
3. `y`: prints 15.
4. `(x + y) / 4`: 22 / 4 = 5.5, toward zero 5. Check: 5 * 4 + 2 = 22. Prints 5.

## cases/2 (`rate.txt`)

1. `2 + 3 * 4` = 2 + 12 = 14.
2. `10 - 4 - 3` = (10 - 4) - 3 = 3.
3. `rate = 17`.
4. `rate % 5 * 2` = (17 % 5) * 2 = 2 * 2 = 4.
5. `-rate / 3` = (-17) / 3 = -5.67, toward zero -5. Check: -5 * 3 + (-2) = -17.

Prints 14, 3, 4, -5.

## hidden/1 (`div.txt`)

1. a = 5. 2. `a + 1` prints 6.
3. `b = a / (a - 5)`: columns b 1, space 2, `=` 3, space 4, a 5, space 6, `/` 7. Left a = 5, right
   5 - 5 = 0, so `/` divides by zero: `error: line 3 col 7: division by zero`, status 20. Line 4
   never runs.

## hidden/2 (`order.txt`)

1. `   ` is blank, skipped (still line 1).
2. `1 - -2` = 1 - (-2) = 3, printed.
3. `  ` is blank.
4. `0 * (1 % 0) + zz`: columns `0` 1, `*` 3, `(` 5, `1` 6, `%` 8, `0` 10, `)` 11, `+` 13, `zz` 15.
   The term `0 * (1 % 0)` is worked out first; its right side `1 % 0` fails although the left
   side is 0: `error: line 4 col 8: division by zero`. The unknown `zz` is never reached.

## hidden/3 (`names.txt`)

1. v = 2. 2. `v = v * v`: the right side uses the old v, 4; v becomes 4. 3. `v` prints 4.
4. `V` (column 1) has no value; names are case-sensitive: `error: line 4 col 1: unknown name V`.

## hidden/4 (`chars.txt`)

1. `12` prints 12.
2. `y = (4 + ) & 1`: columns y 1, `=` 3, `(` 5, `4` 6, `+` 8, `)` 10, `&` 12. The syntax error
   would be at `)` (column 10), but stage 1 finds the bad character `&` first:
   `error: line 2 col 12: bad character`.

## hidden/5 (`open.txt`)

1. `-(2 - 9) % 4`: unary minus on (2 - 9) = -7 gives 7; 7 % 4 = 3. Check: (7 / 4) * 4 + 3 = 4 + 3
   = 7. Prints 3.
2. `7 * (2 + 3  ` ends with two spaces; its length is 12. The tokens `7 * ( 2 + 3` are the start of
   a valid line (a `)` could follow) but the line ends: column 12 + 1 = 13:
   `error: line 2 col 13: syntax error`.

## hidden/6 (`chain.txt`)

1. y = 1.
2. `x = y = 3`: columns x 1, `=` 3, y 5, `=` 7. `x = y` is a valid start (it is a whole
   assignment); no expression contains `=`, so the second `=` cannot follow:
   `error: line 2 col 7: syntax error`.

## hidden/7 (`signs.txt`)

1. `-9 / 4` = -2.25, toward zero -2.
2. `9 / -4` = -2.25, toward zero -2 (unary minus after `/` is allowed: `term = unary {op unary}`).
3. `-9 % 4`: -9 - (-2 * 4) = -9 + 8 = -1.
4. `9 % -4`: 9 - (-2 * -4) = 9 - 8 = 1.
5. `--5 - 005`: -(-5) = 5, minus 5 = 0.
6. `(((4)))*-1` = 4 * (-1) = -4.
7. `4503599627370495 * 2 + 1`: 4503599627370495 * 2 = 9007199254740990, + 1 = 9007199254740991,
   the largest value allowed; nothing on the way is larger.

Prints -2, -2, -1, 1, 0, -4, 9007199254740991; status 0.

## hidden/8 (`empty.txt`)

Empty file: no line, nothing printed, status 0.
