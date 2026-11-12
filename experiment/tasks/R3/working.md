# R3 hand working

Not part of the statement. One entry per case: the links, the groups, then each link with the
reason it is or is not a bridge. Second check: a group of `n` names joined only by bridges has
exactly `n - 1` bridges, and a link on a cycle is never a bridge.

## cases/1 (`map.txt`)

Links: pier-dock, dock-town, town-pier (a triangle), town-hill, hill-farm, farm-mill, mill-hill (a
triangle). All six names form one group: `dock farm hill mill pier town` (d, f, h, m, p, t).

Each triangle link has another way round its triangle, so it is not a bridge. Taking away
town-hill leaves {pier, dock, town} and {hill, farm, mill}: a bridge, written `hill town`.

## cases/2 (`short.txt`)

Line 1 `n1 n2` is a link. Line 2 `n3` holds one word: `error: line 2: bad link`, status 20.

## hidden/1 (`three.txt`)

Line 1 `p q` is a link. Line 2 `p q r` has three words: `error: line 2: bad link`, status 20.

## hidden/2 (`name.txt`)

Line 1 `k1 k2` is a link. Line 2 `p.q r`: `p.q` holds `.`, not a name character: `error: line 2:
bad link`, status 20. Line 3 `s` (one word) is never reached.

## hidden/3 (`twin.txt`)

Links: m-n, n-m (the same two names again, written the other way), n-o. One group `m n o`.

- Taking away line 1 leaves line 2 joining m and n: not a bridge. The same for line 2.
- Taking away n-o leaves o alone: a bridge, `n o`.

## hidden/4 (`loop.txt`)

Links: solo-solo, k-w, w-w. Names solo, k, w. Groups: {k, w} and {solo}; first names k and solo, so
`k w` then `solo`. The two self links are never bridges. Taking away k-w leaves k and w apart (w-w
does not help): a bridge, `k w`.

## hidden/5 (`case.txt`)

Links: zed-Alpha, beta-zed, Beta-beta, _q--x. Groups: {Alpha, Beta, beta, zed} and {-x, _q}.
Character codes: `-` 45, `A` 65, `B` 66, `_` 95, `b` 98, `z` 122.

- Group lines: `-x _q` and `Alpha Beta beta zed`; `-x` comes before `Alpha`, so that group first.
- No cycle: all four links are bridges. Smaller name first: `Alpha zed`, `beta zed`, `Beta beta`,
  `-x _q`. By first names (-x, Alpha, Beta, beta): `-x _q`, `Alpha zed`, `Beta beta`, `beta zed`.

## hidden/6 (`none.txt`)

Two spaces, an empty line, then one space without a line feed: all blank. No link: `groups: 0`,
`bridges: 0`, status 0.

## hidden/7 (`two.txt`)

Links: square s1-s2-s3-s4-s1, then s4-t1, triangle t1-t2-t3-t1, then t3-u1, u1-u2. All nine names
are one group: `s1 s2 s3 s4 t1 t2 t3 u1 u2`.

- Square and triangle links each have another way round: not bridges.
- s4-t1: without it the square and the rest are apart. Bridge `s4 t1`.
- t3-u1: without it {u1, u2} is apart. Bridge `t3 u1`.
- u1-u2: without it u2 is alone. Bridge `u1 u2`.

Three bridges, first names s4, t3, u1.

## hidden/8 (`spaces.txt`)

Line 1 `   hub  a2` (leading and double spaces), line 2 `a10 hub`, line 3 empty (blank), line 4
`hub a1`, line 5 `b hub`, line 6 ` b c9 ` (spaces at both ends), line 7 `b c10` without a line
feed. Links: hub-a2, a10-hub, hub-a1, b-hub, b-c9, b-c10.

Seven names, six links, all connected: a tree, so every link is a bridge (check: 7 - 1 = 6).

- Group: `a1 a10 a2 b c10 c9 hub`. a1 is the start of a10; a10 before a2 since `1` 49 < `2` 50;
  c10 before c9 since `1` < `9`.
- Bridges, smaller name first: `a2 hub`, `a10 hub`, `a1 hub`, `b hub`, `b c9`, `b c10`. By first
  name: a1, a10, a2, then three with first name b, ordered by second name: c10, c9, hub.
