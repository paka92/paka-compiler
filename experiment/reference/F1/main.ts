import { readFileSync, writeFileSync } from "node:fs";

const [rootName, outName] = process.argv.slice(2);
const INCLUDE = /^#include [A-Za-z0-9][A-Za-z0-9._-]{0,29}$/;

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

function readLines(name: string): string[] | undefined {
  try {
    const lines = readFileSync(name, "utf8").split("\n");
    if (lines[lines.length - 1] === "") lines.pop();
    return lines;
  } catch {
    return undefined;
  }
}

let filesRead = 0;
const chain: string[] = [];
const merged = new Set<string>();
const output: string[] = [];

function merge(file: string, lines: string[]): void {
  filesRead++;
  chain.push(file);
  lines.forEach((line, i) => {
    if (!line.startsWith("#include")) {
      output.push(line);
      return;
    }
    const where = `${file} line ${i + 1}`;
    if (!INCLUDE.test(line)) fail(`${where}: bad include`);
    const name = line.slice("#include ".length);
    if (chain.includes(name)) {
      fail(`${where}: include cycle: ${[...chain, name].join(" -> ")}`);
    }
    if (merged.has(name)) return;
    merge(name, readLines(name) ?? fail(`${where}: cannot read ${name}`));
  });
  chain.pop();
  merged.add(file);
}

function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

try {
  merge(rootName, readLines(rootName) ?? fail(`cannot read ${rootName}`));
  try {
    writeFileSync(outName, output.map((line) => line + "\n").join(""));
  } catch {
    fail(`cannot write ${outName}`);
  }
  console.log(`read ${plural(filesRead, "file")}, wrote ${plural(output.length, "line")} to ${outName}`);
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
