# S5 hand working

Not part of the statement. One entry per case: each command with the buffer as `[C/N]` and the
lines, and the undo and redo lists as step counts `u/r` after it.

## cases/1 (`edit.txt`)

`add one`, `add two`: one step, `[2/2] one two`, 1/0. `up`: `[1/2]`. `set uno`: new step,
`[1/2] uno two`, 2/0. `down`: `[2/2]`. `undo`: state before `set`, `[1/2] one two`, 1/1. `print`:
`[1/2]`, `>1|one`, ` 2|two`. `del`: `two` becomes line 1 and current, new step, 2/0.
`undo 2, redo 0`.

## cases/2 (`typo.txt`)

Line 3 is `undo` with a space after it: `error: line 3: bad command`. Nothing runs, so nothing is
printed before it.

## hidden/1 (`ten.txt`)

Ten adds in a row: one step, `[10/10]`. Two `up`: `[8/10]`. `print`: width of `10` is 2, so
`  1|line 1` ... `> 8|line 8` ... ` 10|line 10`. `undo`: back to the empty start, `[0/0]`, 0/1.
`print`: `[0/0]` and no lines.

## hidden/2 (`cap.txt`)

Step 1 `add a`, then `set b`, `set c`, `set d` make steps 2, 3, 4; step 1 is dropped, 3/0. Three
undos: `c`, `b`, then `a`, 0/3. Fourth `undo` (line 8): `line 8: nothing to undo`. `print`:
`[1/1]`, `>1|a`.

## hidden/3 (`split.txt`)

`add x`, `add y`: one step, `[2/2]`. `down` is rejected (`line 3: at bottom`) but still stands
between the adds, so `add z` makes step 2: `[3/3] x y z`. `undo`: `[2/2] x y`, 1/1.

## hidden/4 (`same.txt`)

`add a`: step 1. `print` stands between, so `add b` is step 2: `[2/2]`. `undo`: `[1/1] a`, 1/1.
`set a`: same text, not an edit, redo kept. `redo`: `[2/2] a b`, 2/0. `set B`: step 3,
`[2/2] a B`, 3/0.

## hidden/5 (`del.txt`, no last line feed)

Three adds: step 1, `[3/3] a b c`. `up`: `[2/3]`. `del`: `b` goes, `c` is now line 2 and current,
`[2/2]`, step 2. `down`: `line 7: at bottom`. `del`: `c` goes, no next line, `a` is current,
`[1/1]`, step 3. `del`: `[0/0]`, step 4, step 1 dropped, 3/0. `del` and `up` on the empty buffer:
`line 11: empty buffer`, `line 12: empty buffer`. `undo`: `[1/1] a`, 2/1.

## hidden/6 (`cursor.txt`)

Three adds: `[3/3]`. Two `up`: `[1/3]`. `set A`: step 2, before `[1/3] a b c`, after `[1/3] A b c`.
Two `down`: `[3/3]`. `undo`: `C` comes from the step, `[1/3] a b c`. `down`: `[2/3]`. `redo`:
`[1/3] A b c`, not line 2. 2/0.

## hidden/7 (`text.txt`)

`add first`, `add last`: step 1, `[2/2]`. `up`: `[1/2]`. `add ` (empty text): after line 1,
`[2/3]`, step 2 (the line before is `up`). `add   spaced  `: joins step 2, text `  spaced  ` with
its spaces, `[3/4] first, "", "  spaced  ", last`. `print` shows ` 2|` with nothing after the bar.
`undo`: back to `[1/2] first last`, 1/1.

## hidden/8 (`blank.txt`)

Line 3 is empty: `error: line 3: bad command`. Line 4 `add` without a space is bad too, but later.
