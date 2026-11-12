# Patch apply

Apply an edit script to a text file and write the result to a new file.

## Arguments

The program is run with exactly three arguments: the target name, the script name and the output
name. Each is 1 to 30 characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `.`,
`_`, `-` or `/`. The output name is never the script name, but it may be the target name: then the
target is replaced by the result.

## Files

Both input files hold only the characters with codes 32 to 126 and the line feed (code 10). Each is
a sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file
has no lines. Lines are numbered from 1. Each file has at most 200 lines of at most 200 characters.
Let `n` be the number of lines of the target.

## Script

Every line of the script is a command or a text line. There are no blank lines: an empty line, or
one of only spaces, is a bad line (see Errors).

A number is `0`, or a digit `1` to `9` followed by at most three digits `0` to `9`. So `07` and
`10000` are not numbers. A command is one of these, with exactly one space after the letter and
nothing else on the line, `A` and `B` numbers:

- `d A` or `d A-B`: delete lines `A` to `B` of the target (`d A` is `d A-A`).
- `c A` or `c A-B`: change lines `A` to `B` of the target: delete them and put the text lines of
  the command in their place.
- `a A`: add the text lines of the command after line `A` of the target; `a 0` adds them before
  line 1.

For `d` and `c`, `A` is at least 1 and `B` is at least `A`. A text line is `>` alone, which stands
for an empty line, or `> ` (a `>` and a space) followed by zero or more characters, which stand for
those characters exactly: `>   x` stands for `  x`, and `> > y` for `> y`. The text lines of a `c` or
`a` command are the text lines that follow it directly, up to the next command or the end of the
script; it has at least one. A `d` command has none.

The places of the target are, in order: gap 0, line 1, gap 1, line 2, and so on up to line `n` and
gap `n`. Gap `g` is the place between line `g` and line `g + 1`. A `d` or `c` command takes lines
`A` to `B` and the gaps between them; an `a` command takes gap `A`. Every command must take only
places that come after every place taken by the command before it in the script. So after `d 3-5`,
`a 5` and `d 6` may follow but `a 4` may not; after `a 5`, `d 6` may follow but `a 5` and `c 5` may
not.

## Output

The result is the target with every command applied, all of them to the target as read. Write it
to the output file: every line ends with a line feed, but one. When the last line of the target
lacks a line feed, is not deleted or changed, and no line is added after it, it is also the last
line of the result, and it lacks a line feed there too. A file of the same name that already
exists is replaced entirely. Then print one line and exit with status 0:

```
wrote K lines to OUTPUT (+I -D)
```

`K` is the number of lines of the result, `I` the number of text lines in the script and `D` the
number of target lines deleted or changed, all in decimal. When `K` is 1 the word is `line`.

Example: run with `notes.txt`, `fix.txt` and `new.txt`, where `notes.txt` holds

```
one
two
three
four
```

and `fix.txt` holds

```
d 2
c 3
> THREE
> 3.5
a 4
> five
```

The file `new.txt` is written with the lines `one`, `THREE`, `3.5`, `four` and `five`, and the
output is:

```
wrote 5 lines to new.txt (+3 -2)
```

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number of the script in decimal. The checks are made in this order
and only the first that fails is reported:

1. The script cannot be read, for example because it does not exist: `error: cannot read NAME`,
   with the script name.
2. The script lines are checked in order from line 1, and only the first line with an error is
   reported. On that line, the checks are made in this order:
   1. It is not a command or a text line: `error: script line N: bad line`. So `a 1-2`, `d 0`,
      `>x` and `D 1` are bad lines.
   2. It is a `d` or `c` command with `B` less than `A`: `error: script line N: backward range`
   3. It is a text line that does not follow a `c` or `a` command or one of its text lines:
      `error: script line N: unexpected text`
   4. It is a `c` or `a` command and the next line is not a text line, or there is no next line:
      `error: script line N: missing text`
3. The target cannot be read: `error: cannot read NAME`, with the target name.
4. The commands are checked in script order, and only the first command with an error is reported,
   with `N` its line. On that command, the checks are made in this order:
   1. It names a line after line `n` (for `a`, `A` is more than `n`): `error: script line N: past
      end`
   2. It takes a place that does not come after every place taken by the command before it:
      `error: script line N: out of order`
5. The output cannot be written, for example because its name names a folder that does not exist:
   `error: cannot write NAME`, with the output name.

When a check of 1 to 4 fails, no file is written. The program never creates a folder.
