# N3 hand working

Not part of the statement. One entry per case: the check of every line (LN for line N), the
holidays of the file, then each answer. Weekdays count on from Monday 1900-01-01 (73413 days to
2100-12-31; every seventh day after it is a Monday). Second check: one printed line per `days`,
`weekday` and `add` line.

## cases/1 (`dates.txt`)

All lines are well formed with valid dates; L7 is empty. Holidays: 2023-12-26 (Tue), 2024-01-01
(Mon), 2023-12-30 (Sat, no effect).

- L1: 2000 divides by 400, a leap year, so 2000-02-29 is valid. Tuesday.
- L2: 2023-12-25 to 2024-01-01 is 7 days, to 2024-01-08 7 more: 14.
- L3: Fri 2023-12-22. Forward: Mon 25 (1), Tue 26 holiday, Wed 27 (2), Thu 28 (3). 2023-12-28.
  The holidays stand below this line and still count.
- L6: Tue 2024-01-02. Back: Mon 01 holiday, Sun 31, Sat 30, Fri 29 (1), Thu 28 (2), Wed 27 (3).
  2023-12-27.
- L9: 2100 is not a leap year; 2100-02-28 is the day before 2100-03-01: -1.
- L10: Tue 1900-01-02. Back: Mon 1900-01-01 (1); the next step is before 1900-01-01: `line 10: out
  of range`.

## cases/2 (`bad.txt`)

L1 is fine. L2 is well formed, but 1900 divides by 100 and not by 400, so 1900-02-29 is not valid:
`error: line 2: bad date`, status 20. L3 (`x`) is a bad command but later; nothing else prints.

## hidden/1 (`h1.txt`)

L1 2000-02-29 valid, L2 valid. L3 is well formed, but April has 30 days: `error: line 3: bad date`,
status 20. L4 (`+5`) is a bad command but later.

## hidden/2 (`h2.txt`)

L1 2024-02-29 valid (2024 divides by 4). L2: `+1` is not 1 to 3 digits with an optional `-`, so the
form check fails first, though 2100-02-29 is also not valid: `error: line 2: bad command`, status
20. L3 is not reached.

## hidden/3 (`h3.txt`)

- L1: years 1900 to 2100 hold 201 * 365 days plus 49 leap days (1904 to 2096, step 4; 2000 is
  one, 1900 and 2100 are not): 73414 days. From the first day to the last is one less: 73413.
- L2: the same dates turned round: -73413.
- L3: 73413 = 7 * 10487 + 4, so four days after a Monday: Friday.
- L4: Monday.
- L5: the same date: 0.

## hidden/4 (`h4.txt`)

- L1: 1900 is not a leap year: Jan 31 + Feb 28 = 59 days after Monday 1900-01-01; 59 = 56 + 3:
  Thursday.
- L2: 2100-03-01 is 59 days after 2100-01-01, a Friday (73413 - 364 = 73049 days after the start,
  73049 = 7 * 10435 + 4); 59 = 56 + 3, so three days after Friday: Monday.
- L3: 2024-12-25 is a Wednesday (statement example); 2024-12-31 is six days later: Tuesday.
- L4: 1900 to 1986 hold 87 * 365 + 21 leap days (1904 to 1984) = 31776 days; 31776 = 7 * 4539 + 3,
  so 1987-01-01 is a Thursday. 1987-06-15 is 31 + 28 + 31 + 30 + 31 + 14 = 165 days later; 165 =
  7 * 23 + 4: Monday.
- L5: 1 day to 2000-01-01, 31 to 2000-02-01, 29 more (2000 is a leap year) to 2000-03-01: 61.

## hidden/5 (`h5.txt`)

Holidays: 2024-05-10 (Fri, listed twice), 2024-05-12 (Sun, no effect). All stand below L1 to L3.

- L1: Thu 2024-05-09. Forward: Fri 10 holiday, Sat, Sun, Mon 13 (1), Tue 14 (2). 2024-05-14.
- L2: Mon 2024-05-13. Back: Sun 12, Sat 11, Fri 10 holiday, Thu 09 (1). 2024-05-09.
- L3: Sat 2024-05-11 is not a working day. Forward: Sun 12, Mon 13 (1). 2024-05-13.
- L7: Sat 2024-05-11. Back: Fri 10 holiday, Thu 09 (1). 2024-05-09.

## hidden/6 (`h6.txt`)

Holiday: 2100-12-31 (Fri).

- L1: from Fri 2100-12-31 the next working day is after 2100-12-31: `line 1: out of range`.
- L2: from Mon 1900-01-01 the last working day before it is before 1900-01-01: `line 2: out of
  range`.
- L3: Thu 2100-12-30. Fri 31 is a holiday, Sat and Sun follow, then 2101: `line 3: out of range`.
- L4: Wed 1900-01-03. Back: Tue 02 (1), Mon 01 (2). 1900-01-01, the first date in range.
- L5: `-0` is 0: the date itself, 2100-12-25, a Saturday.

## hidden/7 (`h7.txt`)

Holiday: 2025-01-01 (Wed). 2024-12-25 is not a holiday in this file.

- L2: Fri 2024-12-20. Forward: Mon 23 (1), Tue 24 (2), Wed 25 (3), Thu 26 (4), Fri 27 (5), Mon 30
  (6), Tue 31 (7), Wed 01 holiday, Thu 02 (8), Fri 03 (9), Mon 06 (10). 2025-01-06.
- L3: Fri 2025-01-10. Back: Thu 09 (1), Wed 08 (2), Tue 07 (3), Mon 06 (4), Fri 03 (5), Thu 02
  (6), Wed 01 holiday, Tue 31 (7), Mon 30 (8), Fri 27 (9). 2024-12-27.
- L4: 2025-01-01 is a Wednesday, three days later: Saturday.

## hidden/8 (`h8.txt`)

L1 is empty. The last line has no line feed. The holiday on L3 does not touch `days`.

- L2: 2023 is not a leap year: 1.
- L4: 1996 is a leap year: 29.
- L5: 1900 is not a leap year: 28.
- L6: -1, across a year end.
- L7: April has 30 days: 1 to 2020-05-01, then 30 more: 31.
