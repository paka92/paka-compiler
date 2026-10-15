import { readFileSync } from "node:fs";

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

const COINS = [5, 10, 20, 50, 100, 200];

type Command =
  | { kind: "load"; slot: string; price: number; count: number }
  | { kind: "coin"; value: number }
  | { kind: "select"; slot: string }
  | { kind: "cancel" };

const money = (cents: number) =>
  `${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
const moneyList = (coins: number[]) => coins.map(money).join(" ");

function parseBox(line: string | undefined): number[] {
  const m = /^box((?: [0-9]{1,3}){6})$/.exec(line ?? "") ?? fail("bad box");
  return m[1].trim().split(" ").map(Number);
}

function parseCommand(line: string): Command | null {
  let m = /^load ([A-Z][1-9]) ([0-9]{1,2})\.([0-9]{2}) ([0-9]{1,2})$/.exec(line);
  if (m) {
    const price = Number(m[2]) * 100 + Number(m[3]);
    const count = Number(m[4]);
    if (price < 5 || price % 5 !== 0 || count < 1) return null;
    return { kind: "load", slot: m[1], price, count };
  }
  if ((m = /^coin ([0-9]{1,3})$/.exec(line))) return { kind: "coin", value: Number(m[1]) };
  if ((m = /^select ([A-Z][1-9])$/.exec(line))) return { kind: "select", slot: m[1] };
  return line === "cancel" ? { kind: "cancel" } : null;
}

class Machine {
  credit = 0;
  session: number[] = [];
  slots = new Map<string, { price: number; items: number }>();

  constructor(public coinBox: number[]) {}

  takeOut(coins: number[]): void {
    coins.forEach((coin) => this.coinBox[COINS.indexOf(coin)]--);
  }

  /** The coins paid for `change`, largest first, if the greedy rule can pay it. */
  changeCoins(change: number): number[] | null {
    const paid: number[] = [];
    for (let i = COINS.length - 1; i >= 0; i--) {
      const n = Math.min(Math.floor(change / COINS[i]), this.coinBox[i]);
      for (let k = 0; k < n; k++) paid.push(COINS[i]);
      change -= n * COINS[i];
    }
    return change === 0 ? paid : null;
  }

  run(c: Command): string | null {
    switch (c.kind) {
      case "load": {
        if (this.credit !== 0) return "busy";
        const items = this.slots.get(c.slot)?.items ?? 0;
        if (items + c.count > 20) return `slot full ${c.slot}`;
        this.slots.set(c.slot, { price: c.price, items: items + c.count });
        return null;
      }
      case "coin": {
        const i = COINS.indexOf(c.value);
        if (i < 0 || this.credit + c.value > 500) return `rejected ${money(c.value)}`;
        this.coinBox[i]++;
        this.credit += c.value;
        this.session.push(c.value);
        return null;
      }
      case "select": {
        const slot = this.slots.get(c.slot);
        if (!slot) return `no slot ${c.slot}`;
        if (slot.items === 0) return `sold out ${c.slot}`;
        if (this.credit < slot.price) return `insert ${money(slot.price - this.credit)} more`;
        const change = this.credit - slot.price;
        const paid = this.changeCoins(change);
        if (!paid) return "no change";
        slot.items--;
        this.takeOut(paid);
        this.credit = 0;
        this.session = [];
        return change === 0
          ? `sold ${c.slot}`
          : `sold ${c.slot}, change ${money(change)}: ${moneyList(paid)}`;
      }
      case "cancel": {
        if (this.credit === 0) return "nothing to return";
        const line = `returned ${money(this.credit)}: ${moneyList(this.session)}`;
        this.takeOut(this.session);
        this.credit = 0;
        this.session = [];
        return line;
      }
    }
  }
}

function run(text: string): string[] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const machine = new Machine(parseBox(lines[0]));
  const commands = lines
    .slice(1)
    .map((line, i) => parseCommand(line) ?? fail(`line ${i + 2}: bad command`));
  const out: string[] = [];
  for (const c of commands) {
    const line = machine.run(c);
    if (line !== null) out.push(line);
  }
  out.push(`credit ${money(machine.credit)}`, `box ${machine.coinBox.join(" ")}`);
  const names = [...machine.slots.keys()].sort();
  for (const name of names) {
    const { items, price } = machine.slots.get(name)!;
    out.push(`${name}: ${items} at ${money(price)}`);
  }
  return out;
}

try {
  console.log(run(readFileSync(process.argv[2], "utf8")).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
