import { readFileSync } from "node:fs";

class Fail extends Error {}

const FIELDS: [string, RegExp][] = [
  ["name", /^[A-Za-z0-9_-]{1,20}$/],
  ["quantity", /^-?[0-9]{1,3}$/],
  ["price", /^[0-9]{1,5}\.[0-9]{2}$/],
  ["discount", /^(?:[0-9]{1,2}|0[0-9]{2}|100)%$/],
  ["tax", /^[0-9]{1,2}\.[0-9]{2}%$/],
];

const abs = (n: bigint) => (n < 0n ? -n : n);

// Divides rounding to the nearest integer, halves away from zero. The divisor is positive.
function roundDiv(n: bigint, divisor: bigint): bigint {
  const q = (2n * abs(n) + divisor) / (2n * divisor);
  return n < 0n ? -q : q;
}

function cents(n: bigint): string {
  const rest = String(abs(n) % 100n).padStart(2, "0");
  return `${n < 0n ? "-" : ""}${abs(n) / 100n}.${rest}`;
}

interface Item {
  name: string;
  gross: bigint;
  discount: bigint;
  net: bigint;
  tax: bigint;
  total: bigint;
}

function parseItem(line: string, n: number): Item {
  const words = line.split(" ").filter((word) => word !== "");
  if (words.length !== 5) {
    throw new Fail(`line ${n}: expected 5 fields, got ${words.length}`);
  }
  const bad = FIELDS.find(([, pattern], k) => !pattern.test(words[k]));
  if (bad) throw new Fail(`line ${n}: bad ${bad[0]}`);

  const [name, quantity, price, discountRate, taxRate] = words;
  const gross = BigInt(quantity) * BigInt(price.replace(".", ""));
  const discount = roundDiv(gross * BigInt(discountRate.slice(0, -1)), 100n);
  const net = gross - discount;
  const tax = roundDiv(net * BigInt(taxRate.slice(0, -1).replace(".", "")), 10000n);
  return { name, gross, discount, net, tax, total: net + tax };
}

function main(): void {
  const lines = readFileSync(process.argv[2], "utf8").split("\n");
  const items = lines
    .map((line, i) => ({ line, n: i + 1 }))
    .filter(({ line }) => line.trim() !== "")
    .map(({ line, n }) => parseItem(line, n));
  if (items.length === 0) throw new Fail("no items");

  const sum = (field: "net" | "tax" | "total") =>
    items.reduce((acc, item) => acc + item[field], 0n);
  const total = sum("total");
  const out = items.map(
    (i) =>
      `${i.name}: gross ${cents(i.gross)} discount ${cents(i.discount)} net ${cents(i.net)} ` +
      `tax ${cents(i.tax)} total ${cents(i.total)}`,
  );
  out.push(`net ${cents(sum("net"))}`, `tax ${cents(sum("tax"))}`);
  out.push(total >= 0n ? `total ${cents(total)}` : `refund ${cents(-total)}`);
  console.log(out.join("\n"));
}

try {
  main();
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
