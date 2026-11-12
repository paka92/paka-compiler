# Grade book

Work out each student's weighted score from part scores, then grade and rank the students.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 100 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A line that holds no word
is blank and is skipped. A name is 1 to 20 characters, each a letter `A` to `Z` or `a` to `z`, a
digit `0` to `9`, `_` or `-`. Names are compared exactly, case included.

The first line that is not blank is the parts line. Its first word is `parts`, and each later word
is a part: a name, then `:`, then the part's weight, which is 1 to 3 digits with a value from 1 to
100. Leading zeros are allowed: `030` is 30. The weights must add up to exactly 100. Let `P` be the
number of parts.

Every later line that is not blank is a student line: a name, then exactly `P` scores, one for each
part in the order of the parts line. A student line whose name is `parts` is still a student line.
A score is one of:

- 1 to 3 digits with a value from 0 to 100, leading zeros allowed: the student's percent on that
  part;
- the same, followed directly by `L`: handed in late. Its value is 10 less, but never below 0, so
  `85L` counts as 75 and `4L` as 0;
- `x`: excused. The part does not count for this student.

## Scores

For one student, let `S` be the sum of the weights of the parts that are not excused, and `T` the
sum, over those parts, of the weight times the value of the score. When `S` is 0 the student has no
score. Otherwise the student's score is `T` divided by `S`, exactly, without rounding. Every
comparison below uses this exact score.

The grade is `A` for a score of at least 90, `B` for at least 80, `C` for at least 70, `D` for at
least 60, `F` below 60, and `I` for a student with no score.

The rank of a student with a score is 1 plus the number of students whose score is higher. Equal
scores share a rank: scores of 90, 85, 85 and 70 rank 1, 2, 2 and 4.

The median is worked out from the scores of the students who have one, sorted: the middle score
when their number is odd, and the mean of the two middle scores, exactly, when it is even.

## Output

A score or median is printed rounded to one decimal: to the nearest multiple of 0.1, and when the
exact value is halfway between two of them, to the larger one. It is written as the whole part with
no leading zeros (`0` when it is below one), a `.` and the one decimal digit. So 83.25 prints as
`83.3` and 89.96 as `90.0`. The grade is still decided by the exact score: 89.96 is a `B`.

Print one line per student who has a score, in order of rank; students of equal rank are in ASCII
order of their names (character by character from the left, by character code, and at the first
difference the smaller code comes first; when one name is the start of the other, the shorter one
comes first):

```
RANK NAME SCORE GRADE
```

Then one line per student with no score, in ASCII order of their names:

```
- NAME - I
```

Then the line `median M`, with `M` the median as above, or `-` when no student has a score. Then
the line

```
grades A=a B=b C=c D=d F=f I=i
```

with the number of students of each grade in decimal.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
parts hw:30 lab:20 exam:50
ann 90 80L 85
bob x 100 x
cy 95 70 x
```

Output:

```
1 bob 100.0 A
2 cy 85.0 B
3 ann 83.5 B
median 85.0
grades A=1 B=2 C=0 D=0 F=0 I=0
```

ann's late lab counts as 70, so `T` is 30 x 90 + 20 x 70 + 50 x 85 = 8350 and `S` is 100. bob has
only the lab: `T` 2000, `S` 20. cy has no exam: `T` 30 x 95 + 20 x 70 = 4250, `S` 50.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number in decimal, and `WORD` a word exactly as written.

The lines are checked in order from line 1, and only the first line with an error is reported. On
that line, the checks are made in the order listed and only the first that fails is reported.

On the parts line:

1. The first word is not `parts`: `error: line N: expected parts`
2. There is no later word: `error: line N: no parts`
3. The later words are checked from left to right. For each word, in this order:
   1. It does not hold exactly one `:`, or the part before the `:` is not a name:
      `error: line N: bad part WORD`
   2. The part after the `:` is not a weight: `error: line N: bad weight WORD`
   3. The name appeared earlier on the line: `error: line N: duplicate part NAME`, with `NAME` the
      name.
4. The weights do not add up to 100: `error: line N: weights sum to T`, with `T` their sum in
   decimal.

On a student line:

1. The first word is not a name: `error: line N: bad name WORD`
2. The name is that of a student on an earlier line: `error: line N: duplicate student NAME`
3. The line does not hold exactly `P` words after the name: `error: line N: expected P scores, got
   K`, with `K` the number of words after the name, both in decimal.
4. The words after the name are checked from left to right, and the first that is not a score is
   reported: `error: line N: bad score WORD`

When every line is good: if the file holds no parts line, the error is `error: no parts`; if it
holds a parts line but no student line, the error is `error: no students`.
