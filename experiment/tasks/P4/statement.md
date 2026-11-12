# Register machine

Assemble a small program for a machine with six registers, then run it within a step limit.

## Arguments

The program is run with exactly two arguments: the name of the program file, which always exists
and can be read, then the step limit `LIMIT`.

`LIMIT` must be a whole number from 1 to 100000 written in decimal digits, the first of which is
not `0`. When it is not, print `error: bad limit` and nothing else, and exit with status 20,
without reading the file.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, every line counted. The file has at most 200 lines of at most
200 characters.

## Lines

On every line, a `;` and everything after it is a comment and is dropped. What is left is split
into words at spaces; a run of spaces counts as one break, and spaces at the start and end are
ignored. So a line may hold no words.

When the first word ends with `:`, it defines a label, and the label's name is that word without
the `:`. The words after a label, or all the words when there is no label, are an instruction:
its name, then its operands. A line with no words left after the label holds no instruction.

A label names the first instruction at or after its own line. When no instruction follows, the
label names the end of the program.

## Operands

Each operand has one of three kinds:

- `R`, a register: one of `a`, `b`, `c`, `d`, `e` and `f`.
- `V`, a value: a register, or a number written as an optional `-` and then 1 to 7 digits
  (`007` and `-0` are numbers).
- `L`, a label name: a lower-case letter `a` to `z` followed by any lower-case letters, digits and
  underscores. A label name that is also a register name is allowed.

## Instructions

| Instruction | Effect |
|---|---|
| `set R V` | R becomes V. |
| `add R V` | R becomes R + V. |
| `sub R V` | R becomes R - V. |
| `mul R V` | R becomes R * V. |
| `div R V` | R becomes R / V, the fraction dropped, rounding toward zero: `-7 / 2` is -3. |
| `mod R V` | R becomes the remainder that goes with `div`, R - (R / V) * V: `-7 mod 2` is -1 and `7 mod -2` is 1. |
| `jmp L` | Go to the instruction named by L. |
| `jz V L` | Go to L when V is 0. |
| `jlt V V L` | Go to L when the first value is less than the second. |
| `call L` | Push the place of the next instruction (the end of the program when there is none) on the call stack and go to L. |
| `ret` | Pop a place off the call stack and go there. |
| `out V` | Print V in decimal, with a `-` in front when it is negative, and a line feed. |
| `halt` | Stop. |

## Running

The six registers start at 0 and the call stack starts empty. The machine runs the instructions
one at a time, starting with the first instruction of the file. After an instruction that does not
go elsewhere, the next instruction in the file runs. The program ends normally after `halt`, or
when it goes to the end of the program, or when it runs past the last instruction. When it ends
normally, the exit status is 0.

Every instruction run counts as one step, `halt` included. Before an instruction runs, when
`LIMIT` steps have already been run, the program stops with the error `step limit`.

These are the errors while running. The instruction that meets one does not take effect.

- `division by zero`: `div` or `mod` with V equal to 0.
- `overflow`: an `add`, `sub` or `mul` whose result is less than -9999999 or greater than
  9999999.
- `stack overflow`: `call` when the call stack already holds 16 places.
- `ret without call`: `ret` when the call stack is empty.

On an error while running, the lines printed by `out` so far stay, then `error: line N: WHAT` is
printed, where `N` is the line of the instruction that met the error (for `step limit`, the one
that was about to run) and `WHAT` is the error, and the exit status is 20.

## Assembling

The whole file is checked before anything runs. When it is wrong, print exactly one line,
`error: line N: WHAT`, and nothing else, and exit with status 20.

First, the lines are checked in order, and the first line with an error is reported. On a line,
the first of these that applies is its error:

- `bad label`: the label's name is not a label name.
- `duplicate label`: an earlier line defines a label with the same name.
- `unknown instruction`: the instruction's name is not one in the table.
- `wrong count`: the instruction does not have the number of operands the table gives it.
- `bad operand`: an operand, checked from the left, is not of its kind.

When no line has an error, the first operand of kind `L` that names a label no line defines,
taken in line order and from the left on a line, gives the error `unknown label NAME`, with `NAME`
its name.

## Example

With `LIMIT` 100, the program

```
; factorial in a subroutine
      set a 5
      call fact
      out b
      halt
fact: set b 1
loop: jz a done
      mul b a
      sub a 1
      jmp loop
done: ret
```

prints `120` and exits with status 0, after 27 steps.
