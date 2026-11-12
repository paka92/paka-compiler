import { readFileSync } from "node:fs";

interface Pos {
  line: number;
  col: number;
}

interface Token extends Pos {
  kind: "name" | "string" | "symbol";
  text: string;
}

type Value =
  | { kind: "string"; text: string }
  | { kind: "list"; items: Value[] }
  | { kind: "table"; entries: Entry[] };

interface Entry {
  name: string;
  at: Pos;
  value: Value;
}

class Fail extends Error {
  constructor(
    message: string,
    readonly at: Pos,
  ) {
    super(message);
  }
}

// Reads the string whose opening quote is at `start`; returns its text and the index after it.
function readString(text: string, start: number, line: number): [string, number] {
  let value = "";
  let i = start + 1;
  while (i < text.length) {
    const c = text[i];
    if (c === '"') return [value, i + 1];
    if (c !== "\\") {
      value += c;
      i++;
      continue;
    }
    const next = text[i + 1];
    const hex = text.slice(i + 2, i + 4);
    const code = parseInt(hex, 16);
    if (next === '"' || next === "\\") {
      value += next;
      i += 2;
    } else if (next === "x" && /^[0-9a-fA-F]{2}$/.test(hex) && code >= 32 && code <= 126) {
      value += String.fromCharCode(code);
      i += 4;
    } else {
      throw new Fail("bad escape", { line, col: i + 1 });
    }
  }
  throw new Fail("unterminated string", { line, col: start + 1 });
}

function scanLine(text: string, line: number, tokens: Token[]): void {
  let i = 0;
  while (i < text.length && text[i] !== "#") {
    const c = text[i];
    const col = i + 1;
    if (c === " ") {
      i++;
    } else if (c === '"') {
      const [value, end] = readString(text, i, line);
      tokens.push({ kind: "string", text: value, line, col });
      i = end;
    } else if (/[A-Za-z0-9_]/.test(c)) {
      const word = /^[A-Za-z0-9_]+/.exec(text.slice(i))![0];
      tokens.push({ kind: "name", text: word, line, col });
      i += word.length;
    } else if ("=[]{},".includes(c)) {
      tokens.push({ kind: "symbol", text: c, line, col });
      i++;
    } else {
      throw new Fail("bad character", { line, col });
    }
  }
}

function parse(tokens: Token[], end: Pos): Entry[] {
  let pos = 0;
  const syntaxError = (): never => {
    throw new Fail("syntax error", tokens[pos] ?? end);
  };
  const accept = (symbol: string): boolean => {
    const token = tokens[pos];
    if (token === undefined || token.kind !== "symbol" || token.text !== symbol) return false;
    pos++;
    return true;
  };
  const expect = (symbol: string): void => {
    if (!accept(symbol)) syntaxError();
  };

  function entries(): Entry[] {
    const list: Entry[] = [];
    while (tokens[pos]?.kind === "name") {
      const { text: name, line, col } = tokens[pos++];
      expect("=");
      list.push({ name, at: { line, col }, value: value() });
    }
    return list;
  }

  function value(): Value {
    const token = tokens[pos];
    if (token?.kind === "string") {
      pos++;
      return { kind: "string", text: token.text };
    }
    if (accept("[")) {
      const items: Value[] = [];
      if (!accept("]")) {
        do items.push(value());
        while (accept(","));
        expect("]");
      }
      return { kind: "list", items };
    }
    if (accept("{")) {
      const inner = entries();
      expect("}");
      return { kind: "table", entries: inner };
    }
    return syntaxError();
  }

  const top = entries();
  if (pos < tokens.length) syntaxError();
  return top;
}

function checkKeys(entries: Entry[]): void {
  const seen = new Set<string>();
  for (const { name, at, value } of entries) {
    if (seen.has(name)) throw new Fail(`duplicate key ${name}`, at);
    seen.add(name);
    checkValue(value);
  }
}

function checkValue(value: Value): void {
  if (value.kind === "table") checkKeys(value.entries);
  if (value.kind === "list") value.items.forEach(checkValue);
}

function show(path: string, value: Value, out: string[]): void {
  switch (value.kind) {
    case "string":
      out.push(`${path} = "${value.text.replace(/[\\"]/g, "\\$&")}"`);
      break;
    case "list":
      if (value.items.length === 0) out.push(`${path} = []`);
      value.items.forEach((item, i) => show(`${path}[${i}]`, item, out));
      break;
    case "table":
      if (value.entries.length === 0) out.push(`${path} = {}`);
      showEntries(path, value.entries, out);
      break;
  }
}

function showEntries(path: string, entries: Entry[], out: string[]): void {
  const sorted = [...entries].sort((a, b) => (a.name < b.name ? -1 : 1));
  for (const { name, value } of sorted) {
    show(path === "" ? name : `${path}.${name}`, value, out);
  }
}

try {
  const lines = readFileSync(process.argv[2], "utf8").split("\n");
  const tokens: Token[] = [];
  lines.forEach((text, i) => scanLine(text, i + 1, tokens));
  const end = { line: lines.length, col: lines[lines.length - 1].length + 1 };
  const top = parse(tokens, end);
  checkKeys(top);
  const out: string[] = [];
  showEntries("", top, out);
  if (out.length > 0) console.log(out.join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: line ${error.at.line} col ${error.at.col}: ${error.message}`);
  process.exitCode = 20;
}
