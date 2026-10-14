import { readFileSync } from "node:fs";

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

/** Cost and number of moves of the best path from a cell to the end. */
type Best = { cost: number; moves: number } | null;
type Move = [name: string, dr: number, dc: number];

/** The moves in tie-break order. */
const MOVES: Move[] = [
  ["R", 0, 1],
  ["D", 1, 0],
  ["X", 1, 1],
];

function readGrid(text: string): string[] {
  const rows = text.split("\n");
  if (rows[rows.length - 1] === "") rows.pop();
  if (rows.length === 0 || rows[0] === "") fail("empty grid");
  rows.forEach((row, i) => {
    const bad = row.search(/[^0-9#]/);
    if (bad >= 0) fail(`line ${i + 1} col ${bad + 1}: bad cell`);
  });
  const ragged = rows.findIndex((row) => row.length !== rows[0].length);
  if (ragged >= 0) fail(`line ${ragged + 1}: ragged row`);
  return rows;
}

function solve(g: string[]): string {
  const rows = g.length;
  const cols = g[0].length;
  const isOpen = (r: number, c: number) => r < rows && c < cols && g[r][c] !== "#";
  if (!isOpen(0, 0)) fail("start is a wall");
  if (!isOpen(rows - 1, cols - 1)) fail("end is a wall");

  const best: Best[][] = g.map((row) => [...row].map(() => null));
  const through = (r: number, c: number, [name, dr, dc]: Move): Best => {
    const corner = name !== "X" || (isOpen(r, c + 1) && isOpen(r + 1, c));
    if (!corner || !isOpen(r + dr, c + dc)) return null;
    const next = best[r + dr][c + dc];
    if (next === null) return null;
    const extra = name === "X" ? 2 : 0;
    return { cost: next.cost + Number(g[r][c]) + extra, moves: next.moves + 1 };
  };
  const same = (a: Best, b: Best) => a?.cost === b?.cost && a?.moves === b?.moves;

  best[rows - 1][cols - 1] = { cost: Number(g[rows - 1][cols - 1]), moves: 0 };
  for (let r = rows - 1; r >= 0; r--) {
    for (let c = cols - 1; c >= 0; c--) {
      if (!isOpen(r, c) || (r === rows - 1 && c === cols - 1)) continue;
      for (const move of MOVES) {
        const b = through(r, c, move);
        const old = best[r][c];
        if (b && (!old || b.cost < old.cost || (b.cost === old.cost && b.moves < old.moves))) {
          best[r][c] = b;
        }
      }
    }
  }
  const total = best[0][0] ?? fail("no path");

  let path = "";
  for (let r = 0, c = 0; r !== rows - 1 || c !== cols - 1; ) {
    const [name, dr, dc] = MOVES.find((m) => same(through(r, c, m), best[r][c]))!;
    path += name;
    r += dr;
    c += dc;
  }
  const runs = path.match(/R+|D+|X+/g) ?? [];
  const written = runs.map((run) => (run.length > 1 ? run[0] + run.length : run));
  const moves = runs.length ? `moves ${total.moves}: ${written.join(" ")}` : "moves 0";
  return `cost ${total.cost}\n${moves}`;
}

try {
  console.log(solve(readGrid(readFileSync(process.argv[2], "utf8"))));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
