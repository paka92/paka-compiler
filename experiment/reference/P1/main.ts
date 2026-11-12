import { readFileSync } from "node:fs";

interface Token {
  text: string;
  col: number;
}

type Expr =
  | { kind: "num"; value: bigint }
  | { kind: "name"; name: string; col: number }
  | { kind: "neg"; operand: Expr }
  | { kind: "bin"; op: string; col: number; left: Expr; right: Expr };

interface Line {
  target?: string;
  expr: Expr;
}

class CalcError extends Error {
  constructor(
    readonly col: number,
    message: string,
  ) {
    super(message);
  }
}

const isName = (token: Token | undefined) => token !== undefined && /^[A-Za-z_]/.test(token.text);

function tokenize(line: string): Token[] {
  const bad = line.search(/[^ 0-9A-Za-z_+\-*/%()=]/);
  if (bad >= 0) throw new CalcError(bad + 1, "bad character");
  const pattern = /[0-9]+|[A-Za-z_][A-Za-z0-9_]*|[-+*/%()=]/g;
  return [...line.matchAll(pattern)].map((m) => ({ text: m[0], col: m.index! + 1 }));
}

function parse(tokens: Token[], endCol: number): Line {
  let pos = 0;
  const syntaxError = (): never => {
    throw new CalcError(tokens[pos]?.col ?? endCol, "syntax error");
  };
  const accept = (...symbols: string[]): Token | undefined => {
    const token = tokens[pos];
    if (token === undefined || !symbols.includes(token.text)) return undefined;
    pos++;
    return token;
  };

  const leftAssociative = (operand: () => Expr, ...ops: string[]) => (): Expr => {
    let left = operand();
    for (let op = accept(...ops); op; op = accept(...ops)) {
      left = { kind: "bin", op: op.text, col: op.col, left, right: operand() };
    }
    return left;
  };

  function primary(): Expr {
    const token = tokens[pos];
    if (accept("(")) {
      const inner = expression();
      if (!accept(")")) syntaxError();
      return inner;
    }
    if (token === undefined || !/^[0-9A-Za-z_]/.test(token.text)) {
      return syntaxError();
    }
    pos++;
    return isName(token)
      ? { kind: "name", name: token.text, col: token.col }
      : { kind: "num", value: BigInt(token.text) };
  }

  function unary(): Expr {
    return accept("-") ? { kind: "neg", operand: unary() } : primary();
  }

  const term = leftAssociative(unary, "*", "/", "%");
  const expression = leftAssociative(term, "+", "-");

  let target: string | undefined;
  if (isName(tokens[0]) && tokens[1]?.text === "=") {
    target = tokens[0].text;
    pos = 2;
  }
  const expr = expression();
  if (pos < tokens.length) syntaxError();
  return { target, expr };
}

function evaluate(expr: Expr, variables: Map<string, bigint>): bigint {
  switch (expr.kind) {
    case "num":
      return expr.value;
    case "name": {
      const value = variables.get(expr.name);
      if (value === undefined) throw new CalcError(expr.col, `unknown name ${expr.name}`);
      return value;
    }
    case "neg":
      return -evaluate(expr.operand, variables);
    case "bin": {
      const left = evaluate(expr.left, variables);
      const right = evaluate(expr.right, variables);
      if (expr.op === "+") return left + right;
      if (expr.op === "-") return left - right;
      if (expr.op === "*") return left * right;
      if (right === 0n) throw new CalcError(expr.col, "division by zero");
      return expr.op === "/" ? left / right : left % right;
    }
  }
}

function main(): void {
  const lines = readFileSync(process.argv[2], "utf8").split("\n");
  const variables = new Map<string, bigint>();
  const out: string[] = [];
  for (const [i, text] of lines.entries()) {
    if (text.trim() === "") continue;
    try {
      const { target, expr } = parse(tokenize(text), text.length + 1);
      const value = evaluate(expr, variables);
      if (target === undefined) out.push(String(value));
      else variables.set(target, value);
    } catch (error) {
      if (!(error instanceof CalcError)) throw error;
      out.push(`error: line ${i + 1} col ${error.col}: ${error.message}`);
      process.exitCode = 20;
      break;
    }
  }
  if (out.length > 0) console.log(out.join("\n"));
}

main();
