# G1 hand working

Not part of the statement. One entry per case: the context check, then the walk with the two table
values compared at every step where the lines differ, then the groups and headers. K, D and I mark
a keep, delete and insert. Second check: in every hunk, the keeps plus deletes equal `B` and the
keeps plus inserts equal `E`, and the walk uses every old and new line once.

## cases/1 (`old.txt`, `new.txt`, `C` = 1)

n = 7, m = 8.

- (0,0) apple = apple: K.
- (1,1) banana / blueberry: L(2,1) = |cherry date elder fig grape| = 5, L(1,2) = 5; 5 >= 5: D banana.
- (2,1) cherry / blueberry: L(3,1) = 4 < L(2,2) = 5: I blueberry.
- (2,2) to (6,6): cherry, date, elder, fig: K each.
- (6,6) grape / guava: L(7,6) = 0 (i = n) < L(6,7) = 1: I guava.
- (6,7) grape = grape: K.

List: K D I K K K K I K. Between the change at 3 and the one at 8 lie 4 keeps > 2: two groups.

- Hunk 1: K apple, D banana, I blueberry, K cherry. Old lines 1-3, new lines 1-3: `@@ -1,3 +1,3 @@`.
- Hunk 2: K fig (old 6, new 6), I guava (new 7), K grape (old 7, new 8): `@@ -6,2 +6,3 @@`.

## cases/2 (`v1.txt`, `v2.txt`, `C` = 2)

- alpha, beta, gamma: K.
- (3,3) delta / DELTA: L(4,3) = 0 (i = n), L(3,4) = 0 (delta / epsilon); 0 >= 0: D delta.
- (4,3): i = n, j < m: I DELTA, then (4,4): I epsilon.

List: K K K D I I. One group; 2 keeps before (beta, gamma), none after. Old beta, gamma, delta =
lines 2-4: `-2,3`. New beta, gamma, DELTA, epsilon = lines 2-5: `+2,4`.

## hidden/1 (`C` = `011`)

Digits only, but the value is 11 > 10: `error: bad context`, status 20. The files (which differ) are
not read.

## hidden/2 (`C` = `-0`)

`-` is not a digit: `error: bad context`, status 20, though the value would be 0 and both files are
empty.

## hidden/3 (`C` = 3)

Old `p\nq\n` and new `p\nq` both have the lines p, q. Two K: `no differences`.

## hidden/4 (`C` = 0)

- (0,0) m / k: L(1,0) = |n| = 1 < L(0,1) = |m n| = 2: I k.
- m, n: K.

List: I K K. Hunk: only I k (no context). It uses no old line, and 0 old lines come before it:
`-0,0`. New line 1: `+1,1`.

## hidden/5 (`C` = 3, new file empty)

m = 0, so j = m at every step: D r, D s, D t. Old lines 1-3: `-1,3`. No new line used, none before:
`+0,0`.

## hidden/6 (`C` = 0)

- (0,0) red / green: L(1,0) = |green| = 1, L(0,1) = |red| = 1; 1 >= 1: D red.
- (1,0) green = green: K.
- (2,1): i = n: I red.

List: D K I. One keep between the changes > 0: two groups.

- Hunk 1: D red. Old line 1: `-1,1`. No new line, none before: `+0,0`.
- Hunk 2: I red. No old line; 2 old lines (red, green) used before: `-2,0`. New line 2 (green is new
  line 1): `+2,1`.

## hidden/7 (`C` = 1)

- a1: K.
- (1,1) a2 / B2: L(2,1) = |a3 a4 a6 a7 a8| = 5, L(1,2) = 5: D a2. (2,1) a3 / B2: L(3,1) = 4 <
  L(2,2) = 5: I B2.
- a3, a4: K.
- (4,4) a5 / B5: L(5,4) = |a6 a7 a8| = 3, L(4,5) = 3: D a5. (5,4) a6 / B5: L(6,4) = 2 < L(5,5) = 3:
  I B5.
- a6, a7, a8: K.
- (8,8) a9 / B9: L(9,8) = 0, L(8,9) = 0: D a9. (9,8): i = n: I B9.

List: K D I K K D I K K K D I. Between I B2 and D a5: 2 keeps <= 2, same group. Between I B5 and
D a9: 3 keeps > 2, new group.

- Hunk 1: K a1 ... K a6. Old a1-a6 = lines 1-6; new a1 B2 a3 a4 B5 a6 = lines 1-6: `@@ -1,6 +1,6 @@`.
- Hunk 2: K a8, D a9, I B9 (list end). Old 8-9, new 8-9: `@@ -8,2 +8,2 @@`. a7 is in no hunk.

## hidden/8 (`C` = `07`)

`07` is valid with value 7. Old `x\ny\nz` (no last line feed) has lines x, y, z.

- x: K.
- (1,1) y / Y: L(2,1) = |z| = 1, L(1,2) = |z| = 1: D y. (2,1) z / Y: L(3,1) = 0 < L(2,2) = 1: I Y.
- z: K.
- (3,3): i = n: I w.

List: K D I K I. One group (1 keep between). Up to 7 keeps before: only x; none after. Old x y z =
`-1,3`; new x Y z w = `+1,4`.
