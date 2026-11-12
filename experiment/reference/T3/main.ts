import { readFileSync } from "node:fs";

const [file, order] = process.argv.slice(2);

class Fail extends Error {}

type Chunk = { number: number } | { text: string };

function chunks(line: string): Chunk[] {
  return (line.match(/[0-9]+|[^0-9]+/g) ?? []).map((chunk) =>
    /^[0-9]/.test(chunk) ? { number: Number(chunk) } : { text: chunk.toLowerCase() },
  );
}

function compareChunks(a: Chunk, b: Chunk): number {
  if ("number" in a && "number" in b) return a.number < b.number ? -1 : a.number > b.number ? 1 : 0;
  if ("number" in a) return -1;
  if ("number" in b) return 1;
  return a.text < b.text ? -1 : a.text > b.text ? 1 : 0;
}

function compareLines(a: Chunk[], b: Chunk[]): number {
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    const result = compareChunks(a[i], b[i]);
    if (result !== 0) return result;
  }
  return a.length - b.length;
}

function sorted(text: string): string[] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const long = lines.findIndex((line) => /[0-9]{16}/.test(line));
  if (long >= 0) throw new Fail(`line ${long + 1}: number too long`);
  const sign = order === "up" ? 1 : -1;
  return lines
    .map((line) => ({ line, key: chunks(line) }))
    .sort((a, b) => sign * compareLines(a.key, b.key))
    .map(({ line }) => line);
}

try {
  if (order !== "up" && order !== "down") throw new Fail("bad order");
  for (const line of sorted(readFileSync(file, "utf8"))) console.log(line);
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
