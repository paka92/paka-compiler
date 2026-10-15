# P5 hand working

Not part of the statement. One entry per case: the rule it checks, the value of each top-level
item in order, and the error with its position.

## cases/1

The example. `(sum 10)`: 10 + 9 + ... + 1 + 0 = 55, with 11 calls of `sum` running at the deepest.
Line 4 `(let (x 2 y x) y)`: the item for `y` is the symbol `x` at column 13
(`(let (x 2 y x) y)`: `(` 1, `let` 2 to 4, `(` 6, `x` 7, `2` 9, `y` 11, `x` 13). It is evaluated
with the local names from before the `let`, which are none, and there is no global `x`:
`error: line 4 col 13: unbound x`. Line 5 never runs.

## cases/2

`(twice 4)`: 8. `(twice 1 2)`: two values for one parameter: `error: line 3 col 1: wrong arity`.

## hidden/1: the body sees only its parameters and the globals

- `(f 1)`: the parameter `x` = 1 comes before the global `x` = 10: 2.
- `(g 1)`: `x` is not a parameter of `g`, so the global 10: 11.
- `(let (y 5) (h))`: `y` is local to the `let`, not to `h`'s body; `h` has no parameters and
  there is no global `y`: the error is at the `y` in `h`'s body, line 6 column 13:
  `error: line 6 col 13: unbound y`.

## hidden/2: let

- `(let (x 1) (let (x 2 y x) (+ (* x 10) y)))`: the inner `y` gets `x` from before the inner `let`,
  which is 1; the inner `x` is 2 and replaces the outer one. 2 * 10 + 1 = 21.
- `(let () 7)`: no names: 7.
- `(let (a 1 a 2) a)`: two names the same: `error: line 3 col 1: bad let`.

## hidden/3: what is evaluated, and when

- `(if #t 1 (/ 1 0))`: the else part is not evaluated: 1.
- `(if (< 1 2) #f #t)`: `#f`.
- `(1 (/ 1 0))`: all items are evaluated before the first is checked, so the error is in
  `(/ 1 0)` at column 4, not `not a function`: `error: line 3 col 4: division by zero`.

## hidden/4: functions as values

- `sq`: `<fn sq>`. `+`: the built-in, `<fn +>`.
- `(define + sq)`: the global `+` now holds the function named `sq`. `+` prints `<fn sq>`.
- `(+ 3)`: one value for the one parameter `n`: 9.
- `(define sq 2)`: the global `sq` is now 2; the function held by `+` keeps its body: `(+ 4)` gives
  16.
- `(+ 3163)`: 3163 * 3163 = 10004569 > 9999999, in the body `(* n n)` at line 1 column 16:
  `error: line 1 col 16: overflow`.

## hidden/5: the depth limit, exactly

`(down 49)` runs `down` with 49, 48, ..., 0: 50 calls; when `(down 0)` starts, 49 are running, so
it is allowed: 0. `(down 50)` needs 51: when `(down 0)` is to start, 50 are running. The call is
`(down (- n 1))` at line 1 column 32: `error: line 1 col 32: too deep`.

## hidden/6: define

`(define (f) (define y 1))`: a function with no parameters; its body is not evaluated now, so the
inner `define` is no error yet. `f`: `<fn f>`. `(define g)`: two items: `error: line 3 col 1: bad
define`.

## hidden/7: unclosed list

Reading reaches the end with `(define` (line 2 column 1) and `(+ x` (line 3 column 3) still open;
the first of them is reported: `error: line 2 col 1: unclosed (`. Reading failed, so `(+ 1 2)` on
line 1 is not run and prints nothing.

## hidden/8: reading order

Line 2 `(+ x -2-))`: `-2-` starts with `-` and a digit and is not a number: a bad atom at column 6,
met before the extra `)` at column 10: `error: line 2 col 6: bad atom`. No final line feed. Line 1
is not run.
