import { readFileSync } from "node:fs";

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

type Service = { deps: string[]; threshold: number };

const isName = (s: string) => /^[A-Za-z0-9_-]{1,20}$/.test(s);
const ascii = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

/** The service a line defines, or null when its words are not a service line. */
function parseService(words: string[]): [string, Service, boolean] | null {
  const name = words[0].endsWith(":") ? words[0].slice(0, -1) : "";
  if (!isName(name)) return null;
  let deps = words.slice(1);
  let threshold = -1;
  if (deps.length >= 2 && deps[deps.length - 2] === "/") {
    if (!/^[0-9]{1,3}$/.test(deps[deps.length - 1])) return null;
    threshold = Number(deps[deps.length - 1]);
    deps = deps.slice(0, -2);
  }
  if (!deps.every(isName)) return null;
  const given = threshold >= 0;
  return [name, { deps, threshold: given ? threshold : deps.length }, given];
}

function run(text: string): string[] {
  const services = new Map<string, Service>();
  const used: [number, string][] = [];
  const down: string[] = [];
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  lines.forEach((line, i) => {
    const n = i + 1;
    const words = line.split(" ").filter((w) => w !== "");
    if (words.length === 0) return;
    if (words.length === 2 && words[0] === "down") {
      if (!isName(words[1])) fail(`line ${n}: bad line`);
      used.push([n, words[1]]);
      down.push(words[1]);
      return;
    }
    const parsed = parseService(words) ?? fail(`line ${n}: bad line`);
    const [name, service, given] = parsed;
    if (services.has(name)) fail(`line ${n}: duplicate service ${name}`);
    const twice = service.deps.find((d, j) => service.deps.indexOf(d) < j);
    if (twice !== undefined) fail(`line ${n}: duplicate dependency ${twice}`);
    if (given && (service.threshold === 0 || service.threshold > service.deps.length)) {
      fail(`line ${n}: bad threshold`);
    }
    service.deps.forEach((d) => used.push([n, d]));
    services.set(name, service);
  });
  const unknown = used.find(([, name]) => !services.has(name));
  if (unknown) fail(`line ${unknown[0]}: unknown service ${unknown[1]}`);
  if (down.length === 0) fail("nothing down");
  return cascade(services, down);
}

function cascade(services: Map<string, Service>, first: string[]): string[] {
  const names = [...services.keys()].sort(ascii);
  const isDown = new Set<string>();
  let falling = [...new Set(first)].sort(ascii);
  const out: string[] = [];
  for (let r = 0; falling.length > 0; r++) {
    out.push(`round ${r}: ${falling.join(" ")}`);
    falling.forEach((name) => isDown.add(name));
    falling = names.filter((name) => {
      const { deps, threshold } = services.get(name)!;
      const count = deps.filter((d) => isDown.has(d)).length;
      return !isDown.has(name) && threshold > 0 && count >= threshold;
    });
  }
  const up = names.filter((name) => !isDown.has(name));
  out.push(`up: ${up.length ? up.join(" ") : "none"}`);
  return out;
}

try {
  console.log(run(readFileSync(process.argv[2], "utf8")).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
