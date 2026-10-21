# Conformance cases

`paka/tests/conformance.rs` runs every case by the pass rules below, with none skipped.

`ownership/` covers ownership and shared data (M2-05, M2-10, the Map rows of M2-14).
`errors/` holds one rejected program for every error kind a program file can raise (M2-42).

`core/` covers values, expressions, control flow and functions (M2-02, M2-03, M2-06, M2-09, M2-12 where they
bear on these).

`builtins/` covers the built-in functions and collection types (M2-14): every row of its table, the
edges of its text, List, Map, sort and trap rules, the seven samples, and the check-time rejections
it pins.

`types/` covers user types, `match`, generics, function values and errors (M2-02, M2-09, M2-12, the
type rows of M2-14).

`apps/` covers apps, effects, grants and refusals, the answer queue, the eight traps, `paka examples`
and holes (M2-06, M2-15, with spec/m1.md). The cases here reuse `core/order_call_arguments` and
`core/order_assignment_right_first` by citation.

## Case format

A case is a folder `<area>/<name>/`, `<name>` matching `[a-z0-9_]+`, holding a full program in
`main.paka` (an app, shaped like `tests/cases/run/*`).

- A run case adds `stdout`, `status` (decimal digits and a line feed) and, only when standard error
  is not empty, `stderr`. Status 0 cases show their behaviour through printed output; status 3
  cases hold the trap report of M2-06 §1.2 byte for byte. A run case for a `stack` or `memory`
  trap holds `trap` instead of `stderr`: the kind and one line feed. Its `status` is 3.
- A reject case adds `reject` alone: `main.paka:LINE:COL: error[CODE]` and one line feed. Only the
  code and position are pinned; message texts are not, since the records fix most of them only in
  parts.
- A reject line with no position, `main.paka: error[CODE]`, is for the kinds that have none
  (`no-app`).

Files added by the `apps/` cases. Each is optional.

- `args`: the arguments after `main.paka`, one per line, each ended by a line feed. A lone line feed
  is one empty argument. The bytes are passed as they are, so a line may hold an invalid UTF-8
  byte. No file means no argument.
- `command`: one line, the words after `paka`, replacing the default `run main.paka` and `args`
  (`check main.paka`, `examples main.paka`, `build main.paka -o out`).
- `files/`: before the command the case folder is copied to a fresh folder, and the contents of
  `files/` are copied into its root, keeping sub-folders; `files/` itself is not there. The command
  runs in that folder. A name such as `~` or `$PAKA_DIR` is a plain folder name.
- `after/`: files that must exist after the command with exactly these bytes, same relative paths.
- `absent`: paths, one per line, that must not exist after the command.
- `present`: paths, one per line, that must exist after the command.

## Pass rules

The case folder is the working directory, so every diagnostic path is `main.paka`.

- Run case: `paka run main.paka` passes when standard output equals `stdout` byte for byte, the
  exit status equals `status`, and standard error equals `stderr` byte for byte (empty when the
  case has no `stderr` file). The format is spec/m1.md §11.
- Run case with `trap`: standard output equals `stdout`, the exit status is 3, and standard error
  is exactly one line `<app>: trap <kind> at main.paka:LINE:COL` and a line feed, with `<kind>` the
  `trap` text and any LINE and COL. This extends spec/m1.md §11.
- Reject case: `paka run main.paka` exits with status 1, standard output is empty, and standard
  error is exactly one line that starts with the `reject` text without its line feed, followed by
  `: ` and at least one more byte (spec/m1.md §11).
- Reject case with `command`: the same, with that command; the status is 1. A reject line with no
  position is matched as a prefix of the first line of standard error, and the line may end there.
  Standard error is still that one line, except that with `check` it may be followed by the field
  lines of spec/paka.md §12, each of the form `  NAME: value`: two spaces, a lowercase field name,
  `: ` and the value.
- Case with `command` that is not a reject: standard output equals `stdout`, standard error equals
  `stderr` (empty when absent), the status equals `status`. An empty `stdout` file means empty.
- `files/`, `after/`, `absent` and `present` are checked as the format says, on top of the above.

## Exit gate (M2-44)

Two groups of tests, all in `paka/tests/conformance.rs` (code in
`paka/tests/conformance/gate.rs`): `exit_gate` checks the three parts of the M2 exit check that are
not a case, reports everything that fails in one message and fails if anything does, and the
self-tests next to it plant one fault each and expect the gate to fail: an unused node kind, an
unused built-in, a sample that does not check, a sample that traps, an app sample whose run exits
non-zero, an oversize reference. Command, which runs the gate and its self-tests:

    cargo test -p paka --test conformance --locked --offline gate

It is a plain test, so `cargo test --workspace --locked --offline` runs it too.

- **Reference samples.** A block of `spec/paka.md` fenced as `paka` is a sample (the reference
  says so in its first paragraph); no other block is. A sample checks and runs when, saved as
  `main.paka` in a fresh folder, `paka check main.paka` and `paka examples main.paka` both exit 0,
  and, if it has an `app`, `paka run main.paka words.txt` also exits 0, with `words.txt` in that
  folder holding the text `a b c`.
- **Size.** `spec/paka.md` is at most 8,000 tokens under the one counting rule: bytes divided by
  three (M2-18, `paka/tests/reference.rs`). The test rounds the quotient up, so the budget is
  24,000 bytes; rounding down would allow two bytes more.
- **Coverage.** The syntax node kinds are the variants of the node enums of `paka/src/ast.rs`
  (definitions, app entries, types, statements, expressions, string parts, arm bodies, patterns
  and the three operator enums), each written `Enum::Variant`. The built-ins are the distinct
  names in the table of `paka/src/builtins.rs`: types, variants, grants and functions. A name is
  one item however many rows it has (`len` has three), so the gate counts names, not rows. Both
  lists come from the compiler's own code, so a new variant or row is a gap until a case uses it.
  A case counts when it passes by the rules above, has neither a `reject` file nor an
  `expect.json`, and its `main.paka` parses. A node kind is covered when it occurs in the syntax
  tree of a counted case. A built-in type, variant or grant (upper case) is covered when its name
  occurs in one as a type, a name in an expression or pattern, a record name or a grant. A
  built-in function (lower case) is covered only when it is the callee of a call there, since a
  local name may hide a built-in function. A case that expects a refusal through `reject` or
  `expect.json` covers nothing. A case that expects a failing status with no such file counts:
  the `apps/hole_*_build` cases, whose build is refused, are of this kind.

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

### M2-14 §1.2 to §1.3: reading a row, shared rows

| Point | Case(s) |
|---|---|
| `len` on String (code points), List, Map | `len_string_list_map` |
| `is_empty` on the three types | `is_empty_three_types` |
| `contains` on String, empty part, non-ASCII | `contains_string` |
| `contains` on List, `contains([], 1)`, a NaN is never found | `contains_list` |
| `find` on String, empty part, overlap, non-ASCII | `find_string` |
| `find` on List, `find([], 1)` | `find_list` |
| a type with no row: `len(5)`, `abs("x")` | `len_of_int`, `abs_of_string` |
| `len([])` and `len([:])` written bare | `len_of_empty_list`, `len_of_empty_map_literal` |
| `min(1, 2.0)` | `min_of_int_and_float` |
| a built-in is not a value | `builtin_as_value`, `builtin_as_sort_by_function` |
| a user definition of a row's name; a type named like a built-in type | `user_function_named_like_row`, `user_type_named_option` |
| a local may hide a built-in; calling it is `not-a-function` | `local_hides_builtin`, `local_hides_builtin_call` |
| an unlisted habit name | `unknown_habit_name` |
| the labels of `fixed`, `insert` and `list_of` are required | `fixed_digits_unlabelled`, `insert_at_unlabelled`, `list_of_count_unlabelled` |
| a dropped result needs `_ =` | `pop_result_dropped`, `remove_at_result_dropped` |

### M2-14 §1.4: number rows

| Point | Case(s) |
|---|---|
| `min`, `max` on Int, Float, String | `min_max_ints`, `min_max_floats`, `min_max_strings` |
| `abs`, the smallest Int traps | `abs_rows`, `abs_smallest_int` |
| `to_float` | `to_float_rows` |
| `to_int`, NaN, infinity, out of range | `to_int_rows`, `to_int_nan`, `to_int_infinity`, `to_int_too_large`, `to_int_two_to_63`, `to_int_below_min` |
| `sqrt` (with `fixed` to 20 digits), `floor`, `ceil`, `trunc`, `round`, `floor` of a large Float | `sqrt_rows`, `floor_rows`, `floor_large`, `ceil_rows`, `trunc_rows`, `round_rows` |
| `fixed`, digits 0 to 20, digits outside | `fixed_rows`, `fixed_digits_21`, `fixed_digits_negative` |
| `wrap_add`, `wrap_sub`, `wrap_mul` | `wrap_rows` |
| `bit_and`, `bit_or`, `bit_xor`, `bit_not` | `bit_rows` |
| `shift_left`, `shift_right`, count outside 0 to 63 | `shift_rows`, `shift_left_64`, `shift_right_negative` |

### M2-14 §1.5: text to number and characters

| Point | Case(s) |
|---|---|
| `parse_int` shape, limits, non-ASCII digits | `parse_int_rows` |
| `parse_float` shape, huge value, no exponent | `parse_float_rows` |
| `code_at`, non-ASCII; bad position | `code_at_rows`, `code_at_past_end`, `code_at_empty_text`, `code_at_negative` |
| `from_code`, controls, the ends of the range; outside the range or a surrogate | `from_code_rows`, `from_code_too_large`, `from_code_surrogate`, `from_code_negative` |

### M2-14 §1.6 to §1.7: text rows

| Point | Case(s) |
|---|---|
| `split`: empty pieces, empty `sep`, no overlap, non-ASCII | `split_rows` |
| `chars` | `chars_rows` |
| `lines`, a carriage return stays | `lines_rows` |
| `words`, the six white-space characters, U+00A0 stays | `words_rows` |
| `join` | `join_rows` |
| `repeat`; a count below zero | `repeat_rows`, `repeat_negative_count` |
| `pad_left`, `pad_right`: zero, negative width, non-ASCII | `pad_rows` |
| `trim`, `trim_start`, `trim_end` | `trim_rows` |
| `starts_with`, `ends_with` | `starts_ends_rows` |
| `replace`: no rescan, empty `old` | `replace_rows` |
| `upper`, `lower`: ASCII only | `upper_lower_rows` |

### M2-14 §1.8: List rows

| Point | Case(s) |
|---|---|
| `append` | `append_rows` |
| `extend`, with a bound copy | `extend_rows` |
| `pop`; on an empty List | `pop_rows`, `pop_empty` |
| `insert`, `at == len`; past the end, negative | `insert_rows`, `insert_past_end`, `insert_negative_position` |
| `remove_at`; past the end, negative, empty | `remove_at_rows`, `remove_at_past_end`, `remove_at_negative`, `remove_at_empty` |
| `swap`; either position bad, both bad, equal and bad, negative, empty | `swap_rows`, `swap_first_position_bad`, `swap_second_position_bad`, `swap_both_positions_bad`, `swap_equal_positions_out_of_range`, `swap_negative_position`, `swap_empty_list` |
| `reverse` | `reverse_rows` |
| `list_of`: independent copies, zero; a count below zero | `list_of_rows`, `list_of_negative_count` |
| `sort` on Int, String, Float | `sort_ints`, `sort_strings`, `sort_floats` |
| `sort` on `-0.0` and NaN | `sort_floats_zero_and_nan` |
| `sort` on an item type with no order | `sort_of_bools` |
| index and slice bounds | `index_past_end`, `index_negative`, `index_write_past_end`, `slice_rows`, `slice_past_end`, `slice_start_after_end`, `slice_negative_start` |

### M2-14 §1.9 to §1.10: Map, Option, Result, assert

| Point | Case(s) |
|---|---|
| `get`, `has`, `remove`, a missing key | `get_has_remove`, `int_keys_get_remove` |
| `keys`, `values` of an empty Map | `empty_map` |
| `keys`, `values`, `get` give copies | `get_and_values_are_copies` |
| `unwrap_or`, `T` from the other argument | `unwrap_or_rows` |
| `unwrap_or` evaluates both arguments | `unwrap_or_evaluates_fallback` |
| `ok_or` | `ok_or_rows` |
| `assert` true; false | `assert_true_continues`, `assert_false`, `assert_false_in_helper` |
| `assert` with a second argument | `assert_two_arguments` |
| `unreachable`: not reached, as a value, as a statement | `unreachable_not_reached`, `unreachable_reached_as_value`, `unreachable_reached_as_statement` |
| `unreachable` with no expected type; `None` with no type | `unreachable_without_type`, `none_without_type` |
| `unreachable` takes no argument | `unreachable_with_argument` |

### M2-14 §1.11 to §1.12: Map order, keys, literal, index; walking

| Point | Case(s) |
|---|---|
| Int keys by signed number, including the extremes | `key_order_ints` |
| String keys by code point, non-ASCII | `key_order_strings` |
| equal Maps walk and compare alike | `equality_ignores_insertion_order` |
| a later equal key in a literal replaces; both values are evaluated | `literal_duplicate_and_order`, `literal_duplicate_key_evaluates_both` |
| `m[key] = v` adds or replaces; steps need the key; nested Map | `assign_adds_and_replaces` |
| `m[key]` missing: String key, Int key, negative key, empty Map | `read_missing_string_key`, `read_missing_int_key`, `read_missing_negative_key`, `read_missing_key_in_empty_map` |
| `m[key] += 1` on a new key; the count idiom | `plus_equals_on_new_key`, `count_idiom` |
| `m[a][b] = v` with `a` missing; the right side runs first | `nested_assign_missing_first_key`, `nested_assign_right_side_first` |
| `m[key].field = v` with the key missing | `field_assign_missing_key` |
| `&m[key]` with the key missing | `mark_missing_key` |
| `for k in keys(m)` walks in key order | `keys_walk_in_order` |
| a Map key type other than Int or String | `map_key_type_in_annotation`, `map_key_type_in_literal` |
| literal with mixed value types; `[:]` without a type | `map_literal_value_type`, `map_empty_literal_untyped` |
| `for` over a String or a Map | `for_over_string`, `for_over_map` |

### M2-14 §1.13: the sort

| Point | Case(s) |
|---|---|
| stable on equal keys, ascending and descending | `sort_by_stable` |
| `sort_by` with a named function, code point lengths | `sort_by_strings_by_length` |
| a function that always answers `true` reverses; `[3, 1, 2]` becomes `[2, 1, 3]` | `sort_by_always_true` |
| a function that always answers `false` changes nothing | `sort_by_always_false` |
| a function that contradicts itself: the call sequence decides | `sort_by_inconsistent_function` |
| no call under two items | `sort_by_not_called_under_two_items` |
| a trap inside the function, where it traps | `sort_by_function_asserts`, `sort_by_function_traps_at_second_call` |

### M2-14 §1.14: text rules

| Point | Case(s) |
|---|---|
| white space is the six code points 9 to 13 and 32 | `trim_rows`, `words_rows` |
| case maps ASCII only | `upper_lower_rows` |
| matching: exact code point, earliest, no overlap | `find_string`, `contains_string`, `split_rows`, `replace_rows`, `starts_ends_rows` |
| positions and lengths count code points; order by code point | `len_string_list_map`, `find_string`, `code_at_rows`, `sort_strings`, `key_order_strings` |

### M2-14 §1.15: growth and the cap

Cases test the cap check only with sizes no cap could allow: a count or width of 2^62 or more, or a
size product that overflows Int. The cap's exact value and growth just past it are unit tests.

| Point | Case(s) |
|---|---|
| `repeat`: the exact product overflows Int | `repeat_product_overflows` |
| `pad_left`, `pad_right` with a huge width | `pad_left_huge_width`, `pad_right_huge_width` |
| `list_of` with a huge count | `list_of_huge_count` |

### M2-14 §4: the seven samples

| Sample | Case |
|---|---|
| 1 text processing | `sample_word_counts` |
| 2 parsing | `sample_sum_expression` |
| 3 simulations | `sample_traffic_light` |
| 4 grids | `sample_min_path_sum` |
| 5 graphs | `sample_bfs_order` |
| 6 reports | `sample_float_report` |
| 7 file tools | `sample_strip_trailing_space` |

### M2-14 §8, the M2-39b line

| Point | Case(s) |
|---|---|
| the sort with a contradicting function | `sort_by_always_true`, `sort_by_inconsistent_function` |
| Map key order with negative Ints and non-ASCII Strings | `key_order_ints`, `key_order_strings` |
| `+=` on a new key | `plus_equals_on_new_key` |
| a duplicate key in a literal | `literal_duplicate_and_order` |

## apps/: apps, effects, grants, traps, examples and holes

Sources: M2-15, M2-06 §1.1 to §1.6, spec/m1.md §8 to §9, M2-12 for the hole and example forms. A
case name below is a folder in `apps/`. A `*` stands for the cases of that name prefix, listed in
full in the folder. `(Rn)` marks a case that follows a reading in the Settled readings table below.

### The app form (M2-15 §1.1, spec/m1.md §8)

| Point | Case(s) |
|---|---|
| a light app runs; keys in any order; `state` optional; empty `grants` | `app_light`, `app_keys_any_order`, `app_empty_grants`, `app_grants_unused` |
| `start` may be any variant of the start type; its text fields are filled from the arguments | `app_start_second_variant` |
| `update` may have any name | `app_update_other_name` |
| state starts from its field defaults and lives across handler calls | `app_state_defaults`, `app_state_persists` |
| a key twice is `duplicate-app-entry`, before a missing key | `app_duplicate_start`, `app_duplicate_grants`, `app_duplicate_before_missing` |
| a missing key is `missing-start`, `missing-update` or `missing-grants`, in that order, at the app name | `app_missing_start`, `app_missing_update`, `app_missing_grants`, `app_missing_start_before_grants`, `app_missing_update_before_grants`, `app_empty` |
| an unknown entry is `unexpected-token` at the key: M2-02 §5 retired `unsupported-app-entry` | `app_unknown_entry` |
| `state` must be a record with defaults on every field, checked before `update` | `state_is_int`, `state_is_enum`, `state_field_without_default`, `state_function_field` (R1), `state_before_update_signature` |
| `update` is a known function, `(inout State, Msg) -> Effects`, or `(Msg) -> Effects` with no state | `update_unknown_name`, `update_returns_int`, `update_state_needs_inout`, `update_state_given_one_parameter`, `update_without_state_has_two`, `update_wrong_state_type`, `update_message_not_an_enum`, `update_effect_of_other_message`, `update_signature_before_start` |
| `start` is a variant with String fields only; an unknown name, a function and a non-String field are told apart | `start_unknown_name`, `start_is_function`, `start_field_not_string`, `start_before_grants` (R9) |
| no app: `check` passes, `run` and `build` give `no-app` after the program is checked | `no_app_check_ok`, `no_app_run`, `no_app_build`, `no_app_empty_file`, `no_app_error_first`, `no_app_check_error_first` (R7) |
| two apps stay a check error | `multiple_apps`, `multiple_apps_check` |
| a visual app (M5-01): the counter sample checks clean | `view_counter_check` |
| UI examples (M5-06): the to-do sample checks clean and its examples pass with no window (M5-22) | `ui_todo_check` |

### Arguments and the first message (M2-15 §1.4)

| Point | Case(s) |
|---|---|
| each field takes its argument by position | `args_fill_fields`, `args_order_by_position` |
| an empty argument is a field of "" | `args_empty_argument` |
| an argument that is not text is a usage error, status 2 | `args_not_text` |
| the argument count must equal the field count; status 2, text `expected n argument(s) (fields), got m` | `usage_missing_one`, `usage_missing_of_two`, `usage_none_of_two`, `usage_too_many`, `usage_one_extra_for_one`, `usage_fieldless`, `usage_fieldless_two` |

### Effects, the queue and ending (M2-15 §1.5 to §1.8)

| Point | Case(s) |
|---|---|
| `Print` adds one line feed; empty text; multi-byte text; interpolation; order of effects | `print_adds_one_line_feed`, `print_empty_text`, `print_unicode_bytes`, `print_interpolated`, `print_in_order`, `print_before_answer` |
| `Exit` and `ExitWith` need no grant and drop the later effects | `exit_plain`, `exit_needs_no_grant`, `exit_drops_rest`, `exit_before_refused_effect` |
| `ExitWith` 0 and 5 to 255 end with that status | `exit_with_0`, `exit_with_5`, `exit_with_101`, `exit_with_255`, `exit_with_needs_no_grant`, `exit_with_drops_rest` |
| a literal other than 0 or 5 to 255 is rejected at check time | `exit_with_literal_one`, `exit_with_literal_four`, `exit_with_literal_256`, `exit_with_literal_negative` |
| a computed status outside 0 or 5 to 255 stops the program, status 3, `effect ExitWith failed` | `exit_with_computed_one`, `exit_with_computed_two`, `exit_with_computed_three`, `exit_with_computed_four`, `exit_with_computed_256`, `exit_with_computed_negative`; and the accepted ends `exit_with_computed_zero`, `exit_with_computed_five`, `exit_with_computed_255` |
| the list ends without `Exit`: status 3, `ended without Exit` | `ended_empty_list`, `ended_after_print`, `ended_after_answer`, `ended_after_failed_answer` |
| a file answer joins the back of the queue and is applied at the front, just before `update` | `answers_in_order_mixed`, `answers_in_order_two_missing`, `answer_queued_behind_existing`, `then_applied_at_front`, `or_applied_at_front` |
| `Exit` drops queued answers | `exit_drops_queued_answers`, `exit_with_drops_queued_answer` |
| a write done before an `Exit` stays done; a failed write dropped by `Exit` gives no answer | `write_failure_dropped_at_exit`, `write_then_read_same_list`, `read_then_write_same_list` |

### Grants and refusals (M2-15 §1.3, spec/m1.md §9.4)

| Point | Case(s) |
|---|---|
| `Print` without Console, `ReadFile` without ReadFiles, `WriteFile` without WriteFiles are refused: status 3, `effect E refused: grant G is missing`, earlier output kept | `grant_print_refused`, `grant_print_refused_with_files`, `grant_read_refused`, `grant_read_refused_with_write`, `grant_write_refused`, `grant_write_refused_with_read` |
| a refusal stops at that effect: later effects and queued answers are dropped, and nothing is written | `grant_refused_drops_queue`, `grant_refused_in_answer` |
| a missing grant for an effect never reached does nothing | `grant_effect_unreached` |
| an unknown grant is `unknown-grant`; a repeated grant is `duplicate-grant`; left to right, so the first fault wins | `grant_unknown`, `grant_unknown_files`, `grant_unknown_first_of_two`, `grant_unknown_second`, `grant_duplicate`, `grant_duplicate_third`, `grant_unknown_before_duplicate`, `grant_duplicate_before_unknown` |

### ReadFile and WriteFile (M2-15 §1.7)

| Point | Case(s) |
|---|---|
| a read gives the text; an empty file; text with multi-byte characters; a path in a sub-folder; the program's own source | `read_text`, `read_empty_file`, `read_utf8_file`, `read_subdirectory`, `read_own_source` |
| `failed`: empty path, NUL in the path | `read_empty_path`, `read_nul_path`, `write_empty_path`, `write_nul_path` |
| `not found`: missing file, missing folder, a file used as a folder | `read_missing`, `read_missing_directory`, `read_not_a_directory_prefix`, `write_missing_directory`, `write_not_a_directory_prefix` |
| `not a file`: a folder; a device on read | `read_directory`, `read_device`, `write_directory` |
| `not text`: invalid UTF-8, a surrogate, a cut sequence (read only) | `read_not_text`, `read_not_text_surrogate`, `read_not_text_truncated` |
| paths are used as written: `~` and `$VAR` are not expanded | `read_path_tilde`, `read_path_dollar` |
| a write creates, replaces, writes empty text and UTF-8 text, in a sub-folder | `write_creates`, `write_replaces`, `write_empty_text`, `write_utf8`, `write_subdirectory`; a write adds no line feed: `write_adds_nothing` |
| `then:` on a write carries a value into the message | `write_then_carries_value` |
| a failed write is told to `or:`, and the file is not created | `write_missing_directory` |

### Traps (M2-06 §1.1 to §1.3, M2-15 §1.9)

| Point | Case(s) |
|---|---|
| each kind with its report text and status 3: overflow, divide_by_zero, out_of_bounds, bad_conversion, assert, hole | `trap_overflow`, `trap_overflow_multiply`, `trap_divide_by_zero_remainder`, `trap_out_of_bounds`, `trap_out_of_bounds_pop`, `trap_bad_conversion`, `trap_assert`, `trap_unreachable`, `hole_run_trap` |
| `stack` and `memory` by kind and status only (a `trap` file); the call depth and collection numbers have no case (M2-06 §1.4) | `trap_stack_recursion`, `trap_stack_mutual`, `trap_stack_after_output`, `trap_stack_in_then`, `trap_stack_drops_handler_effects`, `trap_memory_repeat`, `trap_memory_repeat_product_overflows`, `trap_memory_list_of`, `trap_memory_after_output`, `trap_memory_drops_handler_effects` |
| `&&` and `\|\|` skip the right side; `\|\|` runs it on false | `trap_and_skips_right`, `trap_or_skips_right`, `trap_or_runs_right`; also `core/short_circuit`, `core/short_circuit_trap` |
| order is strictly left to right | `trap_left_division_first`, `trap_left_index_first`, `trap_second_argument_after_first_ok`; also `core/order_call_arguments`, `core/order_assignment_right_first`, `core/order_left_trap_first` |
| a trap in a handler: that handler's effects are not done, earlier output stays, the program ends | `trap_drops_handler_effects`, `trap_keeps_earlier_output`, `trap_ends_program_queue_dropped` |
| a trap in a `then:` or `or:` function names it as the outermost frame | `trap_in_then`, `trap_in_or` |
| the call chain: eight names exactly, nine names end in `, ...`; the chain itself is `core/trap_frame_chain`, `core/trap_frames_cut` | `trap_frames_exactly_eight`, `trap_frames_nine` |
| the detail line is cut at 256 bytes (R3) | `trap_detail_cut_at_256` |

### paka examples (M2-06 §1.6)

| Point | Case(s) |
|---|---|
| the four statuses and the summary; exit 3 when any is not a pass | `examples_counter`, `examples_counter_fixed` (R2), `examples_trapped` (R6), `examples_unfinished` |
| output is in byte order of the names; no examples | `examples_order_by_name_bytes`, `examples_none` |
| a directory is one program; places name the file they are in and count lines in it | `examples_directory_program` |
| `fail` is at the comparison operator with `left:` and `right:` in the value text form | `examples_fail_comparisons`, `examples_fail_value_text` |
| a non-comparison fails at its first token, with no help lines; an `&&` chain reports the last operand evaluated | `examples_fail_not_a_comparison`, `examples_fail_and_chain` |
| a false `assert` is `trapped`, not `fail` | `examples_assert_is_trapped` |
| an example may compare an effect list | `examples_effects_compare` |
| `run`, `check` and `build` do not run examples | `examples_not_run_by_run`, `examples_not_run_by_check`, `examples_not_run_by_build` |
| an example body is one Bool expression: not Bool, ends in a statement, empty, `?`, a name not in scope, a repeated name, a name that is a function; a check error runs nothing | `example_body_not_bool`, `example_body_ends_in_statement`, `example_empty`, `example_question_mark`, `example_name_not_in_scope`, `example_duplicate_name`, `example_name_clashes_with_function`, `example_check_error_runs_nothing` |
| `return` in an example body is allowed | `example_return_allowed` |

### Holes (M2-06 §1.5)

| Point | Case(s) |
|---|---|
| the one program with `check` note, `run` trap and `build` refusal | `hole_check_note`, `hole_run_trap`, `hole_build_refused` |
| a statement hole: "takes no value" | `hole_statement`, `hole_statement_run` |
| a hole not reached: `check` notes it, `run` passes, `build` refuses | `hole_cold_branch_check`, `hole_cold_branch_run`, `hole_cold_branch_build` |
| several holes: one note each, one build line each in position order (R4); one name twice | `hole_check_two_notes`, `hole_build_two_holes`, `hole_same_name_twice_build` |
| output printed before a hole is kept | `hole_trap_keeps_output` |
| a hole takes its type from its place | `hole_type_argument`, `hole_type_later_argument`, `hole_type_let_annotation`, `hole_type_condition`, `hole_type_other_operand`, `hole_type_logic_operand`, `hole_type_range_side`, `hole_type_last_expression`, `hole_type_list_item`, `hole_type_record_field`, `hole_type_return`, `hole_type_assignment`, `hole_type_match_arm` |
| no place to type it from: `cannot-infer` at the `??` | `hole_cannot_infer_let`, `hole_cannot_infer_len_argument`, `hole_cannot_infer_field_base`, `hole_cannot_infer_negation`, `hole_cannot_infer_interpolation`, `hole_cannot_infer_index_base` |
| `check --json` prints a hole note as one object of kind `hole`, with the hole's type or "takes no value" in `message` (M2-06 §1.5) | `hole_check_json`, `hole_check_json_statement` |

### Warnings: `unused-name` (M2-02 §1.3, §5)

A warning has no error kind and never changes the status. It is expressed with `command`, `stderr`
and `status` (a warning is a diagnostic line, compared byte for byte).

| Point | Case(s) |
|---|---|
| `check` prints `warning[unused-name]: NAME is never used` at the binding of an unused `let`; status stays 0 | `unused_name_let` |
| an unused parameter, a `var`, and an unread result (`let n = parse_int(text)`, M2-12 §1.8) | `unused_name_parameter`, `unused_name_several_in_order`, `unused_name_unread_result` |
| several warnings are listed in position order, and with notes in one position order | `unused_name_several_in_order`, `unused_name_with_hole_note` |
| `check --json` prints a warning as one object of kind `unused-name` (M2-02 §5, M2-06 §1.5) | `unused_name_check_json` |
| used names, unused functions, enums and grants give no warning | `unused_name_used_is_silent` |
| a program with an error prints the error alone, no warning | `unused_name_not_with_error` |
| `run`, `examples` and `build` print no warning | `unused_name_run_silent`, `unused_name_examples_silent`, `unused_name_build_silent` |
| the light program under `check` | `unused_name_light_check` |
| a `for` name and a pattern name that are never used (M2-02 §5) | `unused_name_for_and_pattern` |
| a `var` that is only assigned counts as used (M2-02 §5) | `unused_name_assigned_counts` |
| an `inout` parameter that is only written is used (M2-02 §5) | `unused_name_inout_written` |
| a use in a sibling block, or of the function a local hides, does not use the local | `unused_name_sibling_block`, `unused_name_shadowing` |
| a local of an example warns, its definition the example | `unused_name_example_local` |
| under `--json` warnings and notes are one object per line in position order (M2-06 §1.5) | `unused_name_json_stream` |

### Covered elsewhere

A case cannot make a host fail, so these are left to the host tests of the toolchain:

- a file effect failing with `no permission`, `no space` or `failed` for a file over the String limit
  (`paka/tests/effects.rs`: `a_file_the_process_may_not_use_gives_no_permission`,
  `each_way_a_write_fails_gives_its_reason`, `each_way_a_read_fails_gives_its_reason`);
- `Print` failing on a full or closed standard output (`paka/tests/cli.rs`:
  `full_disk_gives_status_3_and_its_line`, `closed_pipe_gives_status_3_and_its_line`); a closed
  descriptor and a file size limit have no test;
- the call-depth and collection-size numbers of M2-06 §1.4 (`stack` and `memory` are tested with sizes no cap could allow);
- a stack trap inside a deep copy or compare, and `deep(1 / 0)` at the limit;
- the entry text of an example that traps `stack` or `memory`;
- rollback of earlier effects when a later one fails.

## errors/: one rejected program per error kind

The reject format above pins only the code and the position. These cases also pin the offending
text, the types and the fix, so they use a second small format. Both formats stay.

### Case format

A case is a folder `errors/<name>/` holding `main.paka` and `expect.json`: one JSON object on one
line, then a line feed. Its keys:

| Key | Meaning |
|---|---|
| `kind` | the error kind id; always present |
| `line`, `column` | 1-based; the column counts bytes |
| `text` | the offending text, exactly as the source holds it |
| `expected`, `found` | the two types of a kind that carries types (only `type-mismatch`) |
| `fix` | `null`, or `{"start","end","replacement"}`; `start` and `end` are the byte offsets of the offending text. Written only for the kinds that carry a fix (`unknown-type`, `unknown-name`, `unknown-grant`) |
| `via` | `run` or `build`, only for a kind that is raised by that command and not by `paka check` |

### Pass rule

The case folder is the working directory.

- Without `via`: `paka check main.paka --json` exits with status 1, writes nothing to standard
  output, and writes one JSON object to standard error (the `check-json` stage of spec/m1.md §11). Every key of `expect.json`, except
  `via`, equals the key of the same name in that object (`kind`, `line`, `column`, `text`, `expected`,
  `found`, `fix`). A key that `expect.json` lacks is not compared. The message wording is never
  compared.
- `via: run`: `paka run main.paka` exits with status 1, and the first line of standard error starts
  with `main.paka: error[KIND]: `.
- `via: build`: the same with `paka build main.paka -o out`.
- `via` with `line`, `column` and `text`: the first line of standard error starts with
  `main.paka:LINE:COL: error[KIND]: ` instead, and the source of `main.paka` at that line and
  column starts with `text`. Only the first line is compared; a kind that prints one line per
  position (`hole-in-build`) has its other lines unpinned.

### Cases

Each case holds the same small app (`enum Msg { Start }`, `update`, `app errors`, grant `Console`)
and changes only what its kind needs, so the kind is the first error found. A kind that carries a
fix has a second case with no near name (`..._no_fix`), which pins `"fix":null`.

Pins: line and column of the diagnostic. A `*` marks a text whose length follows the span rule of
spec/m1.md §10: a named token covers itself (`\`), the first token of a construct covers the
construct (the target `c.shape.w`, the operand or write `&xs`).

| Kind | Case | Pins | Record |
|---|---|---|---|
| `invalid-byte` | `invalid_byte` | 4:1 | spec/m1.md §10 |
| `unexpected-character` | `unexpected_character` | 4:13 | spec/m1.md §10; M2-03: a bare `}` in a string |
| `unterminated-string` | `unterminated_string` | 4:11 | spec/m1.md §10; `tests/errors/unterminated_string` |
| `bad-escape` | `bad_escape` | 4:13* | spec/m1.md §10; M2-03 |
| `unexpected-token` | `unexpected_token` | 4:12 | spec/m1.md §10; M2-02 §5: `;` |
| `unexpected-inout-mark` | `unexpected_inout_mark` | 5:11 | spec/m1.md §10; M2-05 §5 |
| `nesting-too-deep` | `nesting_too_deep` | 4:66 | spec/m1.md §10; M2-02 §5; `tests/cases/parse/nesting_too_deep` |
| `unsupported-form` | `unsupported_form` | via `build`, no position | spec/m1.md §10 (build, no position) |
| `duplicate-name` | `duplicate_name` | 3:6 | spec/m1.md §10; `tests/cases/check/duplicate_enum` |
| `duplicate-local` | `duplicate_local` | 5:7 | spec/m1.md §10; M2-02 §5 |
| `no-app` | `no_app` | via `run`, no position | spec/m1.md §10 (no position) |
| `multiple-apps` | `multiple_apps` | 13:5 | spec/m1.md §10; `tests/cases/check/two_apps` |
| `unknown-type` | `unknown_type` | 3:12, fix | spec/m1.md §10; M2-02 §1.3, §5 |
| `unknown-type` | `unknown_type_no_fix` | 3:12, no fix | spec/m1.md §10; M2-02 §1.3, §5 |
| `wrong-type-arguments` | `wrong_type_arguments` | 3:12 | spec/m1.md §10; M2-02 §5 |
| `unknown-name` | `unknown_name` | 5:11, fix | spec/m1.md §10; M2-02 §1.3, §5 |
| `unknown-name` | `unknown_name_no_fix` | 4:11, no fix | spec/m1.md §10; M2-02 §1.3, §5 |
| `wrong-argument-count` | `wrong_argument_count` | 4:4 | spec/m1.md §10; `tests/cases/check/print_without_argument` |
| `type-mismatch` | `type_mismatch` | 4:3, types | spec/m1.md §10; `tests/errors/type_mismatch` |
| `duplicate-app-entry` | `duplicate_app_entry` | 9:3 | spec/m1.md §10 |
| `missing-start` | `missing_start` | 1:5 | spec/m1.md §10 |
| `missing-update` | `missing_update` | 1:5 | spec/m1.md §10 |
| `missing-grants` | `missing_grants` | 1:5 | spec/m1.md §10 |
| `bad-update-signature` | `bad_update_signature` | 9:11 | spec/m1.md §10; `tests/cases/check/update_takes_string` |
| `bad-start` | `bad_start` | 8:10 | spec/m1.md §10; `tests/cases/check/start_is_effect` |
| `bad-state` | `bad_state` | 8:10 | spec/m1.md §10; M2-15; `tests/cases/parse/app_state_entry` |
| `unknown-grant` | `unknown_grant` | 10:12, fix | spec/m1.md §10; `tests/errors/unknown_grant` |
| `unknown-grant` | `unknown_grant_no_fix` | 10:12, no fix | spec/m1.md §10; `tests/errors/unknown_grant` |
| `duplicate-grant` | `duplicate_grant` | 10:21 | spec/m1.md §10 |
| `hole-in-build` | `hole_in_build` | via `build`, 4:19 | spec/m1.md §10; M2-06 §1.5 |
| `no-finite-value` | `no_finite_value` | 3:6 | M2-05 §5; `ownership/finite_record_holds_itself` |
| `unused-result` | `unused_result` | 4:3 | M2-02 §5; `tests/cases/check-text/unused_result_call` |
| `nothing-to-discard` | `nothing_to_discard` | 5:3 | M2-02 §5 |
| `missing-value` | `missing_value` | 5:1 | M2-02 §5: at the block's `}` |
| `unreachable-code` | `unreachable_code` | 5:3 | M2-02 §5 |
| `break-outside-loop` | `break_outside_loop` | 4:3 | M2-02 §5 |
| `if-needs-else` | `if_needs_else` | 4:11 | M2-02 §5 |
| `return-last` | `return_last` | 4:3 | M2-02 §5 |
| `assign-to-let` | `assign_to_let` | 5:3 | M2-02 §5; `ownership/assign_to_let` |
| `bad-assignment-target` | `bad_assignment_target` | 8:3* | M2-02 §5; M2-05 §5; `ownership/assign_to_variant_field` |
| `cannot-infer` | `cannot_infer` | 4:11 | M2-02 §5; `tests/cases/check-text/cannot_infer_discarded_empty` |
| `wrong-label` | `wrong_label` | 4:10 | M2-02 §5; `tests/cases/check-text/fn_value_labelled_call` |
| `not-a-function` | `not_a_function` | 5:11 | M2-02 §5; `tests/cases/check-text/not_a_function_int` |
| `missing-inout-mark` | `missing_inout_mark` | 5:10 | M2-05 §5; `ownership/mark_missing` |
| `bad-inout-target` | `bad_inout_target` | 5:11, the `a` of `&a[0]` | M2-05 §5; `ownership/mark_on_let` |
| `inout-overlap` | `inout_overlap` | 11:19* | M2-05 §5; `ownership/overlap_same_place_twice` |
| `changed-while-read` | `changed_while_read` | 11:14 | M2-05 §5; `ownership/read_operator_operand` |
| `changed-while-walked` | `changed_while_walked` | 5:24* | M2-05 §5; `ownership/walk_append_to_walked` |
| `not-a-value` | `not_a_value` | 4:11 | `tests/cases/check-text/built_in_as_value` |
| `literal-out-of-range` | `literal_out_of_range` | 4:28 | spec/m1.md §10; M2-15 |
| `missing-arm` | `missing_arm` | 5:11 | M2-02 §5; `tests/cases/check-text/missing_arm_none` |
| `duplicate-arm` | `duplicate_arm` | 7:5 | M2-02 §5 |
| `wildcard-on-enum` | `wildcard_on_enum` | 5:5 | M2-02 §5; `tests/cases/check-text/wildcard_on_option` |
| `bad-view-signature` | `bad_view_signature` | 16:18 | M5-01 §2; `tests/cases/check/view_inout_parameter` |
| `element-outside-view` | `element_outside_view` | 17:3 | M5-01 §1.3; `tests/cases/check/view_element_in_fn` |
| `wrong-child-block` | `wrong_child_block` | 17:3 | M5-01 §2; `tests/cases/check/view_missing_child_block` |
| `missing-key` | `missing_key` | 19:7 | M5-02; `tests/cases/check/view_loop_child_without_key` |
| `key-on-root` | `key_on_root` | 16:63 | M5-02; `tests/cases/check/view_key_on_root` |
| `duplicate-key` | `duplicate_key` | 19:28 | M5-02; `tests/cases/check/view_duplicate_key` |
| `ui-outside-example` | `ui_outside_example` | 27:15 | M5-06 §2; `tests/cases/check/ui_type_in_fn` |
| `bad-ui-start` | `bad_ui_start` | 14:21 | M5-06 §2; `tests/cases/check/ui_start_string` |
| `bad-ui-path` | `bad_ui_path` | 28:17 | M5-06 §1.2; `tests/cases/check/ui_bad_path` |

Not in the table, since a program cannot reach them in a file:

| Kind | Why no case |
|---|---|
| `file-unreadable` | needs a file that cannot be read; a case folder always holds `main.paka` (spec/m1.md §13) |
| `clang-missing` | needs a machine without `clang`; tested out of the folder (spec/m1.md §13) |
| `clang-failed` | needs a failing `clang` run (spec/m1.md §13) |
| `runtime-missing` | needs `libpaka_rt.a` removed from the executable's folder (spec/m1.md §13) |
| `bad-migrate-signature` | a migrate is not a program file; tested in paka/tests/migrate.rs (M5-07 §2) |
| `migrate-needs-reset` | a migrate is not a program file; tested in paka/tests/migrate.rs (M5-07 §2) |
| `effect-in-migrate` | a migrate is not a program file; tested in paka/tests/migrate.rs (M5-07 §2) |
| `old-value-use` | a migrate is not a program file; tested in paka/tests/migrate.rs (M5-07 §2) |

`unused-name` is a warning and has no kind; its cases are under "Warnings: `unused-name`" in
the apps/ part. `unsupported-form` is temporary (spec/m1.md §10): drop its case when the back end
builds the core form.

### types/ M2-02: records, enums, names

| Point | Case(s) |
|---|---|
| record fields, defaults, order of fields in a literal | `record_defaults_and_order` |
| field omitted with no default | `record_field_omitted` |
| label not a field / out of order | `record_label_not_a_field`, `record_label_out_of_order` |
| duplicate field name | `record_duplicate_field` |
| unknown field read; field read on a non-record, on a variant | `record_field_read_unknown`, `field_on_non_record`, `field_read_on_variant` |
| field default is a constant: call, sum, non-empty list | `field_default_call`, `field_default_sum`, `field_default_list_of_values` |
| field default: bare variant that has fields | `field_default_bare_variant_with_fields` |
| record literal in an `if` header | `record_literal_in_header_unparenthesized`, `record_literal_in_header_parenthesized` |
| variant arguments: too few, too many, wrong type, labelled | `variant_too_few_fields`, `variant_too_many_fields`, `variant_wrong_field_type`, `variant_labelled_argument` |
| fieldless variant called | `fieldless_variant_called_empty`, `fieldless_variant_called_with_value` |
| bare variant with fields as a value | `bare_variant_with_fields_as_value` |
| duplicate variant, variant field names | `duplicate_variant_name`, `variant_duplicate_field` |
| names are one pool with types, variants, built-ins | `type_named_like_builtin`, `variant_named_like_builtin`, `variant_named_like_type`, `function_named_like_row` |
| a local hides a built-in name; calling it is `not-a-function` | `local_hides_builtin_not_a_function` |

### types/ M2-02: `match`

| Point | Case(s) |
|---|---|
| arm forms: block, single line, alternatives, `_`, nested patterns | `match_arm_forms`, `match_enum_arms`, `match_int_string_bool` |
| a single-line arm `_ = e`, its `?` passing an error on | `match_arm_discard` |
| `Option` and `Result` subjects | `match_option_result` |
| an arm that leaves: bare after `=>` is refused at the keyword; as a block it runs | `match_arm_return`, `match_arm_break`, `match_arm_continue`, `match_arms_that_leave` |
| arm types differ | `match_arm_type_differs` |
| a single-line arm with no value: `missing-value` at its pattern (M2-02 §5) | `match_single_line_arm_no_value` |
| empty `match`; comma after an arm | `match_empty`, `match_comma_after_arm` |
| name in an alternatives arm | `match_alternative_binds_name` |
| subject type: Float, List, record | `match_subject_float`, `match_subject_list`, `match_subject_record` |
| `missing-arm` at `match`: Int, String, Bool, user enum (one, several), Option, Result | `match_missing_int_wildcard`, `match_missing_string_wildcard`, `match_missing_bool_arm`, `match_missing_one_variant`, `match_missing_several_variants`, `match_missing_option_arm`, `match_missing_result_arm` |
| `wildcard-on-enum` at `_`: user enum, Option, Result, `_` alone, an arm written as `_ = e` | `wildcard_on_user_enum`, `wildcard_on_option`, `wildcard_on_result`, `wildcard_only_arm_on_enum`, `wildcard_on_enum_discard_arm` |
| `duplicate-arm` at the later pattern: Int, String, Bool, variant, in alternatives | `duplicate_arm_int`, `duplicate_arm_string`, `duplicate_arm_bool`, `duplicate_arm_variant`, `duplicate_arm_in_alternatives` |
| `duplicate-arm`: arm after `_`; `_` after both Bool arms | `duplicate_arm_after_wildcard`, `duplicate_arm_wildcard_after_both_bools` |
| pattern type: Int on Bool, String on Int, variant of another enum | `pattern_int_on_bool`, `pattern_string_on_int`, `pattern_variant_of_other_enum` |
| pattern names: unknown variant, field count too few / too many | `pattern_unknown_variant`, `pattern_too_few_fields`, `pattern_too_many_fields` |
| pattern shape: fieldful variant bare, fieldless variant with a name | `pattern_fieldful_variant_bare`, `pattern_fieldless_variant_with_name` |

### types/ M2-09 §1.4 to §1.6, M2-03: equality, order, text, arithmetic types

| Point | Case(s) |
|---|---|
| `==` on scalars and text | `eq_scalars_and_text` |
| `==` on Float: NaN, zero | `eq_float_nan_and_zero`, `eq_nan_inside_values` |
| `==` on records, enums, lists, maps, `Option`, `Result`, effect lists | `eq_records_and_enums`, `eq_lists_and_maps`, `eq_option_and_result`, `eq_effect_lists` |
| `==` across types, Int against Float | `eq_different_types`, `eq_int_float` |
| `<` on Bool, List, `Option`, record | `order_on_bool`, `order_on_list`, `order_on_option`, `order_on_record` |
| comparison chain | `compare_chain` |
| `+` on lists, String with Int, Int with Float; `max` on mixed numbers | `plus_on_lists`, `plus_string_int`, `sum_int_float`, `max_mixed` |
| `len` on an Int | `len_of_int` |
| interpolation takes Int, String, Bool only | `interp_float`, `interp_list`, `interp_option`, `interp_record`, `interp_enum`, `interp_fn_value` |
| `sort` on Int, Float, String; on records | `sort_int_float_string`, `sort_on_records` |

### types/ M2-09 §1.3: generics, the four-step walk

| Point | Case(s) |
|---|---|
| a bare `[]`, `[:]`, `None`, `Ok(..)`, `Err(..)` has no type | `generics_bare_empty_list`, `generics_bare_empty_map`, `generics_bare_none`, `generics_bare_ok`, `generics_bare_err` |
| `len([])`, `[] == []`, `None == None`, `match None`, `[[]]` | `generics_len_of_empty_list`, `generics_empty_equals_empty`, `generics_none_equals_none`, `generics_match_subject_open`, `generics_list_of_empty_list` |
| branches that both stay open | `generics_ok_err_branches_open` |
| a typed side or branch closes the other | `generics_branch_typing`, `generics_result_branches_typed`, `generics_list_literal_typing` |
| one letter bound twice to different types | `generics_append_wrong_item`, `generics_unwrap_or_wrong_fallback` |
| accepted walk: expected type, argument, assigned to a typed place | `generics_walk_accepted` |
| a joined list type that is still open (`[None, None]`): accepted where a type is given, `cannot-infer` where none is, at the list's `[` (M2-02 §5) | `generics_open_list_annotated`, `generics_open_list_unannotated`, `generics_open_list_argument` |
| an open map literal is `cannot-infer` at its `[` (M2-02 §5) | `generics_open_map_unannotated` |

### types/ M2-09 §1.7 to §1.11: function values

| Point | Case(s) |
|---|---|
| a `fn` name is a value of type `fn(P) -> R`; stored, passed, returned, called | `fv_store_pass_call`, `fv_stored_in_values`, `fv_returned_from_function` |
| a variant is a constructor value | `fv_variant_as_constructor` |
| labels are not in the type; the value is positional | `fv_labels_not_in_type`, `fv_call_with_label` |
| `fn` type with no parameter and no result | `fv_zero_parameter_and_no_result` |
| `==` on function values | `fv_equality`, `fv_equality_of_different_types` |
| call: not a function, too few / too many arguments, wrong type, `&` mark | `fv_call_of_int`, `fv_call_too_few_arguments`, `fv_call_too_many_arguments`, `fv_call_wrong_argument_type`, `fv_call_with_inout_mark` |
| built-in as a value is `not-a-value` | `fv_builtin_fixed_not_a_value`, `fv_builtin_len_not_a_value`, `fv_sort_by_builtin_as_before` |
| built-in variant with fields as a value | `fv_variant_some_not_a_value`, `fv_variant_ok_not_a_value`, `fv_variant_print_not_a_value`, `fv_variant_exit_with_not_a_value` |
| fieldless variant where a function is expected | `fv_fieldless_variant_for_fn_type` |
| `inout` function: not a value, not an argument, not in a `fn` type | `fv_inout_fn_not_a_value`, `fv_inout_fn_as_argument`, `fv_inout_in_function_type` |
| a type name as a value | `fv_type_name_as_value` |
| a function passed to itself | `fv_pass_function_to_itself` |
| a bare `fn` name where its own result is expected | `fv_bare_fn_where_result_expected` |
| effect-list functions: `then`, `or`, bare `Print` | `fv_then_given_message`, `fv_then_given_builtin_variant`, `fv_then_too_many_fields`, `fv_or_given_effect`, `fv_effect_list_bare_print` |
| `sort_by` before: built-in, result not Bool, wrong item type | `fv_sort_by_builtin_as_before`, `fv_sort_by_result_not_bool`, `fv_sort_by_wrong_item_type` |
| app state reaches a function type, with or without a default | `fv_state_reaches_function_type`, `state_function_field_no_default` |
| a record holding a function outside the state | `fv_state_unrelated_record_holds_function` |

### types/ habit spellings (M2-09 §2, M2-12 §1.9)

| Spelling | Case(s) |
|---|---|
| `fn` in an expression, inside a body | `habit_fn_in_expression`, `habit_fn_inside_body` |
| lambda: `\|a, b\| ..`, `\|\| ..`, `a -> ..`, `(a, b) => ..` | `habit_lambda_pipes`, `habit_lambda_empty_pipes`, `habit_lambda_arrow_single`, `habit_lambda_arrow_tuple` |
| generic `fn`, `type`, `enum` with `<T>` | `habit_generic_fn`, `habit_generic_type`, `habit_generic_enum` |
| `impl`, `trait`, `interface`, `protocol`, `extension` | `habit_impl`, `habit_trait`, `habit_interface`, `habit_protocol`, `habit_extension` |
| `if let` | `habit_if_let` |
| `null`, `nil` | `habit_null`, `habit_nil` |
| `unwrap`, `expect`, `is_some`, `is_ok`, `panic` | `habit_unwrap`, `habit_expect`, `habit_is_some`, `habit_is_ok`, `habit_panic` |
| `??` | `habit_double_question` |

### types/ M2-14 §1.13: `sort_by`

| Point | Case(s) |
|---|---|
| `before(a, b)` true puts `a` first; descending | `sort_by_descending` |
| stable on ties | `sort_by_stable_on_ties` |
| calls `before(right, left)`; always true reverses pairs | `sort_by_always_true_reverses` |
| always false keeps the order | `sort_by_always_false_keeps_order` |
| a `before` that contradicts itself | `sort_by_contradicting_before` |
| fewer than two items: no call | `sort_by_short_lists_call_nothing` |
| labelled `before` function | `sort_by_labelled_before` |
| a trap in `before`; the frame chain | `sort_by_trap_in_before`, `sort_by_trap_chain` |

### types/ M2-12: errors

| Point | Case(s) |
|---|---|
| `?` on `Option` and on `Result` | `err_question_on_option`, `err_question_on_result` |
| `?` on a plain value; on a field of a plain record | `err_question_on_plain_value`, `err_question_field_of_plain_record` |
| `?` in a function with no result, Int result, `update`, an example | `err_question_in_function_without_result`, `err_question_in_int_function`, `err_question_in_update`, `err_question_in_example` |
| `?` on `Option` in a `Result` function and the reverse | `err_question_option_in_result_function`, `err_question_result_in_option_function` |
| `?` with a different error type | `err_question_error_type_differs` |
| `?` as a whole statement | `err_question_as_whole_statement` |
| `inout` write before a failing `?` stays | `err_inout_write_before_question_stays` |
| `unused-result` on `parse_int` | `err_unused_parse_int` |
| `parse_int` used as an Int | `err_parse_int_as_int` |
| `unreachable`: reached, with argument, code after it, and `assert` | `err_unreachable_reached`, `err_unreachable_with_argument`, `err_code_after_unreachable`, `err_unreachable_and_assert_not_reached` |
| `assert` false; with a message | `err_assert_false`, `err_assert_with_message` |
| names ask, brackets claim | `err_names_ask_brackets_claim` |
| traps: divide by zero, index past end, missing key | `err_divide_by_zero`, `err_list_index_past_end`, `err_map_missing_key` |
| trap report with a three-frame chain | `err_trap_chain_three_frames` |

## Settled readings

Where the records leave a point open, the owner settled it. The cases follow these readings; the
record named in each row holds the line.

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
| `to_int` and `to_float` appear as the Int/Float bridge; the rest of their rows belong to the built-in function cases | M2-03 §1.5 |
| A shift count outside 0 to 63 traps at the function name, like every other row | M2-06 §1.1, M2-14 §1.15 |
| A slice trap sits at the `[`; its detail is `slice A..<B, length N` for an end past the length, a start past the end and a negative start | M2-06 §1.1, §1.2 |
| `swap` names the first bad position in the detail, checked `i` before `j`, also when `i == j` | M2-14 §1.8 |
| `m[a][b] = v` with `a` missing traps at the `[` of the `a` step; `&m[k]` with `k` missing traps at the `[` | M2-14 §1.11, M2-06 §1.1, core.md §6 |
| `counts[w] += 1` on a new key has the ordinary three-line report; on an Int value the detail is `key "w"; write m[k] = unwrap_or(get(m, k), 0) + 1` | M2-06 §1.2, M2-14 §1.11, §10 |
| A trap inside the function given to `sort_by` names that function first: `in before, called from update` | M2-06 §1.2, M2-14 §1.15 |
| `stack` and `memory` traps pin their one line in a `trap` file, with any position. Cases use only sizes no cap could allow (2^62 or more, or a product that overflows Int) | M2-06 §1.2, §1.4, M2-14 §1.15 |
| `unreachable()` as a whole statement is an `if` body whose last statement it is | M2-14 §1.10 |
| NaN and the infinities are made by `0.0 / 0.0` and `1.0 / 0.0`, and shown by `fixed` as `NaN`, `inf`, `-inf` | M2-03, M2-14 §1.16 |
| A `Print` string may hold line feeds; its lines are separate lines of `stdout` | M2-15 |
| `duplicate-name` for a row's name sits at the name after `fn`; `sort` on an item type with no order sits at the `&` of the argument; `wrong-label` and `type-mismatch` sit at the argument; `wrong-argument-count` at the function name; `cannot-infer` at the empty literal, `None` or the name | M2-02, M2-04, M2-09, M2-14 |
| An unlabelled argument to a labelled row (`insert(xs, 1, 0)`) is `wrong-label` at that argument; `unreachable(1)` is `wrong-argument-count` at the name; `type Option` is `duplicate-name` at the name after `type`; `len([:])` is `cannot-infer` at the `[:]` | M2-02, M2-04, M2-09, M2-14 §1.3 |
| A function type reachable from `state:` is `type-mismatch` at the value before the default check, with or without a default (R1; `types/state_function_field_no_default`) | M2-15 §1.1, M2-09 §1.11 |
| A binary type error on `==`, `+` is at the operator, as `>` is for order (`eq_int_float`, `sum_int_float`, `plus_string_int` and others) | M2-03 §1.13 |
| `max(1, 2.0)` is at the second argument (`max_mixed`) | M2-09 §1.3 |
| A pattern with the wrong number of fields is `wrong-argument-count` at the variant name in the pattern (`pattern_too_few_fields` and others) | M2-02 §5 |
| An Upper name followed by `{` ends a header, so `if Counter { count: 1 } == c` fails at the `:` (`record_literal_in_header_unparenthesized`) | M2-02 §1.9 |
| A field default is reported at the first token not part of a constant: the name of a call, the operator of a sum, the first item of a non-empty list (`field_default_*`) | M2-02 §1.9 |
| An empty `match` is `unexpected-token` at the `}` (`match_empty`) | M2-02 §1.9 |
| A comparison chain is `unexpected-token` at the second comparison operator (`compare_chain`) | spec/paka.md, M2-02 §2 |
| A single-line arm with no value is `missing-value` at its pattern (`types/match_single_line_arm_no_value`) | M2-02 §5 |
| The fixed counter sample also fixes `first_of_none`: its List becomes `[0]`, so all four examples pass (R2) | M2-06 §6 |
| The 256-byte cut of a trap detail has no marker, the cut is on the detail text, and the two-space indent is added after it (R3) | M2-06 §1.2 |
| Several `note[hole]` blocks follow each other with no blank line (R4) | M2-06 §7 |
| An example that traps directly prints `  in example NAME`, not `called from` (R6) | M2-06 §1.2, §6 |
| `no-app` has no position and no message after the code; the reject line is `main.paka: error[no-app]` (R7) | spec/m1.md §11, M2-15 §1.1 |
| `bad-start` for a function name, `unknown-name` for a name that is not defined; a type name is not used (R9) | M2-15 §5 |

## Not decided

Pools are not built (M2-01 §3), and M2-10 §1.4 leaves a shared immutable type, pools, handles,
the stale-handle rule and pool iteration order not decided. There are no cases for them. The trap
kind `stale_handle` is reserved and nothing raises it.

The `core/` cases leave out what the records do not pin: the wording of every message. The habit
spellings `;` and a trailing comma are held by `lex/semicolon` and `parse/trailing_comma`. The call-depth cases are in `apps/` (`trap_stack_*`) and are compared on kind and status only
(M2-06 §1.4). Shift traps and the other built-in rows belong to the built-in function cases.

Several M2-14 points have no case:

- `m[key] = v` on a new key past the cap: it needs 16,777,216 entries, and M2-14 §1.15 only names
  the position, the last `[`. The cap's exact value is a unit test (M2-14 §1.15).
- The value text form of §1.16 beyond the details above: no program can reach it.
- `examples` cases are in `apps/`; the cases that need no other file use `command`.
- String `[i]` and String slice traps: M2-03's, not M2-14 rows. The List index and slice cases above stand in for the shared rule.

NaN in `sort` and `sort_by` is a built-in row (M2-39b), and the `stack` trap inside `sort_by`
belongs to M2-41.
