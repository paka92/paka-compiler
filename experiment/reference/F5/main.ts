import { readFileSync, writeFileSync } from "node:fs";

class Fail extends Error {}

interface Chapter {
  title: string;
  lines: string[];
}

const isBlank = (line: string) => /^ *$/.test(line);
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

// The title of a heading line, or undefined for any other line.
function heading(line: string): string | undefined {
  if (line === "#") return "";
  return line.startsWith("# ") ? line.slice(2).replace(/^ +| +$/g, "") : undefined;
}

function slug(title: string): string {
  const s = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 20)
    .replace(/-$/, "");
  return s === "" ? "chapter" : s;
}

function chapters(text: string): Chapter[] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const found: Chapter[] = [];
  lines.forEach((line, i) => {
    const title = heading(line);
    const last = found[found.length - 1];
    if (title === "") throw new Fail(`line ${i + 1}: empty title`);
    if (title !== undefined) found.push({ title, lines: [] });
    else if (last !== undefined) last.lines.push(line);
    else if (!isBlank(line)) throw new Fail(`line ${i + 1}: text before first chapter`);
  });
  if (found.length === 0) throw new Fail("no chapters");
  for (const c of found) {
    while (c.lines.length > 0 && isBlank(c.lines[c.lines.length - 1])) c.lines.pop();
    while (c.lines.length > 0 && isBlank(c.lines[0])) c.lines.shift();
  }
  return found;
}

function bases(found: Chapter[]): string[] {
  const taken: string[] = [];
  for (const c of found) {
    const s = slug(c.title);
    let base = s;
    for (let n = 2; taken.includes(base); n++) base = `${s}-${n}`;
    taken.push(base);
  }
  return taken;
}

function write(name: string, text: string) {
  try {
    writeFileSync(name, text);
  } catch {
    throw new Fail(`cannot write ${name}`);
  }
}

function run(book: string, tocName: string): string {
  let text: string;
  try {
    text = readFileSync(book, "utf8");
  } catch {
    throw new Fail(`cannot read ${book}`);
  }
  const found = chapters(text);
  let toc = "";
  bases(found).forEach((base, k) => {
    const c = found[k];
    const name = `${base}.txt`;
    write(name, [`# ${c.title}`, ...c.lines].map((l) => l + "\n").join(""));
    toc += `${k + 1}. ${c.title} -> ${name} (${plural(c.lines.length, "line")})\n`;
  });
  write(tocName, toc);
  return `split ${book} into ${plural(found.length, "chapter")}`;
}

try {
  const [book, tocName] = process.argv.slice(2);
  console.log(run(book, tocName));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
