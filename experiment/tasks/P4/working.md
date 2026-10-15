# P4 hand working

Not part of the statement. One entry per case: the rule it checks, a trace of the run (or the
line that fails to assemble), and the output.

## cases/1 (limit 100)

The example. `set a 5`, `call fact` (stack: the place of `out b`), `set b 1`. Each of five rounds
runs `jz`, `mul`, `sub`, `jmp`: b becomes 5, 20, 60, 120, 120 and a goes 5 to 0. Then `jz a done`
jumps, `ret` goes back to `out b`, which prints `120`, and `halt`. Steps: 3 + 20 + 2 + 2 = 27.

## cases/2 (limit 10)

Every line is well formed, so the label pass runs: line 2 `jz a stop`, and no line defines
`stop`: `error: line 2: unknown label stop`. Nothing runs.

## hidden/1 (limit 100): division, remainder, overflow

- b = -7, `div b 2`: -3.5 toward zero is -3. Prints `-3`.
- b = -7, `mod b 2`: -7 - (-3 * 2) = -1. Prints `-1`.
- c = 7, `mod c -2`: 7 / -2 is -3; 7 - (-3 * -2) = 1. Prints `1`.
- a = -7, `div a -2`: 3.5 toward zero is 3. Prints `3`.
- d = 3000000, `mul d 3`: 9000000, inside the range. Prints `9000000`.
- `add d 1000000` (line 16): 10000000 > 9999999: `error: line 16: overflow`, after the five lines.

## hidden/2 (limit 100001)

100001 is above 100000: `error: bad limit`. The file, which is not a program, is never read.

## hidden/3 (limit 12): halt counts as a step

Steps: `set a 3` (1). Round for a = 3: `out`, `sub`, `jz` (a = 2, no jump), `jmp` (5). Round for
a = 2 the same (9). Round for a = 1: `out`, `sub` (a = 0), `jz` jumps to `end` (12). `halt` on line
6 is about to run with 12 steps run: `3`, `2`, `1`, then `error: line 6: step limit`. (With limit
13 the program ends normally.)

## hidden/4 (limit 100): labels on their own lines

- `jmp a` (line 2): `a` is a label name here, though it is also a register.
- `a:` on line 4 has no instruction; lines 5 (comment) and 6 (empty) have none either; it names
  `out 1` on line 7. So `out 99` never runs.
- `out 1`, `call f`, `out 2`, `ret`, `out 3`, `jmp done`.
- `done:` on the last line has no instruction after it: it names the end, and the program ends
  normally.

Output: `1`, `2`, `3`, status 0.

## hidden/5 (limit 1000): stack overflow

Each pass runs `add a 1`, `out a`, `call rec`. Passes 1 to 16 push places (stack 1 to 16). Pass 17
prints `17`, then `call` with 16 places held: `error: line 4: stack overflow`. Output `1` to `17`
first. Steps used: 1 + 17 * 3 = 52, far below the limit.

## hidden/6 (limit 100): ret without call

`set a 2`, `call twice`, `mul a 2` (4), `ret` back to `out a`: prints `4`. No `halt`, so the next
instruction is `mul a 2` again (8), then `ret` with an empty stack: `error: line 5: ret without
call`.

## hidden/7 (limit 100): line errors before labels

Line 2 uses `nowhere`, which no line defines, but the line pass comes first. Lines 1 to 4 are well
formed; line 5 `mul a 1.5`: `1.5` is not a register or number: `error: line 5: bad operand`.
(Line 6 has a bad operand too, but line 5 comes first.)

## hidden/8 (limit 100): label errors first on a line

Line 4 `loop: halt now` defines `loop` again and also has a wrong count; the label is checked
first: `error: line 4: duplicate label`. (Line 6 `Exit:` is a bad label, but comes later.) No final
line feed.
