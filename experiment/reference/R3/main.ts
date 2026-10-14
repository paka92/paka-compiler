import { readFileSync } from "node:fs";

const NAME = /^[A-Za-z0-9_-]{1,20}$/;

class Fail extends Error {}

type Link = [string, string];

function parse(text: string): Link[] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const links: Link[] = [];
  lines.forEach((line, i) => {
    const words = line.split(" ").filter((word) => word !== "");
    if (words.length === 0) return;
    const [a, b] = words;
    if (words.length !== 2 || !NAME.test(a) || !NAME.test(b)) {
      throw new Fail(`line ${i + 1}: bad link`);
    }
    links.push(a < b ? [a, b] : [b, a]);
  });
  return links;
}

// Maps every name to a representative of its group, leaving out the link at index `skip`.
function groupsOf(names: string[], links: Link[], skip = -1): Map<string, string> {
  const parent = new Map(names.map((name) => [name, name]));
  const find = (name: string): string => {
    const up = parent.get(name)!;
    return up === name ? name : find(up);
  };
  links.forEach(([a, b], i) => {
    if (i !== skip) parent.set(find(a), find(b));
  });
  return new Map(names.map((name) => [name, find(name)]));
}

function compare(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function report(links: Link[]): string[] {
  const names = [...new Set(links.flat())].sort();
  const group = groupsOf(names, links);
  const members = new Map<string, string[]>();
  for (const name of names) {
    const root = group.get(name)!;
    members.set(root, [...(members.get(root) ?? []), name]);
  }
  const groups = [...members.values()].sort((x, y) => compare(x[0], y[0]));
  const bridges = links
    .filter(([a, b], i) => {
      const without = groupsOf(names, links, i);
      return without.get(a) !== without.get(b);
    })
    .sort((x, y) => compare(x[0], y[0]) || compare(x[1], y[1]));
  return [
    `groups: ${groups.length}`,
    ...groups.map((g) => g.join(" ")),
    `bridges: ${bridges.length}`,
    ...bridges.map((link) => link.join(" ")),
  ];
}

try {
  console.log(report(parse(readFileSync(process.argv[2], "utf8"))).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
