import { readFileSync, writeFileSync } from "node:fs";

const [mode, first, second] = process.argv.slice(2);
const NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,29}$/;
const HEADER = /^archive (0|[1-9][0-9]{0,2})$/;
const ENTRY = /^entry ([A-Za-z0-9][A-Za-z0-9._-]{0,29}) (0|[1-9][0-9]{0,4})$/;

class Fail extends Error {}

interface Entry {
  name: string;
  content: string;
}

function read(name: string): string {
  try {
    return readFileSync(name, "utf8");
  } catch {
    throw new Fail(`cannot read ${name}`);
  }
}

function write(path: string, text: string): void {
  try {
    writeFileSync(path, text);
  } catch {
    throw new Fail(`cannot write ${path}`);
  }
}

function files(count: number): string {
  return `${count} ${count === 1 ? "file" : "files"}`;
}

function pack(list: string, archive: string): string {
  const names = read(list).split("\n");
  if (names[names.length - 1] === "") names.pop();
  names.forEach((name, i) => {
    if (!NAME.test(name)) throw new Fail(`list line ${i + 1}: bad name`);
    if (names.indexOf(name) < i) throw new Fail(`list line ${i + 1}: duplicate ${name}`);
  });
  const entries = names.map((name) => ({ name, content: read(name) }));
  const text = entries.map((e) => `entry ${e.name} ${e.content.length}\n${e.content}\n`);
  write(archive, `archive ${entries.length}\n${text.join("")}`);
  return `packed ${files(entries.length)} into ${archive}`;
}

function parseArchive(text: string): Entry[] {
  // The archive line number of the line that starts at index `at`.
  const lineAt = (at: number) => text.slice(0, at).split("\n").length;
  const bad = (at: number, reason: string) =>
    new Fail(`archive line ${lineAt(at)}: ${reason}`);
  // The line starting at `at`, without its line feed, when it ends in one.
  const lineFrom = (at: number) => {
    const end = text.indexOf("\n", at);
    return end < 0 ? undefined : text.slice(at, end);
  };

  const header = HEADER.exec(lineFrom(0) ?? "");
  if (!header) throw bad(0, "bad header");
  let at = lineFrom(0)!.length + 1;
  const entries: Entry[] = [];
  for (let n = Number(header[1]); n > 0; n--) {
    const match = ENTRY.exec(lineFrom(at) ?? "");
    if (!match) throw bad(at, "bad entry");
    const [, name, length] = match;
    if (entries.some((e) => e.name === name)) throw bad(at, `duplicate ${name}`);
    const start = at + lineFrom(at)!.length + 1;
    const end = start + Number(length);
    if (text[end] !== "\n") throw bad(at, "bad length");
    entries.push({ name, content: text.slice(start, end) });
    at = end + 1;
  }
  if (at < text.length) throw bad(at, "trailing data");
  return entries;
}

function unpack(archive: string, which: string): string {
  if (which !== "*" && !NAME.test(which)) throw new Fail(`bad name ${which}`);
  const entries = parseArchive(read(archive));
  const chosen = which === "*" ? entries : entries.filter((e) => e.name === which);
  if (chosen.length === 0 && which !== "*") throw new Fail(`no entry ${which}`);
  for (const { name, content } of chosen) write(name, content);
  return `unpacked ${files(chosen.length)} from ${archive}`;
}

try {
  if (mode !== "pack" && mode !== "unpack") throw new Fail("bad mode");
  console.log(mode === "pack" ? pack(first, second) : unpack(first, second));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
