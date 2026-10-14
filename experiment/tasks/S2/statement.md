# Ledger with holds

Run a list of account commands, with money in whole cents, and print every account at the end.

## Arguments

The program is run with exactly one argument: the name of the input file, which always exists and
can be read.

## Input

The input file holds only the characters with codes 32 to 126 and the line feed (code 10). It is a
sequence of lines: each line ends at a line feed, and the last line may lack one. An empty file has
no lines. Lines are numbered from 1, empty lines included. The file has at most 200 lines.

An empty line is skipped. Every other line must be a command: a command word and its fields,
separated by single spaces, with no space at the start or the end of the line. There are six
commands:

```
open NAME
deposit NAME AMOUNT
hold NAME ID AMOUNT
capture ID AMOUNT
release ID
transfer FROM TO AMOUNT
```

- `NAME`, `FROM` and `TO` are account names: one or more lower-case letters `a` to `z`.
- `ID` is a hold id: one or more of the characters `a` to `z` and `0` to `9`.
- `AMOUNT` is one to nine digits, a `.`, and exactly two digits, with a value of at least `0.01`.
  Leading zeros are allowed: `007.50` is 7 units and 50 cents. So `5`, `5.0`, `.50`, `-1.00`,
  `0.00` and `1234567890.00` are not amounts.

With these limits no balance, sum or difference is ever more than 19999999999800 cents.

A line that is not empty and is not one of these commands, written exactly this way, is malformed.

## Accounts and holds

Each account has a balance, which starts at `0.00` when the account is opened. A hold sets aside
part of one account's balance; while the hold is active, its amount is held. The available money of
an account is its balance minus the amounts of all its active holds. Hold ids are shared by all
accounts.

The commands run in file order. Each one either succeeds or is rejected. A rejected command prints
`line N: REASON`, with `N` its line number in decimal, and changes nothing. Each command checks the
reasons in the order listed and reports only the first that applies.

- `open NAME`: rejected with `account exists NAME` when the account is already open. Otherwise it
  opens the account.
- `deposit NAME AMOUNT`: rejected with `no account NAME` when the account is not open. Otherwise the
  balance grows by `AMOUNT`.
- `hold NAME ID AMOUNT`: rejected with `no account NAME`; then with `hold exists ID` when an earlier
  hold that succeeded used this id, whether that hold is still active or not; then with
  `insufficient funds` when the available money is less than `AMOUNT`. Otherwise a new active hold
  of `AMOUNT` is made on the account.
- `capture ID AMOUNT`: rejected with `no hold ID` when no active hold has this id; then with
  `capture exceeds hold` when `AMOUNT` is more than the hold's amount. Otherwise the balance of the
  hold's account shrinks by `AMOUNT` and the hold ends; nothing of its amount is held any more.
- `release ID`: rejected with `no hold ID` when no active hold has this id. Otherwise the hold ends
  and the balance is unchanged.
- `transfer FROM TO AMOUNT`: rejected with `no account FROM`; then with `no account TO`; then with
  `same account` when `FROM` and `TO` are the same; then with `insufficient funds` when the
  available money of `FROM` is less than `AMOUNT`. Otherwise `FROM`'s balance shrinks by `AMOUNT`
  and `TO`'s grows by it.

## Output

Print the rejection lines in the order of their commands. Then print one line per open account, in
rising ASCII order of the names (a name that is the start of a longer name comes first):

```
NAME: balance B, held H
```

`B` is the balance and `H` the sum of the account's active holds. Money is written as its whole
units in decimal with no leading zeros (`0` when there are none), a `.`, and the cents as two
digits: 1205 cents is `12.05` and 7 cents is `0.07`. When no account is open at the end, print the
single line `no accounts` instead of the account lines.

Every printed line ends with a line feed. The exit status is 0.

Example input:

```
open kim
deposit kim 5.00
hold kim h1 2.00
transfer kim lee 4.00
```

Output:

```
line 4: no account lee
kim: balance 5.00, held 2.00
```

## Errors

Before any command runs, every line is checked. When a line is malformed, print exactly this line
for the first malformed line, with `N` its line number in decimal, ending with a line feed, and
nothing else, and exit with status 20:

```
error: line N: bad command
```
