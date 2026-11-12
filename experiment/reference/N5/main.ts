import { readFileSync } from "node:fs";

class Fail extends Error {}

// An exact score num / den, den > 0. All values stay far below 2^53.
interface Ratio {
  num: number;
  den: number;
}

interface Student {
  name: string;
  score: Ratio | undefined;
}

const NAME = /^[A-Za-z0-9_-]{1,20}$/;
const SCORE = /^([0-9]{1,3})(L?)$/;

const compare = (a: Ratio, b: Ratio) => a.num * b.den - b.num * a.den;
const byName = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);

// One decimal, halves up; the value is not negative.
function tenths(r: Ratio): string {
  const t = Math.floor((20 * r.num + r.den) / (2 * r.den));
  return `${Math.floor(t / 10)}.${t % 10}`;
}

function grade(score: Ratio | undefined): string {
  if (score === undefined) return "I";
  const found = ([[90, "A"], [80, "B"], [70, "C"], [60, "D"]] as const).find(
    ([least]) => score.num >= least * score.den,
  );
  return found === undefined ? "F" : found[1];
}

function parseParts(words: string[]): number[] {
  if (words[0] !== "parts") throw new Fail("expected parts");
  if (words.length === 1) throw new Fail("no parts");
  const names: string[] = [];
  const weights: number[] = [];
  for (const word of words.slice(1)) {
    const parts = word.split(":");
    if (parts.length !== 2 || !NAME.test(parts[0])) throw new Fail(`bad part ${word}`);
    const w = Number(parts[1]);
    if (!/^[0-9]{1,3}$/.test(parts[1]) || w < 1 || w > 100) throw new Fail(`bad weight ${word}`);
    if (names.includes(parts[0])) throw new Fail(`duplicate part ${parts[0]}`);
    names.push(parts[0]);
    weights.push(w);
  }
  const sum = weights.reduce((a, b) => a + b, 0);
  if (sum !== 100) throw new Fail(`weights sum to ${sum}`);
  return weights;
}

function parseStudent(words: string[], weights: number[], seen: Student[]): Student {
  const name = words[0];
  if (!NAME.test(name)) throw new Fail(`bad name ${name}`);
  if (seen.some((s) => s.name === name)) throw new Fail(`duplicate student ${name}`);
  const k = words.length - 1;
  if (k !== weights.length) throw new Fail(`expected ${weights.length} scores, got ${k}`);
  let t = 0;
  let s = 0;
  words.slice(1).forEach((word, i) => {
    if (word === "x") return;
    const m = SCORE.exec(word);
    if (m === null || Number(m[1]) > 100) throw new Fail(`bad score ${word}`);
    const value = Math.max(0, Number(m[1]) - (m[2] === "L" ? 10 : 0));
    t += weights[i] * value;
    s += weights[i];
  });
  return { name, score: s > 0 ? { num: t, den: s } : undefined };
}

function report(students: Student[]): string {
  students.sort((a, b) => {
    if (a.score && b.score) return compare(b.score, a.score) || byName(a.name, b.name);
    if (a.score || b.score) return a.score ? -1 : 1;
    return byName(a.name, b.name);
  });
  const scores = students.flatMap((s) => (s.score ? [s.score] : []));
  let out = "";
  for (const st of students) {
    const r = st.score;
    if (r === undefined) {
      out += `- ${st.name} - I\n`;
      continue;
    }
    const rank = 1 + scores.filter((o) => compare(o, r) > 0).length;
    out += `${rank} ${st.name} ${tenths(r)} ${grade(r)}\n`;
  }
  const n = scores.length;
  let median = "-";
  if (n % 2 === 1) median = tenths(scores[(n - 1) / 2]);
  else if (n > 0) {
    const [a, b] = [scores[n / 2 - 1], scores[n / 2]];
    median = tenths({ num: a.num * b.den + b.num * a.den, den: 2 * a.den * b.den });
  }
  out += `median ${median}\n`;
  const counts = ["A", "B", "C", "D", "F", "I"].map(
    (g) => `${g}=${students.filter((s) => grade(s.score) === g).length}`,
  );
  return out + `grades ${counts.join(" ")}\n`;
}

function run(text: string): string {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  let weights: number[] | undefined;
  const students: Student[] = [];
  lines.forEach((line, i) => {
    const words = line.split(" ").filter((w) => w !== "");
    if (words.length === 0) return;
    try {
      if (weights === undefined) weights = parseParts(words);
      else students.push(parseStudent(words, weights, students));
    } catch (error) {
      if (error instanceof Fail) throw new Fail(`line ${i + 1}: ${error.message}`);
      throw error;
    }
  });
  if (weights === undefined) throw new Fail("no parts");
  if (students.length === 0) throw new Fail("no students");
  return report(students);
}

try {
  process.stdout.write(run(readFileSync(process.argv[2], "utf8")));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
