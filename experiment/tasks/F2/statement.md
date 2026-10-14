# Split by key

Split the records of one input file into one output file per key, and write an index file.

## Arguments

The program is run with exactly three arguments: the input name, the field number `F`, and the
index name.

- The input name is 1 to 30 characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`,
  `.`, `_` or `-`, and it ends with `.csv`.
- `F` must be one or more decimal digits (`0` to `9`) and nothing else, with a value from 1 to 9;
  leading zeros are allowed, so `03` is 3. So `0`, `10`, `+1` and the empty argument are not valid.
- The index name is 1 to 30 characters from the same set and `/`. It never ends with `.txt` and is
  never the input name.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, empty lines included. The file has at most 200 lines of at most
200 characters.

An empty line, one with no characters at all, is skipped. Every other line is a record, a line of
only spaces included. A record is cut into fields at every `,`: a record with `k` commas has `k + 1`
fields, numbered from 1. Fields may be empty and are never trimmed. The key of a record is its
field `F`. The rest of a record is its other fields, in order, joined with `,`: the rest of
`a,,b` with `F` = 1 is `,b`, and the rest of a record with one field is empty.

## Keys and file names

The file name of a key is the key with every upper-case letter `A` to `Z` changed to lower case,
then every character that is not `a` to `z` or `0` to `9` changed to `_`, then `.txt` added. So
`New York` gives `new_york.txt` and `Q.2` gives `q_2.txt`.

Keys are compared exactly, case included: `Oslo` and `oslo` are two keys. The first record with a
key gives the key its file name. Two different keys must not give the same file name (see Errors).

## Output

For every key, write its file: the rest of every record with that key, one line each, in input
order, each ending with a line feed.

Then write the index file, with one line per key:

```
NAME COUNT KEY
```

`NAME` is the key's file name, `COUNT` its number of records in decimal, and `KEY` the key exactly
as in the input. Every line ends with a line feed. With no record, the index file is empty.

Both the key files and the index lines go in rising ASCII order of the file names: `a.txt` comes
before `a1.txt`, which comes before `a_b.txt`, which comes before `ab.txt`. The key files are
written in that order, and the index file last. A file of the same name that already exists is
replaced entirely.

When every file is written, print one line and exit with status 0:

```
split R records into K files
```

`R` is the number of records and `K` the number of keys, both in decimal. When `R` is 1 the word is
`record`, and when `K` is 1 the word is `file`.

Example: run with `sales.csv`, `2` and `list`, where `sales.csv` holds

```
jan,Oslo,4
jan,Bergen,7
feb,Oslo,5
```

The file `bergen.txt` is written with the line `jan,7`, the file `oslo.txt` with the lines `jan,4`
and `feb,5`, and `list` with the lines `bergen.txt 1 Bergen` and `oslo.txt 2 Oslo`. The output is:

```
split 3 records into 2 files
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. The checks are made in this order and only the first that fails is reported:

1. `F` is not valid: `error: bad field`. This is checked before the input is read.
2. The input cannot be read, for example because it does not exist: `error: cannot read NAME`,
   with the input name.
3. Every record is checked in file order, and the first record that fails one of these checks is
   reported, with `N` its line number in decimal. On that record the checks are made in this order:
   1. It has fewer than `F` fields: `error: line N: no field F`, with `F` in decimal without
      leading zeros. For example, with `F` = 3 the record `a,b` on line 4 gives
      `error: line 4: no field 3`.
   2. Its key is empty: `error: line N: empty key`. For example, with `F` = 1 the record `,b` has
      an empty key.
   3. Its key has not appeared in an earlier record, but its file name is that of an earlier
      key: `error: line N: KEY clashes with EARLIER in NAME`, with `EARLIER` that earlier key and
      `NAME` the file name. For example, with `F` = 2, the records `x,New York` and then
      `y,new-york` give `error: line 2: new-york clashes with New York in new_york.txt`.
4. A file cannot be written, for example because the index name names a folder that does not
   exist: `error: cannot write NAME`, with the name of the first file in writing order that cannot
   be written.

When a check of 1 to 3 fails, no file is written. When a write fails, the files written before it
stay as written. The program never creates a folder.
