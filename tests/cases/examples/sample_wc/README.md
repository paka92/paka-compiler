# wc sample with examples

The wc program of tests/cases/run-interp/sample_wc_count, unchanged, with two function examples of
`count_words` added after the app, run with `paka examples`. Expected output is written by hand.
This is the copy M7-11 approves and releases.

- `count_words_empty` and `count_words_two_lines` are the names docs/decisions/M7-03.md section
  1.1 shows. The empty text has no word; `one two`, a line feed, two spaces, `three` and a tab
  hold three, since spaces, tabs and line feeds all split words (M2-14).
