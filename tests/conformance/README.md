# Conformance cases

These cases are not yet required: no harness runs them today. M2-41b runs them.

`ownership/` covers ownership and shared data (M2-05, M2-10, the Map rows of M2-14).

`core/` covers values, expressions, control flow and functions (M2-02, M2-03, M2-06, M2-09, M2-12 where they
bear on these).

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
- Reject case: `paka run main.paka` exits with status 1, standard output is empty, and standard
  error is exactly one line that starts with the `reject` text without its line feed, followed by
  `: ` and at least one more byte (spec/m1.md §11).

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

### core: Int (M2-03 §1.2, §1.12, §2.1; M2-06 §1.1)

| Point | Case(s) |
|---|---|
| literals, the 64-bit extremes | `int_literals`, `int_extremes` |
| `+ - *`, unary `-`, results near the extremes | `int_arithmetic`, `int_extremes` |
| `/` truncates, `%` takes the sign of the left side, `MIN % -1` is 0 | `int_division_truncates`, `int_extremes` |
| `+ - *` overflow traps at the operator, detail is the operation | `int_overflow_add`, `int_overflow_sub`, `int_overflow_mul` |
| negation of the lowest Int traps at the `-` | `int_overflow_negate` |
| `MIN / -1` traps `overflow` | `int_overflow_min_div` |
| zero divisor, `/` and `%` | `int_divide_by_zero`, `int_modulo_by_zero` |
| `op=` on Int | `int_compound_assign` |
| `op=` overflow traps at the operator, detail is the long form; `/=` by zero | `int_compound_overflow`, `int_compound_divide_by_zero` |
| comparisons on Int | `int_comparisons` |
| a literal above the range, below it, a negated parenthesis (not folded) | `literal_too_large`, `literal_below_range`, `literal_negated_paren` |
| a leading zero | `literal_leading_zero` |
| `1.`, `.5`, `0x10`, `1e3`, `1_000` | `literal_trailing_point`, `literal_leading_point`, `literal_hex`, `literal_exponent`, `literal_underscore` |
| a Float literal that rounds to infinity | `literal_float_infinite` |
| `let`/`var` with a type | `typed_locals` |

### core: Float and `fixed` (M2-03 §1.3 to §1.6, §2.2 to §2.4)

| Point | Case(s) |
|---|---|
| `+ - * /`, unary `-`, text through `fixed` only | `float_arithmetic` |
| infinities and NaN never trap; their text | `float_never_traps` |
| NaN equals nothing, itself included; every order test is false | `float_nan_compare` |
| `-0.0` equals `0.0`; its sign shows in `fixed` | `float_signed_zero` |
| ties away from zero, no tie when the Float is not exact | `float_fixed_rounding` |
| exact digits, up to 20, no exponent | `float_fixed_exact` |
| `to_float`, `to_int` drop the fraction; the Float/Int bridge | `float_conversions` |
| `to_int(NaN)` traps `bad_conversion` at the function name | `float_to_int_nan_trap` |
| `fixed` digits outside 0 to 20 trap `out_of_bounds`, checked before the Float | `float_fixed_digits_trap` |
| a Float in `"{...}"` | `float_in_interpolation` |
| Int and Float never mix; `%` on Float | `float_int_mix`, `float_needs_point`, `float_modulo` |
| a Float literal too small is 0.0, no error | `literal_float_tiny` |

### core: Bool and String (M2-03 §1.7, §1.9 to §1.11, §1.13, §2.5)

| Point | Case(s) |
|---|---|
| `! && ||`, `== !=` on Bool | `bool_operators` |
| Bool has no order and no `+` | `bool_no_order`, `bool_plus` |
| a condition must be Bool | `condition_not_bool` |
| the seven escapes, `//` inside a string | `string_escapes` |
| a bad escape, an unterminated string, `"{"` alone, a bare `}` | `string_bad_escape`, `string_unterminated`, `string_open_brace_alone`, `string_bare_close_brace` |
| code points: `len`, index, slice | `string_unicode`, `string_index_slice` |
| compare by code point, a proper prefix first | `string_compare` |
| no normalisation | `string_no_normalisation` |
| `+`, `+=` join | `string_concat` |
| `+` with an Int (mixed operands are `type-mismatch` at the operator) | `string_plus_int` |
| interpolation: Int, Bool, String, expressions, `\{` | `string_interpolation` |
| interpolation: a quote inside, an empty one | `string_interpolation_quote`, `string_interpolation_empty` |
| index out of range, a negative index, slice end beyond the length, slice start after the end | `string_index_trap`, `string_index_negative_trap`, `string_slice_end_trap`, `string_slice_order_trap` |
| a String position is not assignable | `string_assign_index` |

### core: expressions (M2-02 §1.5, §1.7; M2-03 §1.13, §2.8; M2-06 §1.3)

| Point | Case(s) |
|---|---|
| precedence: unary, `* / %`, `+ -`, left grouping | `precedence_arithmetic` |
| a `-` before a literal folds; `a -1` still subtracts | `minus_literal_folds` |
| arithmetic over comparison over `&&` over `||` | `precedence_logic` |
| comparisons do not chain | `comparison_chain` |
| `&&` and `||` skip the right side | `short_circuit` |
| `&&` runs the right side when the left does not decide | `short_circuit_trap` |
| left operand before right: first trap wins | `order_left_trap_first` |
| call arguments, left to right | `order_call_arguments` |
| right side of `=` before the store's check | `order_assignment_right_first` |
| interpolation parts, left to right | `order_interpolation_parts` |
| range ends, start first, before the first step | `order_range_ends` |
| a line continues after an operator, `,`, `=`, `=>`, `|`, inside `(` `[` | `newline_continuation` |
| a line starting with an operator does not continue | `newline_ends_statement` |
| comments | `comments_and_blank_lines` |
| nesting: 64 levels fit, the 65th does not | `nesting_limit_accepted`, `nesting_too_deep` |
| nesting: the `{` of an interpolation is a level; an `if` in the condition of another is a level | `nesting_interpolation_counts`, `nesting_header_if` |
| a chain: 1,000 links fit, the 1,001st does not | `chain_limit_accepted`, `chain_too_long` |
| `..` is not `..<` | `range_two_dots` |

M2-41 reuses `order_call_arguments` and `order_assignment_right_first`.

### core: control flow (M2-02 §1.3, §1.4, §1.9, §5; M2-03 §2.8; M2-12 §1.3)

| Point | Case(s) |
|---|---|
| `if`, `else if`, `else` as statements | `if_statement` |
| `if` as a value | `if_value` |
| only the chosen branch runs | `if_evaluates_one_branch` |
| value `if` needs `else`; branch types agree; `else` on the line of `}` | `if_needs_else`, `if_branches_differ`, `else_on_new_line` |
| `while` | `while_loop` |
| `for` over a range: empty, reversed, negative, expression ends | `for_range` |
| range ends evaluated once | `for_range_ends_once` |
| range next to the Int extremes | `for_range_at_limits` |
| `for` over a List, an empty List | `for_over_list` |
| `break` and `continue` leave the innermost loop | `nested_loops_break_continue` |
| `break`/`continue` outside a loop | `break_outside_loop` |
| a statement after `break`, `continue`, `return` | `unreachable_after_break`, `unreachable_after_continue`, `unreachable_after_return` |
| `match` on Int (with `-1` and `|`) | `match_int` |
| `match` on String, on Bool | `match_string`, `match_bool` |
| arm forms: expression, assignment, block, `{}` | `match_arm_forms` |
| `break`, `continue` in arm blocks | `match_loop_control` |
| `return` in an arm block | `match_arm_return` |
| no comma after an arm; `=> break`, `=> return` without a block | `comma_after_arm`, `arm_break_needs_block`, `arm_return_needs_block` |
| missing arm on Int, on Bool | `missing_arm_int`, `missing_arm_bool` |
| duplicate arm, an arm after `_`, `_` after `true` and `false` | `duplicate_arm`, `arm_after_wildcard`, `wildcard_after_both_bools` |
| a pattern of the wrong type; a Float subject; arms of different types | `match_pattern_wrong_type`, `match_subject_float`, `match_arms_differ` |
| an empty `match` | `match_empty` |
| sibling blocks reuse a name | `scope_sibling_blocks` |
| a local hides a function; calling it | `local_hides_function`, `local_hides_function_call` |
| `duplicate-local` for `let`, `for` name, enclosing block, a parameter | `duplicate_local_let`, `duplicate_local_for`, `duplicate_local_block`, `duplicate_local_parameter` |
| assigning a `let`, a parameter, a range `for` name | `assign_to_let`, `assign_to_parameter`, `assign_to_for_name` |
| a binding is not visible in its own value | `local_not_visible_in_own_initializer` |
| a dropped value; `_ =` on a call without a value; `_ =` on a value | `unused_result_call`, `nothing_to_discard`, `discard_value_accepted` |
| a call without a result used as a value | `let_of_no_result_call` |

### core: functions (M2-02 §1.1, §5; M2-06 §1.2, §1.3)

| Point | Case(s) |
|---|---|
| results of each value type, `f()` without parameters | `function_result_types`, `call_labels` |
| labels after `;`, in declared order | `call_labels` |
| a bare argument in a label slot, a label on a bare parameter, labels out of order | `wrong_label_bare`, `wrong_label_on_bare`, `wrong_label_order` |
| labels are tested before the argument types | `wrong_label_before_type` |
| argument count, tested before labels | `wrong_argument_count_few`, `wrong_argument_count_many`, `wrong_argument_count_before_label` |
| `inout` parameters, forwarded `&`, Int and String | `inout_calls` |
| `return` early: in `for`, in `while`, bare | `early_return` |
| `return` on the last line, bare, also through a branch or a `match` arm | `return_last`, `return_last_bare`, `return_last_in_branch`, `return_last_in_match_arm` |
| `return` with a value in a function without a result | `return_value_without_result` |
| a block that must end in a value; a single-line arm that gives none; a bare `return` in a function with a result | `missing_value`, `match_arm_no_value`, `bare_return_needs_value` |
| recursion, mutual recursion, definitions in any order | `recursion` |
| a deep but finite recursion | `recursion_depth` |
| trap report inside calls: chain of frames | `factorial_overflow`, `trap_frame_chain` |
| the chain stops at eight names and ends in `, ...` | `trap_frames_cut` |

## Settled readings

Where the records leave a point open, the owner settled it. The cases follow these readings. A row
that starts with "Case reading" is a choice the `core/` cases made; it waits for the owner's
confirmation.

| Reading | Record |
|---|---|
| A place-rooted read of type Int, Float, Bool or a function type is a fresh value. A write inside a callee expression is still caught: the callee is an operand under Rule C | M2-05 §1.2, §1.5 Rule C |
| `inout-overlap` sits at the first token of the later operand: its `&` when it is passed with `&` | M2-05 §5 |
| §1.10 #12 is `var b = id(a)` then `append(&b, 1)` | M2-05 §1.10 |
| A cycle across several types is reported at the first of them in source order | M2-05 §1.9 |
| A walked slice of a place (`for x in xs[0..<2]`) is a place rooted at `xs` | M2-05 §1.2, §1.7 |
| `e?` is fresh | M2-05 §1.2 |
| A function parameter has no default value; a default exists only on a record field | M2-02 §2 (`param` has none, `field` has one) |
| A binary operator whose operand types are not allowed together is `type-mismatch` at the operator: Int with Float, String with Int, an order test or `+` on Bool, `%` on Float | M2-03 §1.13 |
| A folded literal starts at its `-`, so `-9223372036854775809` is `literal-out-of-range` at the `-` | M2-03 §1.12 (the parser folds the `-` into the literal) |
| Negating the lowest Int traps `overflow` at the unary `-`, detail `-(-9223372036854775808)` | M2-06 §1.1, §1.2 |
| A condition that is not Bool is `type-mismatch` at the condition's first token | spec/m1.md §10, M2-02 §1.8 |
| Branches of one `if` or arms of one `match` that differ in type are `type-mismatch` at the later branch's value | M2-02 §5 (branch types), spec/m1.md §10 |
| A pattern of the wrong type is `type-mismatch` at the pattern | M2-02 §5 (`missing-arm`) |
| `duplicate-local` is at the name of the second binding | M2-02 §1.3, §5 |
| `missing-value` at a single-line arm is at the arm's first token, its pattern | M2-02 §5 |
| The body `{` of a function is nesting level 1, so 63 `(` in a body fit and the 64th is `nesting-too-deep` | spec/m1.md §5, M2-02 §1.7 |
| An `if` or `match` standing in the header counts as a level; the one holding it adds none | spec/m1.md §10, M2-02 §1.7 |
| A slice whose start is after its end traps `out_of_bounds` with detail `slice 2..<1, length 3` | M2-06 §1.1, §1.2 |
| After `{` in a string the lexer looks for the first `}` on the line; with none, the string is `unterminated-string` at the opening quote (`"{"`); with one, a `"`, `{` or `}` before it is `unexpected-character` at that byte (`"{"a"}"`) | M2-03 §1.11 |
| Case reading: `to_int` and `to_float` appear as the Int/Float bridge; the rest of their rows belong to the built-in function cases | M2-03 §1.5 |

## Not decided

Pools are not built (M2-01 §3), and M2-10 §1.4 leaves a shared immutable type, pools, handles,
the stale-handle rule and pool iteration order not decided. There are no cases for them. The trap
kind `stale_handle` is reserved and nothing raises it.

The `core/` cases leave out what the records do not pin: the wording of every message. The habit
spellings `;` and a trailing comma are held by `lex/semicolon` and `parse/trailing_comma`. The call-depth case belongs to M2-41 and is compared on kind and status only
(M2-06 §1.4). Shift traps and the other built-in rows belong to the built-in function cases.
