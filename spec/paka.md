# Paka language reference

All of Paka is on this page. A block fenced as `paka` is a whole program that `paka check` accepts.
Any other block is a fragment, refused code or tool output.

## 1. A program

```paka
type Tally { words: Int = 0 }

enum Msg { Start(path: String), Loaded(text: String), Failed(why: String) }

fn count_words(text: String) -> Int {
  len(words(text))
}

fn update(s: inout Tally, m: Msg) -> List<Effect<Msg>> {
  match m {
    Start(path) => [ReadFile(path, then: Loaded, or: Failed)]
    Loaded(text) => {
      s.words = count_words(text)
      [Print("{s.words} words"), Exit]
    }
    Failed(why) => [Print("cannot read file: {why}"), ExitWith(20)]
  }
}

app wc {
  state: Tally
  start: Start
  update: update
  grants: [ReadFiles, Console]
}
```

A program is one `.paka` file, or every `.paka` file directly in one directory. It holds
definitions in any order: `type`, `enum`, `fn`, `example` and at most one `app`. Definition names,
variant names and built-in names share one pool; each name exists once (`duplicate-name`).

Paka has no user generics, interfaces, methods, lambdas, closures, tuples, null, exceptions,
globals, references or shared data. Functions cannot see anything but their parameters.

## 2. Source text

- UTF-8. Non-ASCII characters only inside strings and comments. No tab, no carriage return.
- Names are ASCII letters, digits and `_`, and start with a letter. Upper first letter: types,
  variants, effects, grants. Lower: functions, apps, examples, locals, parameters, fields, labels.
- Keywords, never names: `app break continue else enum example false fn for if in inout let match
  migrate return true type var view while`.
- `//` starts a comment that runs to the line end. No block comments.
- A line break ends a statement. A line continues only inside an open `(` or `[`, after a line
  ending in `,` `=` `->` `=>` `|` `{` or a two-sided operator, or before a line starting with
  `}` `)` `]`. So: no `;` after a statement, `else` stands on the line of the `}` before it, a
  type is written on one line, and `{ x }` may stand on one line.
- No trailing comma anywhere. Brackets nest at most 64 deep.

## 3. Types

| Type | Written | Notes |
|---|---|---|
| `Int` | `0` `42` `-7` | 64-bit signed |
| `Float` | `1.5` `0.0` | IEEE 64-bit; digits on both sides of the point |
| `Bool` | `true` `false` | |
| `String` | `"text"` | a row of Unicode code points; one line |
| `List<T>` | `[a, b]` `[]` | |
| `Map<K, V>` | `["a": 1]` `[:]` | K is `Int` or `String`; entries always in rising key order |
| `Option<T>` | `Some(value)` `None` | |
| `Result<T, E>` | `Ok(value)` `Err(error)` | E is any type |
| `Effect<Msg>` | section 10 | |
| `fn(A, B) -> R` | section 8 | a function type |

No Set (write `Map<K, Bool>`), no character type (a one-character `String`).
Number literals have no leading zero, hex, exponent or `_`.

```
type Counter { count: Int = 0, name: String }        // record; a default is a constant
enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }
Counter { name: "a" }      // fields by name in declared order; one with a default may be left out
c.count                    // field read
Circle(1.5)                // variant fields by position; a variant without fields is written Dot
```

- Variants are written bare, never `Shape.Dot`, and a variant name is unique in the whole program.
  An enum has at least one variant; variant fields have no defaults.
- In the header of `if`, `while`, `for` and `match`, write a record in brackets: `(Counter {})`.
- A type may hold itself only where a finite value exists: through `Option<T>`, a `List`, or an
  enum with another variant (`enum Expr { Num(value: Int), Add(left: Expr, right: Expr) }`).
  Otherwise `no-finite-value`.
- Types are written on every parameter, result and field. A local takes the type of its value:
  `let n = 1`. Values typed by their position (`[]`, `[:]`, `None`, `Ok(x)`, `Err(e)`, a hole,
  `unreachable()`) need a parameter, field, result, operand or annotation to give the type, else
  `cannot-infer`: write `let xs: List<Int> = []`. An `if` or `match` with no expected type takes
  the type of its first branch that has one.

## 4. Functions, statements, expressions

```paka
type Stack { items: List<Int> = [] }

enum Shape { Dot, Circle(r: Float), Rect(w: Float, h: Float) }

fn clamp(x: Int; lo: Int, hi: Int) -> Int {
  max(lo, min(x, hi))
}

fn area(s: Shape) -> Float {
  match s {
    Dot => 0.0
    Circle(r) => r * r * 3.14159
    Rect(w, h) => w * h
  }
}

fn push(s: inout Stack, x: Int) {
  append(&s.items, x)
}

fn sum_odd(n: Int) -> Int {
  var total = 0
  for i in 0..<n {
    if i % 2 == 0 {
      continue
    }
    total += i
  }
  total
}

fn sign(n: Int) -> String {
  if n < 0 { "minus" } else if n == 0 { "zero" } else { "plus" }
}

example tour {
  var st = Stack {}
  push(&st, clamp(12, lo: 0, hi: 9))
  let r = Rect(2.0, 3.0)
  st.items == [9] && area(r) == 6.0 && sum_odd(5) == 4 && sign(-1) == "minus"
}
```

- Parameters before `;` are passed by position; those after it as `name: value`, in declared
  order. A call writes exactly those labels. `f()` calls a function without parameters.
- A function without `-> T` gives no value.
- The value of a block is its last line. `return value` only leaves early; on the last line it is
  the error `return-last`: drop `return`.
- `let x = e` is fixed; `var x = e` can be assigned. Either may carry a type: `let x: T = e`.
- No shadowing: a `let`, `var`, parameter, `for` name or pattern name may not reuse a local name
  visible there (`duplicate-local`). Blocks side by side may reuse a name. A local may hide a
  function name.
- Assignment: `place = e`, also `+= -= *= /= %=`. A place is a `var` local or an inout parameter,
  then any `.field` and `[index]` steps: `g[r][c] = v`, `s.items[i].count += 1`.
- `if c { } else if d { } else { }`. Used as a value it needs `else`.
- `while c { }`; `for x in xs { }` walks a List or an Int range `a..<b` (b not included) and
  nothing else: write `for c in chars(text)`, `for k in keys(m)`. `break`, `continue`.
- `match`: one arm per line, no commas. A pattern is flat: `V`, `V(a, _)` (fields by position,
  each a new name or `_`), an Int, a String, `true`, `false` or `_`. `A | B => ...` joins
  patterns and binds no names. A match on an enum names every variant; a whole-pattern `_` is
  allowed only when the subject is an Int, String or Bool. An arm is an expression, an assignment
  or a block.
- A value must be used. A statement whose value is dropped is `unused-result`; write
  `_ = pop(&q)`. A call that gives no value stands alone, without `_ =`.
- An unused local is only a warning.
- A statement after `return`, `break`, `continue` or `unreachable()` in the same block is the
  error `unreachable-code`.

Operators, tightest first: call, index, `.field`, `?`; unary `-` `!`; `*` `/` `%`; `+` `-`;
`..<`; `==` `!=` `<` `<=` `>` `>=`; `&&`; `||`. Comparisons do not chain: write
`a < b && b < c`. Evaluation is strictly left to right; `&&` and `||` skip the right side when the
left decides.

Both sides of an operator have the same type; nothing converts by itself (`to_float(n)`,
`to_int(x)`).

| Operator | Types |
|---|---|
| `==` `!=` | every type, compared by structure |
| `<` `<=` `>` `>=` | Int, Float, String (by code point) |
| `+` `+=` | Int, Float, String (joins) |
| `-` `*` `/` | Int, Float |
| `%` | Int |
| `!` `&&` `\|\|` | Bool |

## 5. Numbers and text

- An Int result outside -9223372036854775808 to 9223372036854775807 traps `overflow`. `/`
  truncates toward zero, `%` takes the sign of the left side, a zero divisor traps
  `divide_by_zero`. Wrapping and bit operations are functions (section 9).
- Float arithmetic never traps. NaN equals nothing, itself included.
- String escapes, exactly: `\n` `\r` `\t` `\\` `\"` `\{` `\}`.
- `"{expr}"` puts an Int, Bool or String into the text. The expression holds no `"`, `{` or `}`;
  bind anything larger to a `let` first. A Float is written `"{fixed(x, digits: 2)}"`. No other
  type becomes text by itself: build the text from its parts.
- `s[i]` is a one-character String, `s[a..<b]` and `xs[a..<b]` are slices, each a new value.
  Positions start at 0 and count code points; `len` counts the same. A position outside traps
  `out_of_bounds`.
- A String changes only as a whole: `s = s + "x"`, `s += "x"`. `s[i] = "x"` is refused.
- A digit test: `c >= "0" && c <= "9"`.

## 6. Values and `inout`

- Every local, field and item is its own value. `let b = a` copies in meaning: changing one never
  changes the other. Two places never share data.
- Arguments are read-only. To let a function change the caller's data, declare the parameter
  `inout` and mark the argument with `&`: `fn push(s: inout Stack, x: Int)`, `push(&st, 1)`.
- `&` is written on every inout argument and nowhere else. After it comes a `var` local or an
  inout parameter with any field and index steps (`&g.adj[a]`, `&m[key]`). Not a `let` (declare
  it `var`), a read-only parameter (declare it `inout`), a `for` name or pattern name (they are
  copies), a slice (pass the list and the positions) or a String position.
- Two places overlap unless they start at different variables or first differ at two different
  field names. Two index steps into one List or Map always overlap: `xs[i]` and `xs[j]`.
- `inout-overlap`: a place passed with `&` overlaps another argument of the same call.
  Fix: bind it first, `let old = xs` then `grow(&xs, old)`. For two items of one list, bind one
  first or pass the list and both positions. Arguments of type Int, Float or Bool are exempt; a String is not.
- `changed-while-read`: one operand reads a place that a sibling operand changes. This holds for
  operators, list, map and record literals and interpolation too. Fix: bind it first.
- `changed-while-walked`: the body of `for x in xs` changes `xs` or anything overlapping it.
  Fix: walk a copy (`let items = xs`) or walk by position (`for i in 0..<len(xs)`). `while`
  freezes nothing.
- The fields of a variant are not places. To change one, match, build the new value and assign
  the subject: `Rect(w, h) => s = Rect(w + 1, h)`.
- Trees are enums or records that hold themselves; using a subtree twice makes two copies.
  Graphs use list positions: a node is its index.

```paka
type Graph { adj: List<List<Int>> = [] }

fn dfs(g: Graph, seen: inout List<Bool>, u: Int) {
  seen[u] = true
  for v in g.adj[u] {
    if !seen[v] { dfs(g, &seen, v) }
  }
}

example reach {
  let g = Graph { adj: [[1], [0], []] }
  var seen = list_of(false, count: 3)
  dfs(g, &seen, 0)
  seen == [true, true, false]
}
```

## 7. Failure: `Option`, `Result`, `?`

- A function that can fail or have no answer says so in its result: `Option<T>` or
  `Result<T, E>`. Take them apart with `match`, `unwrap_or` or `?`.
- Brackets claim, names ask. `xs[i]`, `m[key]` and `/` claim the position, key or divisor is
  good, and trap when it is not. `get(m, key)`, `find`, `parse_int` and `parse_float` ask, and
  give an `Option`.
- `e?` gives the inside of `Some` or `Ok`; on `None` or `Err` the function returns that at once.
  It may stand only in a function whose result is `Option<_>` (for an Option) or `Result<_, E>`
  with the same `E` (for a Result). Not in `update`, not in an example.
- An Option inside a Result function: `ok_or(opt, error)?`.
- Changes made through an inout parameter before a `?` returns stay made.
- An arm that returns is a block: `None => {`, `return Err("empty")`, `}` on three lines.
- `unreachable()` stands where a value is expected but cannot be reached; it traps `assert`.
- There is no `unwrap`, `expect`, `is_some`, `panic` or null.
- A program reports a failure in `update`: `[Print(why), ExitWith(20)]`.

```paka
fn total(fields: List<String>) -> Result<Int, String> {
  var sum = 0
  for f in fields {
    sum += ok_or(parse_int(f), "not a number: {f}")?
  }
  Ok(sum)
}

fn first_word(text: String) -> Option<String> {
  let ws = words(text)
  if len(ws) == 0 {
    return None
  }
  Some(ws[0])
}

fn count_words(text: String) -> Map<String, Int> {
  var counts: Map<String, Int> = [:]
  for w in words(text) {
    counts[w] = unwrap_or(get(counts, w), 0) + 1
  }
  counts
}

example failures {
  total(["1", "2"]) == Ok(3) && total(["x"]) == Err("not a number: x") &&
    first_word("") == None && count_words("a b a") == ["a": 2, "b": 1]
}
```

## 8. Function values

- A function value is the bare name of a user `fn` without inout parameters, or of a user variant
  with fields (`Loaded` is a `fn(String) -> Msg`). Its type is written `fn(A, B) -> R`.
- A call through a function value is positional and takes no `&`.
- Built-in functions and built-in variants are not values (`not-a-value`): wrap one in a user
  `fn`.
- There are no lambdas. Write a top-level function and pass its name:

```
fn by_score(a: Player, b: Player) -> Bool { a.score > b.score }
sort_by(&players, by_score)
```

- Two function values are equal when they name the same definition.
- An app's state may hold no function type.

## 9. Built-in functions

These 63 names cannot be user function names. `T`, `K`, `V`, `E` stand for any type. All
arguments are positional except the three labels shown (`digits:`, `at:`, `count:`). A row
without `->` gives no value. A row that changes its first argument takes it with `&`.

```
len(x) -> Int                         String: code points; List: items; Map: entries
is_empty(x) -> Bool                   len(x) == 0
contains(text, part) -> Bool          also contains(xs, item)
find(text, part) -> Option<Int>       first position; also find(xs, item)

min(a, b) -> T   max(a, b) -> T       Int, Float or String
abs(x) -> T                           Int or Float
to_float(n: Int) -> Float
to_int(x: Float) -> Int               drops the fraction; traps bad_conversion on NaN or out of range
sqrt(x) floor(x) ceil(x) trunc(x) round(x)    Float -> Float; round: ties away from zero
fixed(x: Float; digits: Int) -> String        digits 0 to 20; ties away from zero
wrap_add(a, b) wrap_sub(a, b) wrap_mul(a, b)  Int, wraps instead of trapping
bit_and(a, b) bit_or(a, b) bit_xor(a, b) bit_not(a)
shift_left(a, count) shift_right(a, count)    count 0 to 63

parse_int(text) -> Option<Int>        "-12", "007"; nothing trimmed; None when not a number
parse_float(text) -> Option<Float>
code_at(text, i) -> Int               the code point at position i
from_code(n) -> String                one character

split(text, sep) -> List<String>      split("a,,b", ",") is ["a", "", "b"]
chars(text) -> List<String>           one String per character
lines(text) -> List<String>           split on "\n"; no empty last piece
words(text) -> List<String>           runs between white space, never empty
join(parts, sep) -> String
repeat(text, n) -> String
pad_left(text, width) -> String       spaces in front up to width
pad_right(text, width) -> String
trim(text) trim_start(text) trim_end(text) -> String     white space off the ends
starts_with(text, part) ends_with(text, part) -> Bool
replace(text, old, new) -> String     every match
upper(text) lower(text) -> String     ASCII letters only

append(&xs, item)
extend(&xs, more)                     appends every item of the List more
pop(&xs) -> T                         removes the last item; traps on an empty List
insert(&xs, item; at: Int)            at == len(xs) appends
remove_at(&xs, i) -> T
swap(&xs, i, j)
reverse(&xs)
sort(&xs)                             rising; List of Int, Float or String
sort_by(&xs, before)                  before: fn(T, T) -> Bool
list_of(item; count: Int) -> List<T>  count copies

get(m, key) -> Option<V>
has(m, key) -> Bool
remove(&m, key)                       a missing key changes nothing
keys(m) -> List<K>                    in rising order
values(m) -> List<V>                  in key order

unwrap_or(opt, fallback) -> T         the inside of Some, else fallback
ok_or(opt, error) -> Result<T, E>     Some(x) gives Ok(x), None gives Err(error)
assert(cond)                          traps assert when false
unreachable()                         traps assert when reached
```

- White space is exactly six characters: code points 9 to 13 and 32.
- Text is matched exactly, left to right, without overlap. No patterns.
- The sort is stable: `before(a, b)` says whether `a` must come before `b`, and items for which
  it never says so keep their order.
- Map: `m[key]` reads and traps `out_of_bounds` when the key is missing. `m[key] = v` adds or
  replaces. Every other use needs the key present: `m[key] += 1` on a new key traps. The count
  idiom is `counts[w] = unwrap_or(get(counts, w), 0) + 1`.
- A bad position, a count below zero, `pop` on an empty List, a shift count or `digits` out of
  range trap `out_of_bounds`.
- There is no `map`, `filter`, `fold`, `sum`, `sorted` or `clamp`: write a loop.

## 10. Apps and effects

```
app NAME {
  state: S          // optional: a record type with a default on every field
  start: Start      // required: a variant of the message enum; its fields are all String
  update: update    // required: the handler
  grants: [Console] // required, may be []: Console, ReadFiles, WriteFiles
}
```

One key per line. The handler has exactly one of two shapes:

```
fn update(s: inout S, m: Msg) -> List<Effect<Msg>>     // with state:
fn update(m: Msg) -> List<Effect<Msg>>                 // without state:
```

| Effect | Needs grant | Answer |
|---|---|---|
| `Print(text)` | `Console` | none; writes the text and a line feed to standard output |
| `ReadFile(path; then: fn(String) -> Msg, or: fn(String) -> Msg)` | `ReadFiles` | `then(text)` or `or(reason)` |
| `WriteFile(path, text; then: Msg, or: fn(String) -> Msg)` | `WriteFiles` | `then` or `or(reason)` |
| `Exit` | none | ends with status 0 |
| `ExitWith(status)` | none | ends with that status: 0 or 5 to 255 |

Effects are plain values; only the list the app's `update` returns is performed. `then:` and `or:`
are both required. A reason is one of `not found`, `no permission`, `not a file`, `not text`,
`no space`, `failed`. `WriteFile` replaces the whole file; `ReadFile` gives the whole file.

How a program runs:

- The command-line arguments fill the fields of the `start:` variant by position. A wrong count
  ends the run with status 2 and a usage line such as `wc: expected 1 argument (path), got 0`.
- The state starts from its defaults. The start message is the first in one queue.
- The runtime takes the front message and calls `update`. When the handler has returned, its
  effects are performed in list order. Each file answer joins the back of the queue.
- Answers arrive in the order their effects were performed. Answers carry no path: keep in the
  state what you wait for.
- `Exit` and `ExitWith` end the program at once. Later effects are not performed, and an answer
  queued at Exit is dropped.
- A trap in a handler performs none of that handler's effects.
- There is no standard input: data comes in through arguments and files.

Four stops end a program with one line on standard error and status 3:

```
<app>: effect <Effect> refused: grant <Grant> is missing
<app>: effect Print failed: <reason>
<app>: effect ExitWith failed: status <n> is not 0 or 5 to 255
<app>: ended without Exit
```

A grant is tested when its effect is about to be performed. Every path must reach `Exit` or
`ExitWith`: an empty queue without one is the fourth stop. A literal status outside 0 or 5 to 255
is the check error `literal-out-of-range`. A program without an app checks, but `paka run` and
`paka build` refuse it (`no-app`).

## 11. Traps, holes, examples

A trap ends the program with status 3. It cannot be caught. Evaluation order decides which trap
comes first. The eight kinds:

| Kind | Raised by |
|---|---|
| `overflow` | Int arithmetic out of range |
| `divide_by_zero` | Int `/` or `%` by 0 |
| `out_of_bounds` | a bad index, slice, key, count or position |
| `bad_conversion` | `to_int`, `from_code` |
| `assert` | `assert(false)`, `unreachable()` |
| `hole` | a hole reached |
| `stack` | more than 10,000 nested calls |
| `memory` | a List, Map or String beyond 16,777,216 items, entries or bytes |

```
<app>: trap <kind> at <file>:<line>:<col>
  <detail>
  in <function>, called from <function>
```

The kind and the position are meaning; the two indented lines are help and may be absent.

A hole `??name` stands for code not yet written; it takes the type its position expects.
`paka check` accepts it (a note, status 0), reaching it traps `hole`, and `paka build` refuses it.

An example is `example name { ... }`: statements, then one Bool expression. It takes no
arguments, performs no effects and may not use `?`. Examples may call `update` on a `var` state
and compare effect lists with `==`. They are checked with the program and run only by
`paka examples`. Each gets one status: `pass` (true), `fail` (false), `trapped` (a trap) or
`unfinished` (a hole reached).

## 12. Tools

| Command | Does |
|---|---|
| `paka check <file> [--json]` | checks; prints the error, or warnings and hole notes |
| `paka run <file> [arguments]` | checks, then runs the app |
| `paka examples <file>` | checks, then runs every example |
| `paka build <file> -o <output>` | checks, then writes an executable |

Exit status: 0 good; 1 an error in the program; 2 bad usage; 3 a trap, a stop or an example that
did not pass; 4 an internal error.

A command reports the first error only, as one line; `paka check` adds one line per field:

```
main.paka:4:3: error[type-mismatch]: expected `List<Effect<Msg>>`, found `String`
  definition: update
  bytes: 63..83
  text: "\"let there be light\""
  expected: List<Effect<Msg>>
  found: String
```

A `fix:` line follows when the error knows its repair. With `--json` the same error is one object
with the keys `kind`, `definition`, `message`, `file`, `start`, `end`, `line`, `column`, `text`,
`expected`, `found`, `fix`. Lines and columns start at 1; columns count bytes.
