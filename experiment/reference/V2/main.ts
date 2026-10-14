import { readFileSync } from "node:fs";

const EVENT = /^(in|out) ([A-Z0-9]{1,8}) ([0-9]{1,3}) ([0-9]{2}):([0-9]{2})$/;

interface Event {
  line: number;
  kind: string;
  plate: string;
  day: number;
  time: string;
  minute: number;
}

class Fail extends Error {}

function parseEvent(text: string, line: number): Event | undefined {
  const match = EVENT.exec(text);
  if (!match) return undefined;
  const [, kind, plate, dayText, hour, min] = match;
  const day = Number(dayText);
  if (day < 1 || day > 366 || Number(hour) > 23 || Number(min) > 59) return undefined;
  const minute = (day - 1) * 1440 + Number(hour) * 60 + Number(min);
  return { line, kind, plate, day, time: `${hour}:${min}`, minute };
}

function parse(text: string): Event[] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const events: Event[] = [];
  lines.forEach((text, i) => {
    if (text === "") return;
    const event = parseEvent(text, i + 1);
    if (!event) throw new Fail(`line ${i + 1}: bad event`);
    events.push(event);
  });
  events.forEach((event, i) => {
    if (i > 0 && event.minute < events[i - 1].minute) {
      throw new Fail(`line ${event.line}: time goes back`);
    }
  });
  return events;
}

function money(cents: number): string {
  return `${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}

function fee(minutes: number): number {
  if (minutes <= 15) return 0;
  const days = Math.floor(minutes / 1440);
  const hours = Math.ceil((minutes - days * 1440) / 60);
  const rates = [200, 150, 150];
  let charge = 0;
  for (let h = 0; h < hours; h++) charge += rates[h] ?? 100;
  return days * 1200 + Math.min(charge, 1200);
}

function run(events: Event[]): string[] {
  const inside = new Map<string, Event>();
  const out: string[] = [];
  let total = 0;
  for (const event of events) {
    const entry = inside.get(event.plate);
    if (event.kind === "in") {
      if (entry) out.push(`line ${event.line}: ${event.plate} is already in`);
      else inside.set(event.plate, event);
    } else if (!entry) {
      out.push(`line ${event.line}: ${event.plate} is not in`);
    } else {
      const minutes = event.minute - entry.minute;
      inside.delete(event.plate);
      total += fee(minutes);
      out.push(`${event.plate}: ${minutes} min, ${money(fee(minutes))}`);
    }
  }
  for (const plate of [...inside.keys()].sort()) {
    const { day, time } = inside.get(plate)!;
    out.push(`${plate}: still in since day ${day} ${time}`);
  }
  out.push(`total: ${money(total)}`);
  return out;
}

try {
  console.log(run(parse(readFileSync(process.argv[2], "utf8"))).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
