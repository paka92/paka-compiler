# Pending cases

Eleven folders for the cases that docs/decisions/M2-15.md section 4 changes or adds. Each has the
layout of spec/m1.md section 11. Nothing runs these folders yet. The task that changes the tool
moves each one into `tests/cases/`; this file goes when the last one has moved.

Five replace an existing case with the same `main.paka`: move them over the old case with
`git mv`.

| Case | Expected under `paka run main.paka` | Decision |
|---|---|---|
| `check/no_app` | `no-app`, no position, status 1 | M2-15 section 4, row 1 |
| `check/empty_file` | the same | M2-15 section 4, row 2 |
| `check/unknown_grant` | `ReadFiles` is a grant: runs, Print refused, status 3 | M2-15 section 4, row 3 |
| `check/order_source_order` | `type-mismatch` at 10:3 | M2-15 section 4, row 4 |
| `parse/two_parameters` | `bad-update-signature` at 9:11 | M2-15 section 4, row 8 |

`parse/two_parameters` goes over `check/unsupported_two_parameters`, where the parser work moved the
old `parse/two_parameters`. The other four go over the case of their own name.

Six are new cases:

| Case | Expected under `paka run main.paka` | Decision |
|---|---|---|
| `parse/variant_payload` | usage line, status 2 | M2-15 section 4, row 5 |
| `parse/app_state_entry` | `bad-state` at 8:11 | M2-15 section 4, row 6 |
| `parse/no_parameter` | `bad-update-signature` at 9:11 | M2-15 section 4, row 7; M2-02 section 4 |
| `parse/inout_parameter` | `bad-update-signature` at 9:11 | M2-15 section 4, row 9 |
| `check/unknown_grant_network` | `unknown-grant` at 10:12 | M2-15 section 4, row 3 |
| `check/order_source_order_files` | `unknown-grant` at 4:12 | M2-15 section 4, row 4 |

The last two keep `unknown-grant` covered: they are the old `check/unknown_grant` and
`check/order_source_order` with the grant `Network` and `Files`.

What the layout cannot hold, because a case is one `paka run main.paka` with no argument:

- `check/no_app` and `check/empty_file`: the change is in `paka check`, which now gives status 0 and
  no output. Under `paka run` the expectation file is the same as before.
- `parse/variant_payload` with one argument prints `let there be light` and ends with status 0.
