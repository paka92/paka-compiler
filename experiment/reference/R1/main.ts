import { readFileSync } from "node:fs";

const TARGET = /^[A-Za-z0-9_-]{1,20}:$/;
const NAME = /^[A-Za-z0-9_-]{1,20}$/;

class Fail extends Error {}

// Maps each name to its dependencies; a name that is never a target maps to none.
function parseRules(text: string): Map<string, Set<string>> {
  const targets = new Set<string>();
  const deps = new Map<string, Set<string>>();
  text.split("\n").forEach((line, i) => {
    const [first, ...rest] = line.split(" ").filter((word) => word !== "");
    if (first === undefined) return;
    if (!TARGET.test(first) || !rest.every((word) => NAME.test(word))) {
      throw new Fail(`line ${i + 1}: bad line`);
    }
    const target = first.slice(0, -1);
    if (targets.has(target)) throw new Fail(`line ${i + 1}: duplicate target ${target}`);
    targets.add(target);
    for (const name of [target, ...rest]) {
      if (!deps.has(name)) deps.set(name, new Set());
    }
    rest.forEach((name) => deps.get(target)!.add(name));
  });
  return deps;
}

function buildOrder(deps: Map<string, Set<string>>): string[] {
  const names = [...deps.keys()].sort();
  const order: string[] = [];
  const done = new Set<string>();
  for (;;) {
    const ready = names.find(
      (name) => !done.has(name) && [...deps.get(name)!].every((dep) => done.has(dep)),
    );
    if (ready === undefined) return order;
    order.push(ready);
    done.add(ready);
  }
}

function findCycle(deps: Map<string, Set<string>>, left: string[]): string[] {
  const path: string[] = [];
  let name: string | undefined = left[0];
  while (name !== undefined && !path.includes(name)) {
    path.push(name);
    name = [...deps.get(name)!].filter((dep) => left.includes(dep)).sort()[0];
  }
  const cycle = path.slice(path.indexOf(name!));
  const start = cycle.indexOf([...cycle].sort()[0]);
  return [...cycle.slice(start), ...cycle.slice(0, start)];
}

try {
  const deps = parseRules(readFileSync(process.argv[2], "utf8"));
  const order = buildOrder(deps);
  const left = [...deps.keys()].filter((name) => !order.includes(name)).sort();
  if (left.length === 0) {
    if (order.length > 0) console.log(order.join("\n"));
  } else {
    const cycle = findCycle(deps, left);
    console.log(`cycle: ${[...cycle, cycle[0]].join(" -> ")}`);
    process.exitCode = 20;
  }
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
