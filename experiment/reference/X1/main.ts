import { readFileSync } from "node:fs";

const [file, countArg] = process.argv.slice(2);

function print(limit: number): void {
  const counts = new Map<string, number>();
  for (const word of readFileSync(file, "utf8").toLowerCase().match(/[a-z]+/g) ?? []) {
    counts.set(word, (counts.get(word) ?? 0) + 1);
  }
  const ranked = [...counts].sort(([a, x], [b, y]) => y - x || (a < b ? -1 : 1));
  if (ranked.length === 0) console.log("no words");
  for (const [word, n] of ranked.slice(0, limit)) console.log(`${n} ${word}`);
}

if (/^[0-9]+$/.test(countArg) && Number(countArg) >= 1 && Number(countArg) <= 1000) {
  print(Number(countArg));
} else {
  console.log("error: bad count");
  process.exitCode = 20;
}
