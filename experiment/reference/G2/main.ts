import { readFileSync } from "node:fs";

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

function readGrid(name: string): string[][] {
  const lines = readFileSync(name, "utf8").split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  if (lines.length === 0 || lines[0] === "") fail("empty grid");
  lines.forEach((line, i) => {
    const bad = line.search(/[^.#o]/);
    if (bad >= 0) fail(`line ${i + 1} col ${bad + 1}: bad cell`);
    if (line.length !== lines[0].length) fail(`line ${i + 1}: ragged row`);
  });
  return lines.map((line) => [...line]);
}

function isEmpty(grid: string[][], row: number, col: number): boolean {
  return grid[row][col] === ".";
}

function step(grid: string[][]): boolean {
  let moved = false;
  const move = (row: number, col: number, toCol: number) => {
    grid[row][col] = ".";
    grid[row + 1][toCol] = "o";
    moved = true;
  };
  for (let row = grid.length - 2; row >= 0; row--) {
    for (let col = 0; col < grid[row].length; col++) {
      if (grid[row][col] !== "o") continue;
      if (isEmpty(grid, row + 1, col)) {
        move(row, col, col);
      } else if (col > 0 && isEmpty(grid, row, col - 1) && isEmpty(grid, row + 1, col - 1)) {
        move(row, col, col - 1);
      } else if (
        col + 1 < grid[row].length &&
        isEmpty(grid, row, col + 1) &&
        isEmpty(grid, row + 1, col + 1)
      ) {
        move(row, col, col + 1);
      }
    }
  }
  return moved;
}

function main(): void {
  const [gridName, stepsArg] = process.argv.slice(2);
  const limit = Number(stepsArg);
  if (!/^[0-9]+$/.test(stepsArg) || limit > 1000) fail("bad steps");
  const grid = readGrid(gridName);

  let steps = 0;
  while (steps < limit && step(grid)) steps++;
  const summary = steps < limit ? `${steps}, settled` : `${limit}, limit`;
  console.log([...grid.map((row) => row.join("")), `steps: ${summary}`].join("\n"));
}

try {
  main();
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
