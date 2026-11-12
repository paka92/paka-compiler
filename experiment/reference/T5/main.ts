import { readFileSync } from "node:fs";

type Key = { field: number; start: number; kind: string; down: boolean };

const KEY = /^([1-9][0-9]?)(?:\.([1-9][0-9]?))?([ni]?)(-?)$/;
const NUMBER = /^(-?)([0-9]{1,9})(?:\.([0-9]{1,6}))?$/;

// The value of a number text in millionths, or null when it is not a number.
function numberValue(text: string): number | null {
  const m = NUMBER.exec(text);
  if (!m) return null;
  const value = Number(m[2]) * 1e6 + Number((m[3] ?? "").padEnd(6, "0"));
  return m[1] === "-" ? -value : value;
}

function keyText(line: string, key: Key): string {
  return (line.split(",")[key.field - 1] ?? "").slice(key.start - 1);
}

function compareBy(key: Key, a: string, b: string): number {
  let x: string | number | null = keyText(a, key);
  let y: string | number | null = keyText(b, key);
  if (key.kind === "i") [x, y] = [x.toLowerCase(), y.toLowerCase()];
  if (key.kind === "n") [x, y] = [numberValue(x), numberValue(y)];
  let order = 0;
  if (x === null || y === null) order = (x === null ? 0 : 1) - (y === null ? 0 : 1);
  else order = x < y ? -1 : x > y ? 1 : 0;
  return key.down ? -order : order;
}

function main(): void {
  const [file, list] = process.argv.slice(2);
  const keys: Key[] = list.split(",").map((text, i) => {
    const m = KEY.exec(text);
    if (!m) {
      console.log(`error: bad key ${i + 1}`);
      process.exit(20);
    }
    return { field: Number(m[1]), start: Number(m[2] ?? "1"), kind: m[3], down: m[4] === "-" };
  });
  const lines = readFileSync(file, "utf8").split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  lines.sort((a, b) => {
    for (const key of keys) {
      const order = compareBy(key, a, b);
      if (order !== 0) return order;
    }
    return 0;
  });
  for (const line of lines) console.log(line);
}

main();
