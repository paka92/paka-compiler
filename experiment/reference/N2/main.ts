import { readFileSync } from "node:fs";

class Fail extends Error {}

const GROUP = /^[A-Za-z0-9_-]{1,20}:$/;
const VALUE = /^-?[0-9]{1,5}$/;

function parseGroups(text: string): Map<string, number[]> {
  const groups = new Map<string, number[]>();
  text.split("\n").forEach((line, i) => {
    const [first, ...rest] = line.split(" ").filter((word) => word !== "");
    if (first === undefined) return;
    if (!GROUP.test(first)) throw new Fail(`line ${i + 1}: bad line`);
    const bad = rest.find((word) => !VALUE.test(word));
    if (bad !== undefined) throw new Fail(`line ${i + 1}: bad value ${bad}`);
    const name = first.slice(0, -1);
    groups.set(name, [...(groups.get(name) ?? []), ...rest.map(Number)]);
  });
  if (groups.size === 0) throw new Fail("no groups");
  return groups;
}

function fixed(x: number): string {
  const text = x.toFixed(2);
  return text === "-0.00" ? "0.00" : text;
}

function describe(name: string, values: number[]): string {
  const n = values.length;
  const sorted = [...values].sort((a, b) => a - b);
  const mean = values.reduce((a, b) => a + b, 0) / n;
  const median = (sorted[Math.floor((n - 1) / 2)] + sorted[Math.floor(n / 2)]) / 2;
  const squares = values.reduce((q, v) => q + (v - mean) ** 2, 0);
  const sd = Math.sqrt(squares / (n - 1));
  const [m, d, s] = [
    n >= 1 ? fixed(mean) : "-",
    n >= 1 ? fixed(median) : "-",
    n >= 2 ? fixed(sd) : "-",
  ];
  return `${name}: n=${n} mean=${m} median=${d} sd=${s}`;
}

try {
  const groups = parseGroups(readFileSync(process.argv[2], "utf8"));
  const names = [...groups.keys()].sort();
  console.log(names.map((name) => describe(name, groups.get(name)!)).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
