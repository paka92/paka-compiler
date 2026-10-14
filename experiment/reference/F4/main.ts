import { readFileSync, writeFileSync } from "node:fs";

class Fail extends Error {}

// OLD in lower case: NEW as written.
type RenameMap = Map<string, string>;

const isKey = (s: string) => /^[A-Za-z0-9_-]{1,20}$/.test(s);
const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

function read(name: string): string {
  try {
    return readFileSync(name, "utf8");
  } catch {
    throw new Fail(`cannot read ${name}`);
  }
}

function parseMap(text: string): RenameMap {
  const map: RenameMap = new Map();
  text.split("\n").forEach((line, i) => {
    const words = line.split(" ").filter((w) => w !== "");
    if (words.length === 0) return;
    const at = (why: string) => new Fail(`map line ${i + 1}: ${why}`);
    if (words.length !== 2 || !words.every(isKey)) throw at("bad entry");
    const [old, renamed] = words;
    if (map.has(old.toLowerCase())) throw at(`duplicate ${old}`);
    map.set(old.toLowerCase(), renamed);
  });
  if (map.size === 0) throw new Fail("empty map");
  return map;
}

// The page with its links rewritten, and the number rewritten.
function rewrite(name: string, text: string, map: RenameMap): [string, number] {
  let count = 0;
  const lines = text.split("\n").map((line, i) => {
    let out = "";
    let pos = 0;
    for (let start = line.indexOf("[[", pos); start >= 0; start = line.indexOf("[[", pos)) {
      const at = (why: string) => new Fail(`${name} line ${i + 1} col ${start + 1}: ${why}`);
      const end = line.indexOf("]]", start + 2);
      if (end < 0) throw at("unclosed link");
      const parts = line.slice(start + 2, end).split("|");
      if (parts.length > 2 || !isKey(parts[0])) throw at("bad link");
      const renamed = map.get(parts[0].toLowerCase());
      if (renamed !== undefined) {
        parts[0] = renamed;
        count++;
      }
      out += line.slice(pos, start) + "[[" + parts.join("|") + "]]";
      pos = end + 2;
    }
    return out + line.slice(pos);
  });
  return [lines.join("\n"), count];
}

function run(mapName: string, pages: string[]): string {
  const map = parseMap(read(mapName));
  const texts = pages.map(read);
  const results = pages.map((page, i) => rewrite(page, texts[i], map));
  let report = "";
  let links = 0;
  let written = 0;
  results.forEach(([text, count], i) => {
    if (count > 0) {
      try {
        writeFileSync(pages[i], text);
      } catch {
        throw new Fail(`cannot write ${pages[i]}`);
      }
      written++;
    }
    links += count;
    report += `${pages[i]}: ${plural(count, "link")}\n`;
  });
  return report + `rewrote ${plural(links, "link")} in ${plural(written, "page")}\n`;
}

try {
  const [mapName, list] = process.argv.slice(2);
  const pages = readFileSync(list, "utf8").split("\n").slice(0, -1);
  process.stdout.write(run(mapName, pages));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
