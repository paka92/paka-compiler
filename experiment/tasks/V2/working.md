# V2 hand working

Not part of the statement. One entry per case: the minute of every event (LN for line N), the
form and time checks, then each stay with `M`, `D`, `R`, `H` and the fee. Second check: the total
is the sum of the printed fees.

## cases/1 (`park.txt`)

Minutes: L1 360, L2 450, L3 465, L4 480, L5 480, L6 540, L7 751, L8 empty, L9 1440, L10 2400,
L11 2879. All lines well formed; never smaller than the line before (L5 equals L4, allowed).

- L1 Q1 in at 360. L2 K9 in at 450. L3 AB12 in at 465.
- L4 K9 is inside: `line 4: K9 is already in`.
- L5 AB12 out: M = 15, at most 15, fee 0.00.
- L6 AB12 is not inside: `line 6: AB12 is not in`.
- L7 K9 out: M = 751 - 450 = 301. D = 0, R = 301, H = 6. Charge 2.00 + 1.50 + 1.50 + 1.00 * 3 =
  8.00.
- L9 ZZ1 in at 1440.
- L10 Q1 out: M = 2400 - 360 = 2040. D = 1, R = 600, H = 10. Charge 2.00 + 3.00 + 7.00 = 12.00,
  not more than 12.00. Fee 12.00 + 12.00 = 24.00.
- L11 A7 in at 2879.

Still in: A7 (day 2 23:59), ZZ1 (day 2 00:00); `A` before `Z`. Total 0.00 + 8.00 + 24.00 = 32.00.

## cases/2 (`late.txt`)

All lines well formed. Minutes: L1 2880 + 600 = 3480, L2 3540, L3 (`03` is day 3) 2880 + 599 =
3479. L3 is smaller than L2: `error: line 3: time goes back`, status 20.

## hidden/1 (`a.txt`)

Pass 1: L3 has day 367, more than 366: malformed. `error: line 3: bad event`, status 20. L2 (09:00
after 10:00) goes back in time, but pass 2 never runs.

## hidden/2 (`b.txt`)

L1 is empty and skipped. L2 is well formed. L3 has minute `60`: malformed. `error: line 3: bad
event`, status 20. L4 (`IN`) is malformed too but comes later.

## hidden/3 (`c.txt`)

L1 to L5 enter A to E at minute 0. Exits at 16, 61, 180, 540, 601, rising.

- A: M = 16, more than 15. D = 0, R = 16, H = 1: 2.00.
- B: M = 61, H = 2: 2.00 + 1.50 = 3.50.
- C: M = 180, H = 3: 2.00 + 1.50 + 1.50 = 5.00.
- D: M = 540, H = 9: 5.00 + 6 * 1.00 = 11.00.
- E: M = 601, H = 11: 5.00 + 8 * 1.00 = 13.00, more than 12.00: 12.00.

No car is left. Total 2.00 + 3.50 + 5.00 + 11.00 + 12.00 = 33.50.

## hidden/4 (`d.txt`)

Minutes: L1 to L3 4 * 1440 + 720 = 6480, L4 5 * 1440 + 720 = 7920, L5 7935, L6 7 * 1440 + 720 =
10800, L7 10080 + 1435 = 11515, L8 11520 + 5 = 11525. Rising.

- M1: M = 1440. D = 1, R = 0, charge 0.00. Fee 12.00.
- M2: M = 1455. D = 1, R = 15, H = 1, charge 2.00. Fee 14.00 (rule 1 looks at the whole stay).
- M3: M = 4320. D = 3, R = 0. Fee 36.00.
- N: M = 10, at most 15: 0.00, across midnight.

Total 12.00 + 14.00 + 36.00 + 0.00 = 62.00.

## hidden/5 (`e.txt`)

Minutes: L1 to L6 360, L7 420, L8 and L9 440. Rising or equal.

- L1 Z9 is not inside: `line 1: Z9 is not in`.
- L2 A10, L3 9Z, L4 A1 in at 360.
- L5 A1 out: M = 0, fee 0.00.
- L6 A1 in again at 360: a new stay.
- L7 A1 is inside: `line 7: A1 is already in`.
- L8 9Z out: M = 80, H = 2: 3.50.
- L9 9Z in again at 440.

Still in: 9Z (day 1 07:20), A1 (day 1 06:00, from L6), A10 (day 1 06:00). `9` (57) is before `A`
(65); `A1` is the start of `A10`, so it comes first. Total 0.00 + 3.50 = 3.50.

## hidden/6 (`f.txt`)

L1, L2 and L4 are empty. L3 Q enters on day `007`, day 7. No stay ends. Output: `Q: still in since
day 7 08:05`, with no leading zeros, then `total: 0.00`.

## hidden/7 (`g.txt`)

All lines well formed. Minutes: L1 1440 + 600 = 2040, L2 2040 (equal, allowed), L3 empty, L4
(`002` is day 2) 1440 + 599 = 2039. The event before L4 is L2, the nearest line that is not empty:
2039 is smaller than 2040. `error: line 4: time goes back`, status 20. L2 (B not inside) is never
run.

## hidden/8 (`h.txt`)

The last line has no line feed. Both cars enter at minute 0.

- L3 12345678 out at 365 * 1440 + 15 = 525615. M = 525615, D = 365, R = 15, H = 1. Fee 365 *
  12.00 + 2.00 = 4382.00.
- L4 ABCDEFGH out at 525600 + 1439 = 527039. M = 527039, D = 365, R = 1439, H = 24: charge 2.00 +
  3.00 + 21.00 = 26.00, more than 12.00: 12.00. Fee 4380.00 + 12.00 = 4392.00.

Lines print in event order. Total 4382.00 + 4392.00 = 8774.00.
