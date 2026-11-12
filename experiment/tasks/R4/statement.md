# Failure cascade

Read a file of services and the services each one needs, take some of them down, and print round
by round how the failure spreads.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, blank lines included. The file has at most 200 lines, and no
line is longer than 200 characters.

A word is a run of characters other than the space, as long as possible. A name is 1 to 20
characters, each a letter `A` to `Z` or `a` to `z`, a digit `0` to `9`, `_` or `-`. A line that
holds no word is blank and is skipped. Every other line must be one of:

- **A service line.** Its first word is a name directly followed by `:`, such as `web:`; that name
  is the service the line defines. Then come zero or more words that are names: the dependencies of
  the service, in order. The line may end with two more words, `/` and then the threshold `K`,
  which is 1 to 3 digits (`0` to `9`), leading zeros allowed.
- **A down line.** Exactly two words: `down` and a name.

So `down: db` defines a service named `down`, while `down db` is a down line.

## The cascade

Every service has a threshold: `K` when its line gives one, else the number of its dependencies. A
service may need itself.

In round 0 every service named on a down line goes down. Naming a service on several down lines is
the same as naming it once. Then rounds 1, 2, 3 and so on are run. In each round, every service that
is still up, has at least one dependency, and has at least `K` dependencies that are down goes down,
where `K` is its threshold. All services of a round are found from the state at the start of the
round, before any of them goes down. The rounds stop at the first round in which no service goes
down.

## Output

Print one line for each round in which some service went down, round 0 first:

```
round R: NAME NAME ...
```

with `R` in decimal and the names of the services that went down in that round in rising ASCII
order (a name that is the start of a longer name comes first), separated by single spaces. Then
print `up: NAME NAME ...` with the services that are still up, in the same order, or `up: none` when
every service is down.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
db:
cache: db
api: db cache / 1
web: api cache
down db
```

Output:

```
round 0: db
round 1: api cache
round 2: web
up: none
```

In round 1, `api` needs only one of its two dependencies down and `cache` needs its one: both go
down. `web` sees neither of them down at the start of round 1, so it goes down in round 2.

## Errors

Each error prints exactly one line, ending with a line feed, and nothing else, and exits with
status 20. `N` below is a line number in decimal.

First the lines are checked in order from line 1, and only the first line with an error is
reported. On that line, the checks are made in this order and only the first that fails is
reported:

1. The line is not blank, not a service line and not a down line: `error: line N: bad line`
2. It is a service line whose service is defined on an earlier line:
   `error: line N: duplicate service NAME`
3. It is a service line that lists the same dependency twice:
   `error: line N: duplicate dependency NAME`, naming the first dependency, from the left, that is
   the same as one before it.
4. It is a service line with a threshold `K` that is 0 or more than its number of dependencies:
   `error: line N: bad threshold`

When every line passes, the names are checked: in line order, and on each line from left to right,
every dependency and the name of every down line must be a service defined on some line, before or
after it. The first that is not gives `error: line N: unknown service NAME`.

When every name passes and the file has no down line, the error is `error: nothing down`.
