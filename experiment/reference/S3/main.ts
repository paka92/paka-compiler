import { readFileSync } from "node:fs";

interface Pos {
  line: number;
  col: number;
}

class Fail extends Error {
  constructor(
    message: string,
    readonly at: Pos,
  ) {
    super(message);
  }
}

// Step 1: removes every backslash-line feed pair; `at[i]` is where `text[i]` stood in the file.
function join(source: string): { text: string; at: Pos[] } {
  let text = "";
  const at: Pos[] = [];
  let line = 1;
  let col = 1;
  for (let i = 0; i < source.length; i++) {
    const c = source[i];
    if (c === "\\" && source[i + 1] === "\n") {
      i++;
      line++;
      col = 1;
      continue;
    }
    text += c;
    at.push({ line, col });
    if (c === "\n") {
      line++;
      col = 1;
    } else {
      col++;
    }
  }
  return { text, at };
}

// Step 2: returns the text without comments.
function strip(text: string, at: Pos[]): string {
  let out = "";
  let i = 0;
  while (i < text.length) {
    const pair = text.slice(i, i + 2);
    if (pair === "//") {
      while (i < text.length && text[i] !== "\n") i++;
    } else if (pair === "/*") {
      const end = commentEnd(text, i, at);
      out += text.slice(i, end).replace(/[^\n]/g, "");
      i = end;
    } else if (text[i] === '"' || text[i] === "'") {
      const end = stringEnd(text, i, at);
      out += text.slice(i, end);
      i = end;
    } else {
      out += text[i++];
    }
  }
  return out;
}

// Returns the index after the block comment that opens at `start`.
function commentEnd(text: string, start: number, at: Pos[]): number {
  let depth = 1;
  let i = start + 2;
  while (depth > 0) {
    if (i >= text.length) throw new Fail("unterminated comment", at[start]);
    const pair = text.slice(i, i + 2);
    if (pair === "/*") depth++;
    if (pair === "*/") depth--;
    i += pair === "/*" || pair === "*/" ? 2 : 1;
  }
  return i;
}

// Returns the index after the string that opens at `start`.
function stringEnd(text: string, start: number, at: Pos[]): number {
  let i = start + 1;
  for (;;) {
    const c = text[i];
    const next = text[i + 1];
    const broken = c === undefined || c === "\n" || (c === "\\" && (next === undefined || next === "\n"));
    if (broken) throw new Fail("unterminated string", at[start]);
    if (c === text[start]) return i + 1;
    i += c === "\\" ? 2 : 1;
  }
}

try {
  const { text, at } = join(readFileSync(process.argv[2], "utf8"));
  const lines = strip(text, at).split("\n");
  if (text === "" || text.endsWith("\n")) lines.pop();
  for (const line of lines) console.log(line.replace(/ +$/, ""));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: line ${error.at.line} col ${error.at.col}: ${error.message}`);
  process.exitCode = 20;
}
