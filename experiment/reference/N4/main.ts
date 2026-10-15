import { readFileSync } from "node:fs";

class Fail extends Error {}

interface Member {
  name: string;
  weight: number;
  total: number;
}

interface Cost {
  name: string;
  amount: number;
  sharers: number[]; // positions in member order
}

const NAME = /^[A-Za-z0-9_-]{1,20}$/;
const AMOUNT = /^-?[0-9]{1,8}\.[0-9]{2}$/;

function parseMembers(words: string[]): Member[] {
  if (words[0] !== "members") throw new Fail("expected members");
  if (words.length === 1) throw new Fail("no members");
  const members: Member[] = [];
  for (const word of words.slice(1)) {
    const parts = word.split(":");
    if (parts.length !== 2 || !NAME.test(parts[0])) throw new Fail(`bad member ${word}`);
    const weight = Number(parts[1]);
    if (!/^[0-9]{1,2}$/.test(parts[1]) || weight < 1) throw new Fail(`bad weight ${word}`);
    if (members.some((m) => m.name === parts[0])) {
      throw new Fail(`duplicate member ${parts[0]}`);
    }
    members.push({ name: parts[0], weight, total: 0 });
  }
  return members;
}

function parseCost(words: string[], members: Member[]): Cost {
  if (words[0] !== "cost") throw new Fail(`unknown command ${words[0]}`);
  if (words.length < 3) throw new Fail("incomplete cost");
  if (!NAME.test(words[1])) throw new Fail("bad cost name");
  if (!AMOUNT.test(words[2])) throw new Fail("bad amount");
  const [units, cents] = words[2].replace("-", "").split(".");
  const size = Number(units) * 100 + Number(cents);
  const amount = words[2].startsWith("-") ? -size : size;
  const listed = new Set<number>();
  for (const word of words.slice(3)) {
    const at = members.findIndex((m) => m.name === word);
    if (at < 0) throw new Fail(`unknown member ${word}`);
    if (listed.has(at)) throw new Fail(`duplicate member ${word}`);
    listed.add(at);
  }
  const sharers = members.map((_, i) => i).filter((i) => words.length === 3 || listed.has(i));
  return { name: words[1], amount, sharers };
}

function split(cost: Cost, members: Member[]): number[] {
  const size = Math.abs(cost.amount);
  const totalWeight = cost.sharers.reduce((sum, i) => sum + members[i].weight, 0);
  const products = cost.sharers.map((i) => size * members[i].weight);
  const shares = products.map((p) => Math.floor(p / totalWeight));
  const order = shares.map((_, k) => k);
  order.sort((a, b) => (products[b] % totalWeight) - (products[a] % totalWeight) || a - b);
  const left = size - shares.reduce((sum, s) => sum + s, 0);
  for (const k of order.slice(0, left)) shares[k] += 1;
  return shares.map((s) => (cost.amount < 0 ? -s : s));
}

function money(c: number): string {
  const units = String(Math.floor(Math.abs(c) / 100)).replace(/\B(?=([0-9]{3})+$)/g, ",");
  const cents = String(Math.abs(c) % 100).padStart(2, "0");
  return `${c < 0 ? "-" : ""}${units}.${cents}`;
}

// total / grand as a percent with one decimal, halves to even.
function percent(total: number, grand: number): string {
  if (grand === 0) return "-";
  const n = total * 1000 * Math.sign(grand);
  const d = Math.abs(grand);
  const q = Math.floor(Math.abs(n) / d);
  const r = Math.abs(n) % d;
  const tenths = 2 * r > d || (2 * r === d && q % 2 === 1) ? q + 1 : q;
  const sign = n < 0 && tenths > 0 ? "-" : "";
  return `${sign}${Math.floor(tenths / 10)}.${tenths % 10}%`;
}

function run(text: string): string {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  let members: Member[] | undefined;
  const costs: Cost[] = [];
  lines.forEach((line, i) => {
    const words = line.split(" ").filter((w) => w !== "");
    if (words.length === 0) return;
    try {
      if (members === undefined) members = parseMembers(words);
      else costs.push(parseCost(words, members));
    } catch (error) {
      if (error instanceof Fail) throw new Fail(`line ${i + 1}: ${error.message}`);
      throw error;
    }
  });
  if (members === undefined) throw new Fail("no members");
  if (costs.length === 0) throw new Fail("no costs");
  const list = members;
  let out = "";
  for (const cost of costs) {
    const shares = split(cost, list);
    const pairs = cost.sharers.map((i, k) => `${list[i].name} ${money(shares[k])}`);
    out += `${cost.name}: ${pairs.join(", ")}\n`;
    cost.sharers.forEach((i, k) => (list[i].total += shares[k]));
  }
  const grand = costs.reduce((sum, c) => sum + c.amount, 0);
  const sorted = [...list].sort(
    (a, b) => b.total - a.total || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0),
  );
  for (const m of sorted) out += `${m.name} ${money(m.total)} ${percent(m.total, grand)}\n`;
  return out + `total ${money(grand)}\n`;
}

try {
  process.stdout.write(run(readFileSync(process.argv[2], "utf8")));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
