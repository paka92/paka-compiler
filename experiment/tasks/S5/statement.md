# Line editor with undo

Run a script of editing commands on a buffer of lines, with undo and redo.

## Arguments

The program is run with exactly one argument: the name of the script file, which always exists and
can be read.

## Input

The script holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1. The script has at most 200 lines.

Every line is one command, written exactly as one of these:

```
add TEXT
set TEXT
del
up
down
undo
redo
print
```

For `add` and `set`, the command word is followed by one space, and `TEXT` is the whole rest of the
line: it may be empty and may hold spaces anywhere, at its start and end included. The other
commands are the word alone, with nothing before or after it.

## The buffer

The buffer is a list of lines of text, numbered from 1, and it starts empty. It has a current line
`C`: 0 when the buffer is empty, else a line number from 1 to `N`, the number of lines. The state
is the buffer together with `C`.

The commands run in script order. A command may be rejected: it then prints `line L: REASON`, with
`L` its line number in the script in decimal, and changes nothing. A command checks the reasons in
the order listed and reports only the first that applies.

- `add TEXT`: puts a new line holding `TEXT` directly after the current line (as line 1 when the
  buffer is empty); the new line becomes the current line.
- `set TEXT`: rejected with `empty buffer` when the buffer is empty. Otherwise, when the current
  line already holds exactly `TEXT`, nothing happens and nothing is printed. Otherwise the current
  line's text becomes `TEXT`.
- `del`: rejected with `empty buffer` when the buffer is empty. Otherwise the current line is
  removed. The line that came after it, which now has its number, becomes the current line; when
  there is none, the line before it does; when the buffer is now empty, `C` is 0.
- `up`: rejected with `empty buffer`; then with `at top` when `C` is 1. Otherwise `C` goes down by
  1.
- `down`: rejected with `empty buffer`; then with `at bottom` when `C` is `N`. Otherwise `C` goes
  up by 1.
- `undo`: rejected with `nothing to undo` when the undo list is empty. Otherwise the last step of
  the undo list is taken off it, the state becomes the one before that step, and the step is put at
  the end of the redo list.
- `redo`: rejected with `nothing to redo` when the redo list is empty. Otherwise the last step of
  the redo list is taken off it, the state becomes the one after that step, and the step is put back
  at the end of the undo list.
- `print`: prints the buffer, as given under Output.

## Steps

The undo and redo lists start empty. Each `add`, each `set` that changes the text and each `del`
that is not rejected is an edit, and each edit either makes a new step or joins the last one:

- An `add` joins the last step of the undo list when the command on the script line just before it
  is also an `add`. The step then covers both: its state before stays as it was, and its state
  after becomes the state after this `add`.
- Every other edit makes a new step, whose state before and state after are the states just before
  and just after the edit. The step is put at the end of the undo list and the redo list is emptied.
  The undo list keeps at most 3 steps: when a new step makes it 4, the first step of the list is
  dropped and can never be undone.

`up`, `down`, `print`, rejected commands and a `set` that changes nothing are not edits. They make
no step, but they do stand between two `add` lines, so the second `add` makes a new step. Undo and
redo set the whole state, `C` included, to the state saved in the step, whatever `C` was just
before.

## Output

A rejected command prints its line when it runs. `print` prints `[C/N]`, with both numbers in
decimal, and then one line per line of the buffer, in order: a mark, the line number, a `|` and the
line's text. The mark is `>` for the current line and a space for every other line. The line number
is in decimal, with spaces put in front of it so that it is as wide as `N` written in decimal.

After the last command, print one more line, `undo U, redo R`, with `U` and `R` the number of steps
in the undo and redo lists, in decimal.

Every printed line ends with a line feed. The exit status is 0.

Example script:

```
add one
add two
up
set uno
down
undo
print
del
```

Output:

```
[1/2]
>1|one
 2|two
undo 2, redo 0
```

The two `add` commands make one step. `up` makes line 1 current, `set uno` makes a second step and
`down` makes line 2 current. `undo` brings back the state before `set`, with line 1 current again.
`del` removes line 1, so `two` is now line 1 and current; a new step is made and the redo list is
emptied.

## Errors

Before any command runs, the whole script is checked. When a line is not a command written exactly
as above, print this line for the first such line, with `L` its line number in decimal, ending
with a line feed, and nothing else, and exit with status 20:

```
error: line L: bad command
```

An empty line is not a command.
