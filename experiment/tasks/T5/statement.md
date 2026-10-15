# Key sort

Sort the lines of a file of comma-separated fields by sort keys given on the command line.

## Arguments

The program is run with exactly two arguments: the name of the input file, which always exists and
can be read, then the key list.

## Input

The file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines; a file holding only a line feed has one empty line. The file has at most 200 lines of at
most 200 characters.

Every line is a record, an empty line included. Its fields are the parts between commas: a line
with `k` commas has `k + 1` fields, and an empty line has one empty field. Fields are numbered from
1 and are taken exactly as written, spaces included. A field that a line does not have counts as
an empty field.

## The key list

The key list is one or more keys separated by single commas, with nothing else around them. A key
is written:

```
FIELD [ "." START ] [ "n" | "i" ] [ "-" ]
```

Square brackets mark a part that may be left out. `FIELD` and `START` are whole numbers from 1 to
99, written in decimal without a leading zero. So `2`, `3.4n-`, `1i` and `12.2-` are keys, while
`0`, `02`, `100`, `1.0`, `1-n`, `1ni`, `1--`, `n` and the empty text are not.

The key's text for a line is field `FIELD` of that line, without its first `START - 1` characters.
`START` is 1 when left out. When the field has fewer than `START` characters, the key's text is
empty. For example, with the key `2.3`, the line `x,ab-17,y` gives the text `-17`.

## Comparing two lines

Two lines are compared key by key, in the order of the key list. For each key, the key's texts of
the two lines are compared by the key's type:

- **No letter: text.** Compare the texts character by character by ASCII code. At the first place
  where they differ, the text with the smaller code comes first. When one text is the start of the
  other, the shorter comes first. Equal texts are equal.
- **`i`: text, case ignored.** As text, after changing every upper-case letter `A` to `Z` into its
  lower-case letter in both texts.
- **`n`: numeric.** A text is a number when it is an optional `-`, then 1 to 9 digits, then
  optionally a `.` followed by 1 to 6 digits, and nothing else. Its value is the decimal number it
  writes, so `2.50` and `2.5` are equal, and `0`, `-0` and `0.000` are all equal. Two numbers
  compare by value, the smaller first. A text that is not a number comes before every number, and
  two texts that are not numbers are equal under this key.
- **`-` after the key** turns the key around: of two lines that the key does not find equal, the
  one that would come first comes second. Lines the key finds equal stay equal.

The first key that does not find the two lines equal decides their order. When every key finds
them equal, they keep their order from the input file, whatever the `-` marks.

## Output

Print every line of the input, exactly as written, in sorted order, each followed by a line feed.
An empty file prints nothing. The exit status is 0.

Example: the key list `2n-,1i` and the input

```
pear,3
Apple,10
fig,x
apple,10.0
Banana,3
```

give

```
Apple,10
apple,10.0
Banana,3
pear,3
fig,x
```

The values 10 and 10.0 are equal, and so are `apple` and `Apple` with case ignored, so those two
lines keep their input order. `x` is not a number, so it comes before every number, and the `-`
turns that around.

## Errors

When the key list is wrong, print exactly one line, `error: bad key K`, ending with a line feed,
and nothing else, and exit with status 20. `K` is the place in the key list of the first key that
is not written as above, counted from 1, in decimal. The key list is split at every comma first,
so `1,,2` has an empty second key and `1,` an empty second key. The key list is checked before the
file is read.
