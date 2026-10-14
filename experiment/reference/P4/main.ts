import { readFileSync } from "node:fs";

type Op = { line: number; name: string; args: string[] };

const KINDS: Record<string, string> = {
  set: "RV", add: "RV", sub: "RV", mul: "RV", div: "RV", mod: "RV",
  jmp: "L", call: "L", jz: "VL", jlt: "VVL", out: "V", ret: "", halt: "",
};
const REGISTER = /^[a-f]$/;
const LABEL = /^[a-z][a-z0-9_]*$/;
const NUMBER = /^-?[0-9]{1,7}$/;
const MAX = 9999999;

function fail(message: string): never {
  console.log(message);
  process.exit(20);
}

function fits(kind: string, w: string): boolean {
  if (kind === "R") return REGISTER.test(w);
  if (kind === "V") return REGISTER.test(w) || NUMBER.test(w);
  return LABEL.test(w);
}

function assemble(text: string): [Op[], Map<string, number>] {
  const ops: Op[] = [];
  const labels = new Map<string, number>();
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  lines.forEach((raw, i) => {
    const line = i + 1;
    const words = raw.split(";")[0].split(" ").filter((w) => w !== "");
    if (words.length > 0 && words[0].endsWith(":")) {
      const name = words.shift()!.slice(0, -1);
      if (!LABEL.test(name)) fail(`error: line ${line}: bad label`);
      if (labels.has(name)) fail(`error: line ${line}: duplicate label`);
      labels.set(name, ops.length);
    }
    if (words.length === 0) return;
    const [name, ...args] = words;
    const kinds = Object.hasOwn(KINDS, name) ? KINDS[name] : undefined;
    if (kinds === undefined) fail(`error: line ${line}: unknown instruction`);
    if (kinds.length !== args.length) fail(`error: line ${line}: wrong count`);
    if (!args.every((w, k) => fits(kinds[k], w))) fail(`error: line ${line}: bad operand`);
    ops.push({ line, name, args });
  });
  for (const op of ops) {
    op.args.forEach((w, k) => {
      if (KINDS[op.name][k] === "L" && !labels.has(w)) fail(`error: line ${op.line}: unknown label ${w}`);
    });
  }
  return [ops, labels];
}

function main(): void {
  const [file, limitText] = process.argv.slice(2);
  if (!/^[1-9][0-9]{0,5}$/.test(limitText) || Number(limitText) > 100000) fail("error: bad limit");
  const limit = Number(limitText);
  const [ops, labels] = assemble(readFileSync(file, "utf8"));
  const regs = [0, 0, 0, 0, 0, 0];
  const stack: number[] = [];
  const value = (w: string) => (REGISTER.test(w) ? regs[w.charCodeAt(0) - 97] : Number(w));
  let pc = 0;
  for (let steps = 0; pc < ops.length; steps++) {
    const { line, name, args: a } = ops[pc];
    if (steps === limit) fail(`error: line ${line}: step limit`);
    let next = pc + 1;
    if (name === "jmp") next = labels.get(a[0])!;
    else if (name === "jz") next = value(a[0]) === 0 ? labels.get(a[1])! : next;
    else if (name === "jlt") next = value(a[0]) < value(a[1]) ? labels.get(a[2])! : next;
    else if (name === "call") {
      if (stack.length === 16) fail(`error: line ${line}: stack overflow`);
      stack.push(next);
      next = labels.get(a[0])!;
    } else if (name === "ret") {
      if (stack.length === 0) fail(`error: line ${line}: ret without call`);
      next = stack.pop()!;
    } else if (name === "out") console.log(String(value(a[0])));
    else if (name === "halt") next = ops.length;
    else {
      const [x, v] = [value(a[0]), value(a[1])];
      if ((name === "div" || name === "mod") && v === 0) fail(`error: line ${line}: division by zero`);
      const results: Record<string, number> = { set: v, add: x + v, sub: x - v, mul: x * v, div: Math.trunc(x / v), mod: x % v };
      const r = results[name] + 0;
      if (Math.abs(r) > MAX) fail(`error: line ${line}: overflow`);
      regs[a[0].charCodeAt(0) - 97] = r;
    }
    pc = next;
  }
}

main();
