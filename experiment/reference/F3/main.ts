import { readFileSync, writeFileSync } from "node:fs";

class Fail extends Error {}

// `kind` is d, c or a; d and c take lines `from` to `to`, and a has `to` equal to `from`.
interface Command {
  line: number;
  kind: string;
  from: number;
  to: number;
  text: string[];
}

const NUMBER = "(0|[1-9][0-9]{0,3})";
const COMMAND = new RegExp(`^(?:([dc]) ${NUMBER}(?:-${NUMBER})?|(a) ${NUMBER})$`);

const splitLines = (text: string) => {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
};

function textLine(line: string): string | undefined {
  if (line === ">") return "";
  return line.startsWith("> ") ? line.slice(2) : undefined;
}

function parseScript(text: string): Command[] {
  const lines = splitLines(text);
  const commands: Command[] = [];
  lines.forEach((line, i) => {
    const at = (why: string) => new Fail(`script line ${i + 1}: ${why}`);
    const text = textLine(line);
    if (text !== undefined) {
      const last = commands[commands.length - 1];
      if (last === undefined || last.kind === "d") throw at("unexpected text");
      last.text.push(text);
      return;
    }
    const m = COMMAND.exec(line);
    if (m === null) throw at("bad line");
    const kind = m[1] ?? m[4];
    const from = Number(m[2] ?? m[5]);
    const to = Number(m[3] ?? m[2] ?? m[5]);
    if (kind !== "a" && from < 1) throw at("bad line");
    if (to < from) throw at("backward range");
    if (kind !== "d" && (i + 1 >= lines.length || textLine(lines[i + 1]) === undefined)) {
      throw at("missing text");
    }
    commands.push({ line: i + 1, kind, from, to, text: [] });
  });
  return commands;
}

// The first and last place taken: gap g is 2g, line l is 2l - 1.
const places = (c: Command) => (c.kind === "a" ? [2 * c.from, 2 * c.from] : [2 * c.from - 1, 2 * c.to - 1]);

function apply(target: string, commands: Command[]): string {
  const lines = splitLines(target);
  const n = lines.length;
  let lastPlace = -1;
  for (const c of commands) {
    if (c.to > n) throw new Fail(`script line ${c.line}: past end`);
    const [first, last] = places(c);
    if (first <= lastPlace) throw new Fail(`script line ${c.line}: out of order`);
    lastPlace = last;
  }
  const out: string[] = [];
  let next = 0; // index of the next target line to copy
  let lastKept = n > 0 && !target.endsWith("\n");
  for (const c of commands) {
    const upto = c.kind === "a" ? c.from : c.from - 1;
    out.push(...lines.slice(next, upto), ...c.text);
    next = c.kind === "a" ? upto : c.to;
    if (c.to === n) lastKept = false;
  }
  out.push(...lines.slice(next));
  const text = out.map((l) => l + "\n").join("");
  return lastKept ? text.slice(0, -1) : text;
}

function read(name: string): string {
  try {
    return readFileSync(name, "utf8");
  } catch {
    throw new Fail(`cannot read ${name}`);
  }
}

function run(target: string, script: string, output: string): string {
  const commands = parseScript(read(script));
  const text = apply(read(target), commands);
  try {
    writeFileSync(output, text);
  } catch {
    throw new Fail(`cannot write ${output}`);
  }
  const added = commands.reduce((sum, c) => sum + c.text.length, 0);
  const gone = commands.reduce((sum, c) => sum + (c.kind === "a" ? 0 : c.to - c.from + 1), 0);
  const k = splitLines(text).length;
  return `wrote ${k} ${k === 1 ? "line" : "lines"} to ${output} (+${added} -${gone})`;
}

try {
  const [target, script, output] = process.argv.slice(2);
  console.log(run(target, script, output));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
