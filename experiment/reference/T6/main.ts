import { readFileSync } from "node:fs";

const NUMBER = /^-?(0|[1-9][0-9]{0,3})$/;
const COUNT: Record<string, number> = { hline: 4, vline: 4, text: 4, box: 5, fill: 6, copy: 7 };
const SIZES: Record<string, number[]> = { hline: [3], vline: [3], box: [3, 4], fill: [3, 4], copy: [3, 4], text: [] };

function isNumber(w: string): boolean {
  return NUMBER.test(w) && w !== "-0";
}

function fail(message: string): never {
  console.log(message);
  process.exit(20);
}

// The error in one command line, if any.
function check(words: string[]): string | null {
  const name = words[0];
  const count = COUNT[name];
  if (count === undefined) return "unknown command";
  if (name === "text" ? words.length < 4 : words.length !== count) return "wrong count";
  const numbers = name === "text" ? 2 : name === "fill" ? 4 : count - 1;
  if (words.slice(1, numbers + 1).some((w) => !isNumber(w))) return "bad number";
  if (name === "fill" && words[5].length !== 1) return "bad char";
  if (SIZES[name].some((i) => Number(words[i]) < 1)) return "bad size";
  return null;
}

class Canvas {
  cells: string[][];
  constructor(readonly w: number, readonly h: number) {
    this.cells = Array.from({ length: h }, () => Array<string>(w).fill(" "));
  }

  on(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < this.w && y < this.h;
  }

  put(x: number, y: number, c: string): void {
    if (this.on(x, y)) this.cells[y][x] = c;
  }

  mark(x: number, y: number, m: string): void {
    if (!this.on(x, y)) return;
    const old = this.cells[y][x];
    const cross = m === "+" || (m === "-" && (old === "|" || old === "+")) || (m === "|" && (old === "-" || old === "+"));
    this.cells[y][x] = cross ? "+" : m;
  }

  // Calls f on every cell of the rectangle that lies on the canvas.
  each(x0: number, y0: number, w: number, h: number, f: (x: number, y: number) => void): void {
    for (let y = Math.max(y0, 0); y < Math.min(y0 + h, this.h); y++) {
      for (let x = Math.max(x0, 0); x < Math.min(x0 + w, this.w); x++) f(x, y);
    }
  }

  run(words: string[]): void {
    const n = words.slice(1).map(Number);
    switch (words[0]) {
      case "hline":
        return this.each(n[0], n[1], n[2], 1, (x, y) => this.mark(x, y, "-"));
      case "vline":
        return this.each(n[0], n[1], 1, n[2], (x, y) => this.mark(x, y, "|"));
      case "text":
        return [...words.slice(3).join(" ")].forEach((c, i) => this.put(n[0] + i, n[1], c));
      case "fill":
        return this.each(n[0], n[1], n[2], n[3], (x, y) => this.put(x, y, words[5]));
      case "box":
        return this.each(n[0], n[1], n[2], n[3], (x, y) => {
          const row = y === n[1] || y === n[1] + n[3] - 1;
          const col = x === n[0] || x === n[0] + n[2] - 1;
          if (row || col) this.mark(x, y, row && col ? "+" : row ? "-" : "|");
        });
      default: {
        const old = this.cells.map((r) => [...r]);
        this.each(n[0], n[1], n[2], n[3], (x, y) => this.put(x + n[4] - n[0], y + n[5] - n[1], old[y][x]));
      }
    }
  }
}

function main(): void {
  const lines = readFileSync(process.argv[2], "utf8").split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const head = (lines[0] ?? "").split(" ");
  const sizeOk = (w: string) => isNumber(w) && Number(w) >= 1 && Number(w) <= 80;
  if (lines.length === 0 || head.length !== 2 || !head.every(sizeOk)) fail("error: line 1: bad header");
  const commands: string[][] = [];
  lines.forEach((line, i) => {
    if (i === 0 || line === "" || line.startsWith("#")) return;
    const words = line.split(" ");
    const what = check(words);
    if (what !== null) fail(`error: line ${i + 1}: ${what}`);
    commands.push(words);
  });
  const canvas = new Canvas(Number(head[0]), Number(head[1]));
  commands.forEach((words) => canvas.run(words));
  for (const row of canvas.cells) console.log(row.join("").replace(/ +$/, ""));
}

main();
