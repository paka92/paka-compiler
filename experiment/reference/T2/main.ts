import { readFileSync } from "node:fs";

const NUMBER = /^-?[0-9]{1,12}$/;

class Fail extends Error {}

function splitFields(line: string, lineNumber: number): string[] {
  const badQuote = new Fail(`line ${lineNumber}: bad quote`);
  const fields: string[] = [];
  let i = 0;
  for (;;) {
    if (line[i] === '"') {
      let value = "";
      for (i++; ; i++) {
        if (i >= line.length) throw badQuote;
        if (line[i] !== '"') {
          value += line[i];
        } else if (line[i + 1] === '"') {
          value += '"';
          i++;
        } else {
          break;
        }
      }
      i++;
      fields.push(value);
      if (i < line.length && line[i] !== ",") throw badQuote;
    } else {
      const comma = line.indexOf(",", i);
      const value = line.slice(i, comma < 0 ? line.length : comma);
      if (value.includes('"')) throw badQuote;
      fields.push(value);
      i = comma < 0 ? line.length : comma;
    }
    if (i >= line.length) return fields;
    i++;
  }
}

function parse(text: string): string[][] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  if (lines.length === 0) throw new Fail("empty input");
  const rows = lines.map((line, i) => splitFields(line, i + 1));
  rows.forEach((row, i) => {
    if (row.length !== rows[0].length) {
      throw new Fail(`line ${i + 1}: expected ${rows[0].length} fields, got ${row.length}`);
    }
  });
  return rows;
}

function table(rows: string[][]): string[] {
  const [header, ...data] = rows;
  const isNumber = header.map(
    (_, c) => data.length > 0 && data.every((row) => NUMBER.test(row[c])),
  );
  const total = header.map((_, c) => {
    if (isNumber[c]) return String(data.reduce((sum, row) => sum + BigInt(row[c]), 0n));
    return c === 0 ? "total" : "";
  });
  const body = isNumber.includes(true) ? [...data, total] : data;
  const widths = header.map((h, c) => Math.max(1, h.length, ...body.map((row) => row[c].length)));

  const line = (row: string[]) =>
    row
      .map((v, c) => (isNumber[c] ? v.padStart(widths[c]) : v.padEnd(widths[c])))
      .join(" | ")
      .replace(/ +$/, "");
  const rule = widths.map((w) => "-".repeat(w)).join("-+-");

  const out = [line(header), rule, ...data.map(line)];
  if (body !== data) out.push(rule, line(total));
  return out;
}

try {
  console.log(table(parse(readFileSync(process.argv[2], "utf8"))).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
