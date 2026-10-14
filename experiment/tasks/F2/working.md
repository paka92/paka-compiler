# F2 hand working

Not part of the statement. One entry per case: the argument checks, then every line (LN for line N)
with its key, file name and rest, the order of the file names, and what is written. Second check:
the counts in the index add up to the number of records.

## cases/1 (`log.csv`, `1`, `index`)

`1` is valid. Records:

- L1 `Web,GET /a,200`: key `Web`, `web.txt`, rest `GET /a,200`.
- L2 `db,query,ok`: key `db`, `db.txt`, rest `query,ok`.
- L3 `Web,GET /b,404`: key `Web`, rest `GET /b,404`.
- L4 `Web`: one field, key `Web`, rest empty.
- L5 is empty and skipped.
- L6 `Web 2,POST,201`: key `Web 2`, `web_2.txt`, rest `POST,201`.
- L7 `web2,PUT,500`: key `web2`, `web2.txt`, rest `PUT,500`.
- L8 `DB.old,,x`: key `DB.old`, `db_old.txt`, rest `,x` (the empty field 2 and `x`).

No field is missing, no key is empty, and the five file names differ. Order: `db.txt` before
`db_old.txt` (`.` 46 < `_` 95); `web.txt` before `web2.txt` (`.` 46 < `2` 50) before `web_2.txt`
(`2` 50 < `_` 95). `web.txt` holds three lines, the last empty. Index counts 1 + 1 + 3 + 1 + 1 = 7
records, 5 keys: `split 7 records into 5 files`.

## cases/2 (`items.csv`, `03`, `out/idx`)

`03` is 3. Keys `red` (L1, L3) and `blue` (L2); rests `1,a`, `2,b`, `3,c`. `blue.txt` and `red.txt`
are written in that order. The index `out/idx` names the folder `out`, which does not exist:
`error: cannot write out/idx`, status 20. The two key files stay.

## hidden/1 (`x.csv`, `0`, `idx`)

`0` has the value 0, not 1 to 9: `error: bad field`, status 20. The input does not exist, but it is
never read.

## hidden/2 (`missing.csv`, `001`, `idx`)

`001` is 1, valid. Only `Missing.csv` exists; names are exact, so `missing.csv` cannot be read:
`error: cannot read missing.csv`, status 20.

## hidden/3 (`t.csv`, `04`, `ix`)

`04` is 4. L1 has 4 fields, key `d`. L2 is empty. L3 `e,f,g` has 3 fields, fewer than 4:
`error: line 3: no field 4`, status 20, with `4` and not `04`. L4 `,,,` (4 empty fields, empty key)
is later.

## hidden/4 (`k.csv`, `2`, `ix`)

L1 key `K`, `k.txt`. L2 `2,` has 2 fields, field 2 empty: `error: line 2: empty key`, status 20.
L3 key `k` would clash with `K` in `k.txt`, but comes later.

## hidden/5 (`c.csv`, `1`, `ix`)

- L1 key `A.b`, `a_b.txt`. L2 key `x`, `x.txt`.
- L3 key `A.b` again: the same key, no clash.
- L4 key `a b`, new, `a_b.txt`, which is the file name of `A.b`: `error: line 4: a b clashes with
  A.b in a_b.txt`, status 20.

## hidden/6 (`m.csv`, `1`, `all`)

A stale `m.txt` holding `old` is in the folder.

- L1 `m,1`: key `m`, `m.txt`, rest `1`.
- L2 `Mo,2`: key `Mo`, `mo.txt`, rest `2`.
- L3 `m-o,3`: key `m-o`, `m_o.txt`, rest `3`.
- L4 `7,4`: key `7`, `7.txt`, rest `4`.
- L5 `m`: key `m`, rest empty.
- L6 two spaces: a record, not an empty line; one field, key two spaces, `__.txt`, rest empty.
- L7 `m,6`: key `m`, rest `6`.
- L8 `Mo,x,y`: key `Mo`, rest `x,y`.

Order: `7.txt` (`7` 55), `__.txt` (`_` 95), then `m.txt`, `m_o.txt`, `mo.txt` (`.` 46 < `_` 95 <
`o` 111). `m.txt` is replaced by `1`, an empty line and `6`. The index line for the spaces key is
`__.txt 1` and a space, then the two spaces of the key. Counts 1 + 1 + 3 + 1 + 2 = 8 records, 5
keys: `split 8 records into 5 files`.

## hidden/7 (`e.csv`, `5`, `index.lst`)

Two empty lines, no record. Field 5 is never needed. The index is written empty. Output:
`split 0 records into 0 files`, plural for 0.

## hidden/8 (`one.csv`, `2`, `idx`)

One line with no line feed. Key `R&D: Q2`: lowered `r&d: q2`, then `&`, `:` and the space become
`_`: `r_d__q2.txt`. Rest `2024-05-01,7`. Index line `r_d__q2.txt 1 R&D: Q2`. Output:
`split 1 record into 1 file`.
