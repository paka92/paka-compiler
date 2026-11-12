import { readFileSync, writeFileSync } from "node:fs";

const [first, second, output] = process.argv.slice(2);

class Fail extends Error {}

function readLines(name: string): string[] {
  let text: string;
  try {
    text = readFileSync(name, "utf8");
  } catch {
    throw new Fail(`cannot read ${name}`);
  }
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

function join(): number {
  const lines = [...readLines(first), ...readLines(second)];
  const width = String(lines.length).length;
  const numbered = lines.map((text, i) => {
    const prefix = `${String(i + 1).padStart(width)} |`;
    return (text === "" ? prefix : `${prefix} ${text}`) + "\n";
  });
  try {
    writeFileSync(output, numbered.join(""));
  } catch {
    throw new Fail(`cannot write ${output}`);
  }
  return lines.length;
}

try {
  const count = join();
  console.log(`wrote ${count} ${count === 1 ? "line" : "lines"} to ${output}`);
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
