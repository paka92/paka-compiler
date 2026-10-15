import { readFileSync } from "node:fs";

class Fail extends Error {}

type Piece =
  | { text: string }
  | { ref: string }
  | { len: string }
  | { name: string; fallback: Piece[] };

const isStart = (c: string | undefined) => c !== undefined && /^[a-z]$/.test(c);

class Line {
  i = 0;
  constructor(
    readonly cs: string,
    readonly n: number,
  ) {}

  fail(col: number, message: string): never {
    throw new Fail(`line ${this.n} col ${col}: ${message}`);
  }

  name(): string {
    const start = this.i;
    while (this.i < this.cs.length && /[a-z0-9_]/.test(this.cs[this.i])) this.i++;
    return this.cs.slice(start, this.i);
  }

  // Pieces up to the line end, or up to the `}` closing a default when `outer` is set.
  pieces(outer?: number): Piece[] {
    const out: Piece[] = [];
    for (;;) {
      const c = this.cs[this.i];
      if (c === undefined) return outer === undefined ? out : this.fail(outer, "unclosed brace");
      if (c === "}" && outer !== undefined) return out;
      const col = ++this.i;
      if (c !== "$") {
        out.push({ text: c });
      } else if (this.cs[this.i] === "$") {
        this.i++;
        out.push({ text: "$" });
      } else if (this.cs[this.i] === "{") {
        this.i++;
        out.push(this.reference(outer ?? col, col));
      } else if (isStart(this.cs[this.i])) {
        out.push({ ref: this.name() });
      } else {
        this.fail(col, "bad dollar");
      }
    }
  }

  reference(outer: number, col: number): Piece {
    const hash = this.cs[this.i] === "#";
    if (hash) this.i++;
    if (this.i === this.cs.length) this.fail(outer, "unclosed brace");
    if (!isStart(this.cs[this.i])) this.fail(col, "bad reference");
    const name = this.name();
    const [a, b] = [this.cs[this.i], this.cs[this.i + 1]];
    if (a === undefined) this.fail(outer, "unclosed brace");
    if (a === "}") {
      this.i++;
      return hash ? { len: name } : { ref: name };
    }
    if (hash || a !== ":") this.fail(col, "bad reference");
    if (b === undefined) this.fail(outer, "unclosed brace");
    if (b !== "-") this.fail(col, "bad reference");
    this.i += 2;
    const fallback = this.pieces(outer);
    this.i++;
    return { name, fallback };
  }
}

function definitions(text: string): Map<string, Piece[]> {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const defs = new Map<string, Piece[]>();
  lines.forEach((cs, k) => {
    if (cs === "" || cs.startsWith("#")) return;
    const line = new Line(cs, k + 1);
    if (!isStart(cs[0])) line.fail(1, "bad name");
    const name = line.name();
    if (cs[line.i] !== "=") line.fail(line.i + 1, "bad name");
    line.i++;
    defs.set(name, line.pieces());
  });
  return defs;
}

function expandAll(defs: Map<string, Piece[]>): string[] {
  const done = new Map<string, string>();
  const chain: string[] = [];
  const expand = (name: string): string => {
    const known = done.get(name);
    if (known !== undefined) return known;
    const at = chain.indexOf(name);
    if (at >= 0) throw new Fail(`cycle ${[...chain.slice(at), name].join(" -> ")}`);
    chain.push(name);
    const value = pieces(defs.get(name)!);
    chain.pop();
    done.set(name, value);
    return value;
  };
  const lookup = (name: string): string => {
    if (!defs.has(name)) throw new Fail(`undefined ${name} in ${chain[chain.length - 1]}`);
    return expand(name);
  };
  const pieces = (ps: Piece[]): string =>
    ps
      .map((p) => {
        if ("text" in p) return p.text;
        if ("ref" in p) return lookup(p.ref);
        if ("len" in p) return String(lookup(p.len).length);
        const value = defs.has(p.name) ? expand(p.name) : "";
        return value === "" ? pieces(p.fallback) : value;
      })
      .join("");
  return [...defs.keys()].map((name) => `${name}=${expand(name)}`);
}

try {
  const out = expandAll(definitions(readFileSync(process.argv[2], "utf8")));
  for (const line of out) console.log(line);
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
