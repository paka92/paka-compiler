# Pending cases

Two folders for the cases that docs/decisions/M2-15.md section 4 changes and that the milestone 1
back ends cannot run yet. Each has the layout of spec/m1.md section 11. The task that replaces the
back ends moves each one into `tests/cases/`; this file goes when the last one has moved.

| Case | Expected under `paka run main.paka` | Decision |
|---|---|---|
| `check/unknown_grant` | `ReadFiles` is a grant: runs, Print refused, status 3 | M2-15 section 4, row 3 |
| `parse/variant_payload` | usage line, status 2 | M2-15 section 4, row 5 |

`check/unknown_grant` goes over nothing: the old case of that name is gone, and
`check/unknown_grant_network` keeps `unknown-grant` covered.

`parse/variant_payload` with one argument prints `let there be light` and ends with status 0; the
layout cannot hold a case with an argument.
