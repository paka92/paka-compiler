# Conformance cases

These cases are not yet required: no harness runs them today. M2-41b runs them.

`ownership/` covers ownership and shared data (M2-05, M2-10, the Map rows of M2-14).

## Case format

A case is a folder `<area>/<name>/`, `<name>` matching `[a-z0-9_]+`, holding a full program in
`main.paka` (an app, shaped like `tests/cases/run/*`).

- A run case adds `stdout`, `status` (decimal digits and a line feed) and, only when standard error
  is not empty, `stderr`. Status 0 cases show their behaviour through printed output; status 3
  cases hold the trap report of M2-06 §1.2 byte for byte.
- A reject case adds `reject` alone: `main.paka:LINE:COL: error[CODE]` and one line feed. Only the
  code and position are pinned; message texts are not, since the records fix most of them only in
  parts.

## Pass rules

The case folder is the working directory, so every diagnostic path is `main.paka`.

- Run case: `paka run main.paka` passes when standard output equals `stdout` byte for byte, the
  exit status equals `status`, and standard error equals `stderr` byte for byte (empty when the
  case has no `stderr` file). The format is spec/m1.md §11.
- Reject case: `paka check main.paka` exits with status 1 and the first line of standard error
  starts with the `reject` text, without its line feed, followed by `: ` and a message. Later lines
  (the field lines of M2-04) are ignored.

## Decision points and their cases

### M2-05 §1.1 to §1.2: copies, places, fresh values

| Point | Case(s) |
|---|---|
| `let`/`var` from a place copies | `copy_let_is_independent` |
| storing a place into a field, item, entry, record, list copies | `copy_on_store` |
| returning a parameter or part of one copies | `copy_on_return_of_parameter`, `fixes_are_accepted` |
| `if`/`match` value that is a place copies | `copy_from_branch_value` |
| `for` name and pattern name are copies | `mark_on_for_name`, `mark_on_pattern_name`, `assign_to_pattern_name`, `walk_for_name_assigned`, `match_arm_changes_subject` |
| passing an argument (read-only or inout) does not copy; changes reach the caller | `inout_change_seen_by_caller` |
| a slice is a value (List and String) | `slice_is_a_value` |
| Int, Float, Bool reads are exempt | `exempt_scalars` |
| a place-rooted read of a function type is fresh; a write in the callee expression is still caught | `function_value_place_is_fresh`, `read_function_value_callee` |
| String is not exempt | `overlap_string_item`, `overlap_field_item_string` |
| fresh values overlap nothing | `fresh_values_overlap_nothing` |
| steps after a slice still count | `overlap_string_after_slice` |
| parentheses change nothing | `walk_parenthesized_place` |

### M2-05 §1.3 overlap table

| Row | Case(s) |
|---|---|
| `p.left` vs `p.right`, apart | `fields_apart` |
| `p` vs `p.right`, overlap | `overlap_prefix`, `overlap_map_prefix` |
| `rows[0]` vs `rows[1]`, overlap | `overlap_two_items` |
| `g.nodes[a].x` vs `g.nodes[b].y`, overlap | `overlap_index_then_fields` |
| `xs` vs `ys`, apart | `fields_apart`, `recursion_distinct_roots` |
| `xs[0..<2]` vs `xs[1]`, overlap | `overlap_slice_against_item` |
| `m[a].balance` vs `m[b].name`, overlap | `overlap_map_entries_fields` |

### M2-05 §1.4: the `&` mark

| Point | Case(s) |
|---|---|
| `&` on a let | `mark_on_let` |
| `&` on a read-only parameter (inside it) | `mark_on_read_only_parameter` |
| `&` on a for name / pattern name | `mark_on_for_name`, `mark_on_pattern_name` |
| `&` on a slice / String position | `mark_on_slice`, `mark_on_string_position` |
| missing `&` | `mark_missing` |
| missing `&` on a labelled argument | `mark_missing_labelled` |
| `&` for a read-only parameter of the callee | `mark_for_read_only_parameter` |
| `&` through a function value | `mark_through_function_value` |
| `&` outside an argument | `mark_outside_argument` |
| `&` with field, List and Map steps, labelled `&` argument | `inout_change_seen_by_caller` |

### M2-05 §1.5: the call rule

| Point | Case(s) |
|---|---|
| Rule A, `&` against a read | `overlap_whole_and_read`, `overlap_read_then_mark`, `overlap_string_item`, `overlap_two_items`, `overlap_slice_of_self`, `overlap_field_item_string`, `overlap_map_entry_read` |
| Rule A, two `&` | `overlap_same_place_twice`, `overlap_two_map_entries`, `overlap_two_int_items_both_inout` |
| Rule B, `&` in a sibling | `read_changed_by_sibling_call` |
| Rule B, order does not matter | `read_changed_order_free` |
| Rule B, String key inside an `&` place | `read_map_key_changed` |
| Rule B, assignment form | `read_changed_by_assignment`, `read_changed_by_match_arm_assignment` |
| Rule C, operator / list / map / record / interpolation / function-value callee | `read_operator_operand`, `read_list_literal_item`, `read_map_literal_value`, `read_record_field`, `read_interpolation_part`, `read_function_value_callee` |
| Rule C, map literal key | `read_map_literal_key` |
| Rule D, accepted forms (`f(&xs[i], g(&xs))`, `f(&xs[pop(&xs)], 1)`, `f(xs[pop(&xs)], 1)`, `push(&st, pop(&st) + pop(&st))`) | `rule_d_resolve_after_operands` |
| Rule D, vanished `&` item traps at the `[` | `rule_d_vanished_item_traps` |
| Rule D, vanished read-only item traps at the `[` | `rule_d_read_item_vanishes_traps` |
| accepted sample lines (exempt reads, fixes) | `exempt_scalars`, `fields_apart`, `fixes_are_accepted` |

### M2-05 §1.6 to §1.9

| Point | Case(s) |
|---|---|
| assignment targets: fields, items, Map entries at any depth | `change_in_place` |
| assigning a let / read-only parameter | `assign_to_let`, `assign_to_read_only_parameter` |
| a variant field is not a place | `assign_to_variant_field`, `assign_to_tree_variant_field` |
| right side runs before the store; overlapping sides defined | `assignment_runs_right_side_first` |
| `xs[len(xs) - 1] = pop(&xs)`: store checks after pop | `assignment_index_before_pop_traps` |
| plain `=` adds a Map key; other steps need it | `change_in_place`, `map_entry_step_needs_key` |
| `swap(&xs, i, j)` / `swap_items(&xs[i], &xs[j])` | `change_in_place`, `overlap_two_int_items_both_inout` |
| §1.7 walked place frozen: `&`, item write, prefix, other item, root | `walk_append_to_walked`, `walk_item_write`, `walk_prefix_mark`, `walk_other_item`, `walk_root_assigned` |
| §1.7 walk over a slice place | `walk_slice_is_a_place` |
| §1.7 fresh walks, fields apart, `keys(m)`, a bound copy | `for_fresh_walk_body_changes_source`, `fields_apart` |
| §1.7 `while` freezes nothing | `while_freezes_nothing`, `graph_sample` |
| §1.7 for name as target | `walk_for_name_assigned` |
| §1.8 arm assigns the subject; names keep their values | `match_arm_changes_subject` |
| §1.9 `no-finite-value`: record, enum, mutual cycle; holder not reported | `finite_record_holds_itself`, `finite_enum_without_exit`, `finite_mutual_cycle`, `finite_holder_not_reported` |
| §1.9 grounded self-holding types | `finite_types_accepted` |
| §1.9 record tree changed in place through `&n.kids[i]` | `record_tree_changed_in_place` |

### M2-05 §1.10 counter-examples

| # | Case(s) |
|---|---|
| 1 | `fresh_values_overlap_nothing` |
| 2 | not observable (a function value carries nothing) |
| 3 | `overlap_two_int_items_both_inout` |
| 4 | `walk_grid_append_row` |
| 5 | `match_arm_changes_subject` |
| 6 | `overlap_index_then_fields` |
| 7 | `recursion_distinct_roots`, `overlap_recursive_slice` |
| 8 | `read_changed_by_sibling_call`, `read_changed_by_assignment`, `read_changed_by_match_arm_assignment` |
| 9 | `rule_d_resolve_after_operands`, `rule_d_vanished_item_traps`, `rule_d_read_item_vanishes_traps` |
| 10 | `read_interpolation_part`, `read_list_literal_item` |
| 11 | `read_slice_changed_in_callee` |
| 12 | `copy_on_return_of_parameter` (with `var b`) |
| 13 | `walk_inout_parameter` |
| 14 | `overlap_map_entry_read`, `overlap_map_prefix`, `fresh_values_overlap_nothing` |
| 15 | `question_mark_payload` |
| 16 | `hole_in_argument` |
| 17 | `trap_while_inout_held` |

### M2-14 rows required here

| Point | Case(s) |
|---|---|
| `extend(&xs, xs)` | `overlap_extend_self` |
| `f(&m[a], m[b])`, `f(&m[a], &m[b])` | `overlap_map_entry_read`, `overlap_two_map_entries` |
| `remove(&m, k)` under `for k in keys(m)` | `for_fresh_walk_body_changes_source` |

### M2-10

| Point | Case(s) |
|---|---|
| §1.1 tree sample, `tree_sample` and `two_copies` | `tree_sample` |
| a value used twice in a tree is two independent copies | `value_used_twice_two_copies` |
| `u.left = Num(0)` refused | `assign_to_tree_variant_field` |
| §1.2 graph sample, `link(&g, g.adj[a][0], b)` accepted | `graph_sample` |
| `for v in g.adj[a] { _ = remove_at(&g.adj[a], i) }` refused | `walk_graph_remove_link` |
| `f(&g.adj[a], g.adj[b])` refused | `overlap_graph_two_lists` |
| a position out of range traps | `graph_position_out_of_range` |
| `Shared`, `Pool`, `Handle` are not reserved | `pool_names_not_reserved` |

## Settled readings

Where the records leave a point open, the owner settled it. The cases follow these readings.

| Reading | Record |
|---|---|
| A place-rooted read of type Int, Float, Bool or a function type is a fresh value. A write inside a callee expression is still caught: the callee is an operand under Rule C | M2-05 §1.2, §1.5 Rule C |
| `inout-overlap` sits at the first token of the later operand: its `&` when it is passed with `&` | M2-05 §5 |
| §1.10 #12 is `var b = id(a)` then `append(&b, 1)` | M2-05 §1.10 |
| A cycle across several types is reported at the first of them in source order | M2-05 §1.9 |
| A walked slice of a place (`for x in xs[0..<2]`) is a place rooted at `xs` | M2-05 §1.2, §1.7 |
| `e?` is fresh | M2-05 §1.2 |

## Not decided

Pools are not built (M2-01 §3), and M2-10 §1.4 leaves a shared immutable type, pools, handles,
the stale-handle rule and pool iteration order not decided. There are no cases for them. The trap
kind `stale_handle` is reserved and nothing raises it.
