import { readFileSync } from "node:fs";

type Node = { at: string; atom?: string; items: Node[] };
type Fn = { name: string; params: string[]; body: Node | null };
type Value = number | boolean | Fn;
type Env = Map<string, Value>;

const SPECIAL = ["if", "let", "define"];
const BUILTINS = ["+", "-", "*", "/", "<", "="];

class LispError extends Error {}

function fail(message: string): never {
  console.log(`error: ${message}`);
  process.exit(20);
}

// "n" number, "b" truth value, "s" symbol, "x" bad atom.
function kind(t: string): string {
  if (/^-?[0-9]{1,7}$/.test(t)) return "n";
  if (t === "#t" || t === "#f") return "b";
  return /^(#|-?[0-9])/.test(t) ? "x" : "s";
}

function nameOf(n: Node): string | null {
  return n.atom !== undefined && kind(n.atom) === "s" && !SPECIAL.includes(n.atom) ? n.atom : null;
}

function distinct(names: (string | null)[]): boolean {
  return names.every((n, i) => n !== null && names.indexOf(n) === i);
}

function parse(text: string): Node[] {
  const open: Node[] = [{ at: "", items: [] }];
  let line = 1;
  let col = 1;
  for (let i = 0; i < text.length; ) {
    const at = `line ${line} col ${col}`;
    const c = text[i];
    let n = 1;
    if (c === ";") n = (/^[^\n]*/.exec(text.slice(i)) ?? [""])[0].length;
    else if (c === "(") open.push({ at, items: [] });
    else if (c === ")") {
      if (open.length === 1) fail(`${at}: unexpected )`);
      const list = open.pop()!;
      open[open.length - 1].items.push(list);
    } else if (c !== " " && c !== "\n") {
      const t = /^[^ \n();]+/.exec(text.slice(i))![0];
      n = t.length;
      if (kind(t) === "x") fail(`${at}: bad atom`);
      open[open.length - 1].items.push({ at, atom: t, items: [] });
    }
    if (c === "\n") [line, col] = [line + 1, 1];
    else col += n;
    i += n;
  }
  if (open.length > 1) fail(`${open[1].at}: unclosed (`);
  return open[0].items;
}

function show(v: Value): string {
  if (typeof v === "number") return String(v);
  if (typeof v === "boolean") return v ? "#t" : "#f";
  return `<fn ${v.name}>`;
}

function builtin(name: string, a: Value, b: Value): Value {
  if (name === "=" && typeof a === "boolean" && typeof b === "boolean") return a === b;
  if (typeof a !== "number" || typeof b !== "number") throw new LispError("type error");
  if (name === "=") return a === b;
  if (name === "<") return a < b;
  if (name === "/" && b === 0) throw new LispError("division by zero");
  const r = { "+": a + b, "-": a - b, "*": a * b, "/": Math.trunc(a / b) }[name]! + 0;
  if (Math.abs(r) > 9999999) throw new LispError("overflow");
  return r;
}

const globals: Env = new Map();
let depth = 0;

function evaluate(node: Node, env: Env): Value {
  const err = (what: string) => new LispError(`${node.at}: ${what}`);
  if (node.atom !== undefined) {
    const t = node.atom;
    if (kind(t) === "n") return Number(t) + 0;
    if (kind(t) === "b") return t === "#t";
    const v = env.get(t) ?? globals.get(t) ?? (BUILTINS.includes(t) ? { name: t, params: [], body: null } : undefined);
    if (v === undefined) throw err(`unbound ${t}`);
    return v;
  }
  const items = node.items;
  if (items.length === 0) throw err("empty call");
  const head = items[0].atom;
  if (head === "define") throw err("bad define");
  if (head === "if") {
    if (items.length !== 4) throw err("bad if");
    const c = evaluate(items[1], env);
    if (typeof c !== "boolean") throw err("type error");
    return evaluate(items[c ? 2 : 3], env);
  }
  if (head === "let") {
    const pairs = items[1]?.items;
    if (items.length !== 3 || items[1].atom !== undefined || pairs.length % 2 !== 0) throw err("bad let");
    const names = pairs.filter((_, i) => i % 2 === 0).map(nameOf);
    if (!distinct(names)) throw err("bad let");
    const inner = new Map(env);
    names.forEach((name, k) => inner.set(name!, evaluate(pairs[2 * k + 1], env)));
    return evaluate(items[2], inner);
  }
  const [f, ...args] = items.map((item) => evaluate(item, env));
  if (typeof f !== "object") throw err("not a function");
  if (f.body === null) {
    if (args.length !== 2) throw err("wrong arity");
    try {
      return builtin(f.name, args[0], args[1]);
    } catch (e) {
      throw err((e as Error).message);
    }
  }
  if (f.params.length !== args.length) throw err("wrong arity");
  if (depth === 50) throw err("too deep");
  depth++;
  const v = evaluate(f.body, new Map(f.params.map((p, i) => [p, args[i]])));
  depth--;
  return v;
}

function define(form: Node): void {
  const [, target, body] = form.items;
  const bad = new LispError(`${form.at}: bad define`);
  if (form.items.length !== 3) throw bad;
  if (target.atom === undefined) {
    const names = target.items.map(nameOf);
    if (names.length === 0 || names[0] === null || !distinct(names.slice(1))) throw bad;
    globals.set(names[0], { name: names[0], params: names.slice(1) as string[], body });
  } else {
    const name = nameOf(target);
    if (name === null) throw bad;
    globals.set(name, evaluate(body, new Map()));
  }
}

function main(): void {
  for (const form of parse(readFileSync(process.argv[2], "utf8"))) {
    try {
      if (form.items[0]?.atom === "define") define(form);
      else console.log(show(evaluate(form, new Map())));
    } catch (e) {
      if (!(e instanceof LispError)) throw e;
      fail(e.message);
    }
  }
}

main();
