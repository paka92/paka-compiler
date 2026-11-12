import { readFileSync, writeFileSync } from "node:fs";

const [inputName, fieldArg, indexName] = process.argv.slice(2);

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

function fileName(key: string): string {
  return key.toLowerCase().replace(/[^a-z0-9]/g, "_") + ".txt";
}

function write(name: string, text: string): void {
  try {
    writeFileSync(name, text);
  } catch {
    fail(`cannot write ${name}`);
  }
}

function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

interface Group {
  key: string;
  rests: string[];
}

function main(): void {
  const field = Number(fieldArg);
  if (!/^[0-9]+$/.test(fieldArg) || field < 1 || field > 9) fail("bad field");

  let text: string;
  try {
    text = readFileSync(inputName, "utf8");
  } catch {
    return fail(`cannot read ${inputName}`);
  }
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();

  const groups = new Map<string, Group>();
  let records = 0;
  lines.forEach((line, i) => {
    if (line === "") return;
    const n = i + 1;
    const fields = line.split(",");
    if (fields.length < field) fail(`line ${n}: no field ${field}`);
    const key = fields[field - 1];
    if (key === "") fail(`line ${n}: empty key`);
    const name = fileName(key);
    let group = groups.get(name);
    if (group === undefined) {
      group = { key, rests: [] };
      groups.set(name, group);
    } else if (group.key !== key) {
      fail(`line ${n}: ${key} clashes with ${group.key} in ${name}`);
    }
    group.rests.push(fields.filter((_, k) => k !== field - 1).join(","));
    records++;
  });

  const sorted = [...groups].sort(([a], [b]) => (a < b ? -1 : 1));
  for (const [name, group] of sorted) {
    write(name, group.rests.map((rest) => rest + "\n").join(""));
  }
  write(
    indexName,
    sorted.map(([name, g]) => `${name} ${g.rests.length} ${g.key}\n`).join(""),
  );
  console.log(`split ${plural(records, "record")} into ${plural(groups.size, "file")}`);
}

try {
  main();
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
