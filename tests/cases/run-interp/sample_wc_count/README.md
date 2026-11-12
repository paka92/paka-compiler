# wc sample

The four `sample_wc_*` cases run the wc program of GENESIS.md section 4 (lines 242 to 260), as
written out in M2-15 section 1.11. Expected output is written by hand from the records:
`words.txt` holds 11 words (runs split by spaces, a tab and line feeds; `naïve` and `café` are one
word each, since only code points 9 to 13 and 32 are white space). The empty file gives `0 words`,
a missing file `cannot read file: not found` with status 20, and no argument the usage line on
standard error with status 2 (M2-15 section 1.4).

Differences from the text of GENESIS.md, each with the decision that required it:

- Added the line `start: Start` to `app wc`: the app form requires a `start:` key (M2-15 section 1.11, M1-06.3).
- Added `or: Failed` to `ReadFile`, the variant `Failed(why: String)` and its arm that prints
  `cannot read file: {why}` and ends with `ExitWith(20)`: `then:` and `or:` are both required
  (M2-15 section 1.3).
- Added `count_words`, which the paper leaves out: `len(words(text))`, using the `words` built-in (M2-14).
- Single spaces in place of the aligned arrows and keys (M4-47).
