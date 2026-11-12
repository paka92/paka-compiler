# Tiny Lisp

Run a program written in a small Lisp and print the value of each top-level expression.

## Arguments

The program is run with exactly one argument: the name of the program file, which always exists
and can be read.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). Lines are
numbered from 1. Columns are numbered from 1: the first character of a line is in column 1. The
file has at most 200 lines of at most 200 characters, and lists in it nest at most 20 deep.

## Reading

The file is read from left to right. Spaces and line feeds separate things and are otherwise
skipped. A `;` starts a comment that runs to the end of its line; the comment is skipped. `(`
opens a list and `)` closes the list opened last. Any other character starts an atom: the longest
run of characters that holds no space, line feed, `(`, `)` or `;`. The position of an atom or a
list is the line and column of its first character; for a list that is its `(`.

An atom is one of:

- a number: an optional `-`, then 1 to 7 digits, and nothing else (`007` is 7, `-0` is 0);
- `#t` (true) or `#f` (false);
- a bad atom: any other atom that starts with `#`, with a digit, or with `-` and then a digit,
  such as `#x`, `12345678`, `1a` or `-2-`;
- a symbol: any other atom, such as `x`, `+`, `-`, `<=`, `empty?` or `--1`.

What is read is a sequence of top-level items, each an atom or a list, and a list holds a sequence
of items. Reading fails at the first of these met from left to right:

- `bad atom`, at the atom;
- `unexpected )`, at a `)` with no open list;
- `unclosed (`, when the end of the file is reached with a list still open, at the first `(` of
  the file that is still open.

When reading fails, print exactly `error: line L col C: WHAT` with the position and the error, and
nothing else, and exit with status 20. Nothing is run.

## Values and names

A value is a whole number, a truth value (`#t` or `#f`) or a function. Functions are the six
built-in functions `+ - * / < =`, named by those symbols, and the functions the program defines.

A name is a symbol other than `if`, `let` and `define`. There are global names, which `define`
gives values, and local names, which a function's parameters and `let` give values. A global name
keeps its value until a later `define` of the same name replaces it.

## Running

The top-level items run in order. A top-level list whose first item is the symbol `define` is a
definition and prints nothing; it must be one of:

- `(define NAME EXPR)`: evaluate `EXPR` with no local names, and give the global `NAME` that value.
- `(define (NAME PARAM ...) BODY)`: give the global `NAME` a new function named `NAME` with the
  parameters `PARAM ...` (none or more, no two the same) and the body `BODY`, one item. `BODY` is
  not evaluated now.

Any other form of a list starting with `define`, at the top level or anywhere else, is the error
`bad define`. Every other top-level item is evaluated with no local names, and its value is
printed and followed by a line feed: a number in decimal with a `-` in front when it is negative,
a truth value as `#t` or `#f`, and a function as `<fn NAME>` with its name, as in `<fn +>`.

Evaluating an item:

- A number or a truth value gives itself.
- A symbol gives the value of the local name of that name, else of the global name, else the
  built-in function named by it. When there is none, the error is `unbound NAME`, at the symbol.
- `()` is the error `empty call`.
- `(if C T E)`, a list of exactly four items whose first is `if`: evaluate `C`; when it gives `#t`,
  evaluate `T` and give its value, and when `#f`, `E`; the other is not evaluated. When `C` gives
  something else, the error is `type error`. A list starting with `if` of any other length is the
  error `bad if`.
- `(let (NAME1 EXPR1 NAME2 EXPR2 ...) BODY)`, a list of exactly three items whose first is `let`
  and whose second is a list of names and items in turn, with no two names the same: evaluate
  `EXPR1`, `EXPR2` and so on in order, all with the local names as they were before the `let`;
  then evaluate `BODY` with the local names as before plus each `NAME` given its value, a `NAME`
  replacing a local name of the same name. Any other form of a list starting with `let` is the
  error `bad let`, found before anything in it is evaluated. A `let` with no names is allowed.
- Any other list is a call. Evaluate all its items in order, the first included. The first must
  give a function, else the error is `not a function`. Then:
  - A defined function needs as many values after the first as it has parameters, else the error
    is `wrong arity`. When 50 calls of defined functions are already running, the error is
    `too deep`. Otherwise its body is evaluated with its parameters, given those values, as the
    only local names, and the call gives that value.
  - A built-in function needs exactly two values after the first, else `wrong arity`. `=` takes
    two numbers or two truth values and gives `#t` when they are equal, else `#f`. `<` takes two
    numbers and gives whether the first is less. `+`, `-` and `*` give the sum, difference and
    product of two numbers; `/` divides the first number by the second, dropping any fraction,
    rounding toward zero: `(/ -7 2)` gives -3. Any other values are the error `type error`. `/`
    by 0 is the error `division by zero`. A result of `+`, `-`, `*` or `/` that is less than
    -9999999 or greater than 9999999 is the error `overflow`.

Unless stated otherwise above, an error is at the position of the list being evaluated. The first
error ends the run: the lines printed so far stay, then `error: line L col C: WHAT` is printed with
the position and the error, and the exit status is 20. When no error occurs, the exit status is 0.

## Example

The program

```
; sum of 1 to n
(define (sum n) (if (< n 1) 0 (+ n (sum (- n 1)))))
(sum 10)
(let (x 2 y x) y)
sum
```

prints

```
55
```

and then `error: line 4 col 13: unbound x`, with exit status 20: `y` is given the value of `x` as
it was before the `let`, and there was no `x` then.
