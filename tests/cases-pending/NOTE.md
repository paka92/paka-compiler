# Pending cases

Re-expected copies of the nine M1 cases that docs/decisions/M2-15.md section 4 changes. Each
folder has the layout of spec/m1.md section 11 and the same `main.paka` as the case it replaces
under `tests/cases/`. Nothing runs these folders yet. The task that changes the tool moves each one
over the old case with `git mv`; this file goes when the last one has moved.

| Case | Expected under `paka run main.paka` | Decision |
|---|---|---|
| `check/no_app` | `no-app`, no position, status 1 | M2-15 section 4, row 1 |
| `check/empty_file` | the same | M2-15 section 4, row 2 |
| `check/unknown_grant` | `ReadFiles` is a grant: runs, Print refused, status 3 | M2-15 section 4, row 3 |
| `check/order_source_order` | `type-mismatch` at 10:3 | M2-15 section 4, row 4 |
| `parse/variant_payload` | usage line, status 2 | M2-15 section 4, row 5 |
| `parse/app_state_entry` | `bad-state` at 8:11 | M2-15 section 4, row 6 |
| `parse/no_parameter` | `bad-update-signature` at 9:11 | M2-15 section 4, row 7; M2-02 section 4 |
| `parse/two_parameters` | `bad-update-signature` at 9:11 | M2-15 section 4, row 8 |
| `parse/inout_parameter` | `bad-update-signature` at 9:11 | M2-15 section 4, row 9 |

What the layout cannot hold, because a case is one `paka run main.paka` with no argument:

- `check/no_app` and `check/empty_file`: the change is in `paka check`, which now gives status 0 and
  no output. Under `paka run` the expectation file is the same as before.
- `parse/variant_payload` with one argument prints `let there be light` and ends with status 0.
- The decision also asks for two new cases that keep `unknown-grant` covered: `check/unknown_grant`
  with `grants: [Network]` (10:12) and `check/order_source_order` with `grants: [Files]` (4:12).
  They are not written here.
