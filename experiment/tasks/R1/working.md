# R1 hand working

Not part of the statement. One entry per case: the rules, then each step of the order with the
ready names in ASCII order ("print" adds the name to the order, which is printed at the end), or
the walk that finds the cycle. Second check: in every printed order each name comes after all of
its dependencies.

## cases/1 (`deps.txt`)

Rules: web needs api, ui; api needs db, log; ui needs log; db none; cli needs api. log is never a
target, so it has no dependencies.

1. Ready: db, log. Print db.
2. Ready: log. Print log.
3. Ready: api (db, log printed), ui (log printed). Print api.
4. Ready: cli (api printed), ui. `c` comes before `u`: print cli, although ui was ready first.
5. Ready: ui. Print ui. 6. Ready: web (api, ui printed). Print web.

Output db, log, api, cli, ui, web. Check: api after db and log, ui after log, cli after api, web
after api and ui.

## cases/2 (`loop.txt`)

Rules: main needs core, net; net needs core, tls; tls needs net; core none. core is ready and
placed in the order (never printed, because the run ends in a cycle). Then main waits for net, net
for tls and tls for net: nothing is ready.

Left: main, net, tls. Walk: main; its left dependencies are net (core is placed), so step to net;
net's left dependency is tls; tls's is net, reached a second time. Cycle: net, tls. Its first name
in ASCII order is net. `cycle: net -> tls -> net`, status 20.

## hidden/1 (`bad.txt`)

Lines 1 `lib:` and 2 `app: lib` are rules. Line 3 `tool:lib` is one word that is not a name
followed by `:` alone: `error: line 3: bad line`, status 20.

## hidden/2 (`twice.txt`)

Lines 1 (target x) and 2 (target y) are rules; together they form a cycle. Line 3 `x: z` is a rule
whose target x is the target of line 1: `error: line 3: duplicate target x`, status 20. Line 4
`q:: r` (bad) comes later, and the cycle is never looked for, since lines are checked first.

## hidden/3 (`ascii.txt`)

Rules: mn needs m; kilo needs Kilo, _tmp; m-n, 7up, -q none. m, Kilo and _tmp are never targets.
Character codes used: `-` 45, `7` 55, `K` 75, `_` 95, `k` 107, `m` 109, `n` 110.

1. Ready: -q, 7up, Kilo, _tmp, m, m-n. Print -q.
2. Print 7up. 3. Print Kilo (kilo still waits for _tmp).
4. Print _tmp. Now kilo is ready.
5. Ready: kilo, m, m-n. `k` comes before `m`: print kilo.
6. Ready: m, m-n. m is the start of m-n, so m first. Print m. Now mn is ready.
7. Ready: m-n, mn. Second characters `-` 45 and `n` 110: print m-n. 8. Print mn.

Output -q, 7up, Kilo, _tmp, kilo, m, m-n, mn. Check: kilo after Kilo and _tmp, mn after m.

## hidden/4 (`self.txt`)

Rules: gen needs gen and util; util none. util is ready and placed; gen waits for itself. Left:
gen. Walk: gen; its left dependency is gen (util is placed), reached a second time. Cycle: gen alone.
`cycle: gen -> gen`, status 20.

## hidden/5 (`rotate.txt`)

Rules: b none; d needs b; a needs t; t needs x; x needs c; c needs t. b, then d, are ready and
placed; then nothing is ready.

Left: a, c, t, x. Walk: a, t, x, c, then t a second time. Cycle: t, x, c (a is not on it). Its
first name in ASCII order is c; from c the steps go to t, then x. `cycle: c -> t -> x -> c`,
status 20.

## hidden/6 (`dup.txt`)

Line 1 has leading spaces and several spaces between words; its first word is `zip:`. Line 2 is
three spaces, blank. The last line lacks a line feed. Rules: zip needs unzip (listed twice, counts
once) and lz; lz none; mid needs zip; alpha needs lz. unzip is never a target.

1. Ready: lz, unzip. Print lz.
2. Ready: alpha (lz printed), unzip. Print alpha.
3. Ready: unzip. Print unzip. Now zip is ready: both its dependencies are printed.
4. Print zip. 5. Print mid.

Output lz, alpha, unzip, zip, mid. Check: zip after unzip and lz, mid after zip, alpha after lz.

## hidden/7 (`blank.txt`)

An empty line, three spaces, an empty line: all blank. No rule, no name: nothing is printed,
status 0.

## hidden/8 (`skip.txt`)

Rules: a needs b, z; z needs y; y needs z; b none; d needs c; c needs d. b is ready and placed; then
nothing is ready. Two cycles exist: y, z and c, d.

Left: a, c, d, y, z. Walk: a; its dependency b comes first in ASCII order but is placed, so the
step goes to z; z to y; y to z, reached a second time. Cycle: z, y. First in ASCII order: y.
`cycle: y -> z -> y`, status 20. The cycle c, d holds a smaller name but is never reached.
