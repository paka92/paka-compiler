# Paka screens

Visual apps: views, UI examples and changing a running app. This page continues `spec/paka.md`
(sections 1 to 12) and is read with it. A block fenced as `paka` is a whole program that
`paka check` accepts; any other block is a fragment or tool output.

## 13. Views

```paka
type Todo { id: Int, title: String, done: Bool = false }

type Todos { draft: String = "", items: List<Todo> = [] }

enum Msg { Draft(text: String), Add, Toggle(id: Int) }

fn update(s: inout Todos, m: Msg) {
  match m {
    Draft(text) => s.draft = text
    Add => {
      append(&s.items, Todo { id: len(s.items) + 1, title: s.draft })
      s.draft = ""
    }
    Toggle(id) => {
      for i in 0..<len(s.items) {
        if s.items[i].id == id { s.items[i].done = !s.items[i].done }
      }
    }
  }
}

view todo_row(t: Todo) -> View<Msg> {
  Row(gap: 8) {
    Button(if t.done { "[x]" } else { "[ ]" }, on: Toggle(t.id))
    Text(t.title)
  }
}

view todo_view(s: Todos) -> View<Msg> {
  Col(gap: 8) {
    Input(s.draft, on_change: Draft, key: "new")
    Button("Add", on: Add)
    if len(s.items) == 0 { Text("Nothing to do") }
    for t in s.items {
      todo_row(t, key: t.id)
    }
    ??footer
  }
}

app todo {
  state: Todos
  update: update
  view: todo_view
  grants: []
}

example add_one {
  var ui = ui_start(todo)
  ui_click(&ui, "'new'")
  ui_type(&ui, "milk")
  ui_click(&ui, "#1")
  ui_text(ui, "'new'") == "" && ui_text(ui, "1/#1") == "milk" && ui_has(ui, "1")
}
```

A visual app adds `view:`, needs `state:`, and may leave out `start:`. Its `update` may give no
result. The state starts from its defaults, the `start:` message is handled if there is one, then
the view runs. The view runs again after every handler that did not trap; a trap in a handler
keeps the state, the tree and the queue. `paka run` refuses a visual app for now (`no-app`);
`paka build` builds it to open in a window; `paka examples` runs its UI examples with no window.

- `view name(params) -> View<Msg>` gives one element. Parameters are read-only; no `inout`, no
  `&`. The app's view takes exactly the state and gives `View<M>` for `update`'s message enum M.
  Else `bad-view-signature`.
- A view body is `let` lines, then one element: an element, a view call, a hole, or an `if`/`else`
  or `match` each of whose branches ends in one.
- A child block `{ }` follows the arguments of `Row` and `Col`, even when empty (`Col() {}`), and
  of nothing else (`wrong-child-block`). Each line adds one child in order: an element, a view
  call, a hole, `let`, `if`, `for` or `match` whose bodies are child blocks. No `var`,
  assignment, `while`, `break`, `continue` or `return` (`unexpected-token`). An arm that adds
  nothing is `V => {}`.
- Elements and view calls stand only there or as the view's result (`element-outside-view`): not
  in a `fn`, a `let`, a list, a field or an argument. A view call gives the caller's `View<M>`.
- `??name` where an element stands draws a placeholder; a hole elsewhere in a view traps.
- A trap in a view keeps the last frame; a first frame that traps shows one error node.

| Element | Arguments | Notes |
|---|---|---|
| `Text` | `text` | fills the width, wraps |
| `Button` | `label`, `on: M` | hugs its label; a click sends `on` |
| `Input` | `text`, `on_change: Variant`, `key:` | fills; holds cursor, selection, focus |
| `Row` | `gap: Int = 0` | children left to right |
| `Col` | `gap: Int = 0` | children top to bottom |

Arguments keep table order; only `gap:` and `key:` may be left out, and `key:` is written last
(`wrong-label`). `on:` takes a message value, such as `Toggle(t.id)`. `on_change:` names a
variant with one String field; each edit sends the whole new text through it. The app holds the
text: the Input shows what the view passes. A negative literal gap is `literal-out-of-range`; a
computed one traps `assert`. Only a click on an Input moves focus; typed keys go to it.

Keys:

- `key:` takes an Int or a String; `3` and `"3"` differ. Any element or view call may carry one.
- An Input needs one, and so does every element or view call a `for` body adds, inside an `if` or
  `match` too (`missing-key`). A hole needs none.
- The root of a view carries no key and is not an Input (`key-on-root`). A key on a view call
  names that view's root.
- A key differs from its siblings' keys. The same literal key twice in one block is
  `duplicate-key`; siblings found sharing one at run time draw as one error node.
- Each child's step is its key, else `#n`, its line among the element lines of its block, counted
  from 0 through `if`, `match` and `for` bodies. A placeholder a loop adds is `#n.r`. An element
  keeps its cursor and focus while its steps and kind stay the same.

## 14. UI examples

```
ui_start(app) -> Ui                  a running app with no window; app is a visual app's name
ui_click(&ui, path)                  Button: sends on; Input: focus, cursor at the end
ui_type(&ui, text)                   one key per code point, to the focused Input
ui_tree(ui) -> String                the tree text
ui_text(ui, path) -> String          what a Text, Button or Input shows
ui_has(ui, path) -> Bool
```

- Only in example bodies (`ui-outside-example`); a `Ui` is neither stored nor compared.
  `ui_start` of anything but a visual app's name is `bad-ui-start`.
- A path joins steps with `/`: an Int key as digits, a String key in `'` quotes, `#n`. The root
  is `""`. `"{id}/#1"` builds one. A malformed literal path is `bad-ui-path`.
- Effects are not performed; one whose grant is missing ends the example `trapped`. Trap
  `assert`: a click on anything but a Button, Input or placeholder; `ui_text` on a Row, Col, error
  node or nothing; typing with no focused Input; any action after `Exit`.
- Status: touching a placeholder or reaching a hole, `unfinished`; another trap, `trapped`; an
  error node in a frame, `fail`; else the last line.

The tree text after `add_one`, one line per node, two spaces deeper per level: kind, step, shown
text, fields, then `focus`. A placeholder is `Hole`, an error node `Error`.

```
Col gap=8
  Input 'new' "" cursor=0 on_change=Draft focus
  Button #1 "Add" on=Add
  Row 1 gap=8
    Button #0 "[ ]" on=Toggle(1)
    Text #1 "milk"
  Hole #4 footer
```

## 15. Changing a running app

While a visual app runs under `paka serve`, an edit that changes a stored type (the state type
or one reached from it; defaults, comments and layout aside) is refused unless the edit, or an
earlier one still waiting, carries `state`: `reset`, a launch again from defaults, or the text of
one migrate. A migrate is not a definition and is never written to the source. When the new
`Counter` adds `step: Int = 1`:

```
migrate(old: Counter) -> Counter {
  Counter { count: old.count }
}
```

The parameter is the running state type, the result the new one (`bad-migrate-signature`).
`old` holds the running version's types; type names in the body are the new ones. An old value of
a changed type is only taken apart (`old-value-use`). No effects (`effect-in-migrate`). A changed
type that holds itself takes a reset (`migrate-needs-reset`). If the migrate traps, the old
version keeps running and the change waits.
