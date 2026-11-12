import { readFileSync } from "node:fs";

interface Entry {
  op: " " | "-" | "+";
  text: string;
}

function readLines(name: string): string[] {
  const lines = readFileSync(name, "utf8").split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

function diff(a: string[], b: string[]): Entry[] {
  const n = a.length;
  const m = b.length;
  const common = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      common[i][j] =
        a[i] === b[j]
          ? common[i + 1][j + 1] + 1
          : Math.max(common[i + 1][j], common[i][j + 1]);
    }
  }
  const list: Entry[] = [];
  let i = 0;
  let j = 0;
  while (i < n || j < m) {
    if (i < n && j < m && a[i] === b[j]) {
      list.push({ op: " ", text: a[i++] });
      j++;
    } else if (j === m || (i < n && common[i + 1][j] >= common[i][j + 1])) {
      list.push({ op: "-", text: a[i++] });
    } else {
      list.push({ op: "+", text: b[j++] });
    }
  }
  return list;
}

const usesOld = (e: Entry) => e.op !== "+";
const usesNew = (e: Entry) => e.op !== "-";

function range(before: Entry[], hunk: Entry[], uses: (e: Entry) => boolean): string {
  const count = hunk.filter(uses).length;
  const start = before.filter(uses).length + (count > 0 ? 1 : 0);
  return `${start},${count}`;
}

function hunks(list: Entry[], context: number): string[] {
  const changes = list.flatMap((e, i) => (e.op === " " ? [] : [i]));
  const groups: number[][] = [];
  for (const index of changes) {
    const group = groups[groups.length - 1];
    if (group !== undefined && index - group[1] - 1 <= 2 * context) group[1] = index;
    else groups.push([index, index]);
  }
  return groups.flatMap(([first, last]) => {
    const start = Math.max(0, first - context);
    const hunk = list.slice(start, last + context + 1);
    const before = list.slice(0, start);
    return [
      `@@ -${range(before, hunk, usesOld)} +${range(before, hunk, usesNew)} @@`,
      ...hunk.map((e) => e.op + e.text),
    ];
  });
}

const [oldName, newName, contextArg] = process.argv.slice(2);
const context = Number(contextArg);

if (!/^[0-9]+$/.test(contextArg) || context > 10) {
  console.log("error: bad context");
  process.exitCode = 20;
} else {
  const out = hunks(diff(readLines(oldName), readLines(newName)), context);
  console.log(out.length === 0 ? "no differences" : out.join("\n"));
}
