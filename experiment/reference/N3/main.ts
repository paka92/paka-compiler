import { readFileSync } from "node:fs";

const DATE = "([0-9]{4}-[0-9]{2}-[0-9]{2})";
const SHAPES = new Map([
  ["days", new RegExp(`^days ${DATE} ${DATE}$`)],
  ["weekday", new RegExp(`^weekday ${DATE}$`)],
  ["add", new RegExp(`^add ${DATE} (-?[0-9]{1,3})$`)],
  ["holiday", new RegExp(`^holiday ${DATE}$`)],
]);
const WEEKDAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MS_PER_DAY = 86_400_000;
const EPOCH = Date.UTC(1900, 0, 1);

class Fail extends Error {}

interface Command {
  line: number;
  name: string;
  args: string[];
}

function isLeap(year: number): boolean {
  return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
}

function isValidDate(date: string): boolean {
  const [year, month, day] = date.split("-").map(Number);
  const length = [31, isLeap(year) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1];
  return year >= 1900 && year <= 2100 && month >= 1 && month <= 12 && day >= 1 && day <= length;
}

// Days since 1900-01-01 (a Monday).
function toDay(date: string): number {
  const [year, month, day] = date.split("-").map(Number);
  return (Date.UTC(year, month - 1, day) - EPOCH) / MS_PER_DAY;
}

function fromDay(day: number): string {
  return new Date(EPOCH + day * MS_PER_DAY).toISOString().slice(0, 10);
}

const weekdayOf = (day: number) => ((day % 7) + 7) % 7;

function parse(text: string): Command[] {
  const commands: Command[] = [];
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  lines.forEach((text, i) => {
    if (text === "") return;
    const line = i + 1;
    const name = text.split(" ")[0];
    const match = SHAPES.get(name)?.exec(text);
    if (!match) throw new Fail(`line ${line}: bad command`);
    const args = match.slice(1);
    const dates = name === "add" ? args.slice(0, 1) : args;
    if (!dates.every(isValidDate)) throw new Fail(`line ${line}: bad date`);
    commands.push({ line, name, args });
  });
  return commands;
}

function run(commands: Command[]): string[] {
  const holidays = new Set(
    commands.filter((c) => c.name === "holiday").map((c) => toDay(c.args[0])),
  );
  const isWorking = (day: number) => weekdayOf(day) < 5 && !holidays.has(day);
  const lastDay = toDay("2100-12-31");
  const out: string[] = [];
  for (const { line, name, args } of commands) {
    if (name === "days") {
      out.push(String(toDay(args[1]) - toDay(args[0])));
    } else if (name === "weekday") {
      out.push(WEEKDAYS[weekdayOf(toDay(args[0]))]);
    } else if (name === "add") {
      const count = Number(args[1]);
      const direction = Math.sign(count);
      let day = toDay(args[0]);
      for (let moved = 0; moved < Math.abs(count); moved++) {
        do {
          day += direction;
        } while (!isWorking(day));
      }
      out.push(day < 0 || day > lastDay ? `line ${line}: out of range` : fromDay(day));
    }
  }
  return out;
}

try {
  const out = run(parse(readFileSync(process.argv[2], "utf8")));
  if (out.length > 0) console.log(out.join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
