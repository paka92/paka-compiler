import { readFileSync } from "node:fs";

const NAME = "([a-z]+)";
const ID = "([a-z0-9]+)";
// At least 0.01: the lookahead rejects an all-zero amount.
const AMOUNT = "((?!0+\\.00(?: |$))[0-9]{1,9}\\.[0-9]{2})";
const FIELDS: Record<string, string[]> = {
  open: [NAME],
  deposit: [NAME, AMOUNT],
  hold: [NAME, ID, AMOUNT],
  capture: [ID, AMOUNT],
  release: [ID],
  transfer: [NAME, NAME, AMOUNT],
};
const COMMANDS = new Map(
  Object.entries(FIELDS).map(([word, fields]) => [word, new RegExp(`^${[word, ...fields].join(" ")}$`)]),
);

interface Command {
  line: number;
  word: string;
  args: string[];
}

interface Hold {
  account: string;
  cents: number;
}

class Fail extends Error {}

function toCents(amount: string): number {
  return Number(amount.replace(".", ""));
}

function money(cents: number): string {
  return `${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}

function parse(text: string): Command[] {
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const commands: Command[] = [];
  lines.forEach((text, i) => {
    if (text === "") return;
    const word = text.split(" ")[0];
    const match = COMMANDS.get(word)?.exec(text);
    if (!match) throw new Fail(`line ${i + 1}: bad command`);
    commands.push({ line: i + 1, word, args: match.slice(1) });
  });
  return commands;
}

function run(commands: Command[]): string[] {
  const balances = new Map<string, number>();
  const holds = new Map<string, Hold>();
  const usedIds = new Set<string>();
  const out: string[] = [];

  const held = (account: string) =>
    [...holds.values()].filter((h) => h.account === account).reduce((sum, h) => sum + h.cents, 0);
  const available = (account: string) => balances.get(account)! - held(account);

  // Returns the reason a command is rejected, or undefined after running it.
  function execute({ word, args }: Command): string | undefined {
    if (word === "open") {
      if (balances.has(args[0])) return `account exists ${args[0]}`;
      balances.set(args[0], 0);
    } else if (word === "deposit") {
      if (!balances.has(args[0])) return `no account ${args[0]}`;
      balances.set(args[0], balances.get(args[0])! + toCents(args[1]));
    } else if (word === "hold") {
      const [account, id, amount] = args;
      if (!balances.has(account)) return `no account ${account}`;
      if (usedIds.has(id)) return `hold exists ${id}`;
      if (available(account) < toCents(amount)) return "insufficient funds";
      usedIds.add(id);
      holds.set(id, { account, cents: toCents(amount) });
    } else if (word === "capture") {
      const hold = holds.get(args[0]);
      if (!hold) return `no hold ${args[0]}`;
      if (toCents(args[1]) > hold.cents) return "capture exceeds hold";
      balances.set(hold.account, balances.get(hold.account)! - toCents(args[1]));
      holds.delete(args[0]);
    } else if (word === "release") {
      if (!holds.delete(args[0])) return `no hold ${args[0]}`;
    } else {
      const [from, to, amount] = args;
      if (!balances.has(from)) return `no account ${from}`;
      if (!balances.has(to)) return `no account ${to}`;
      if (from === to) return "same account";
      if (available(from) < toCents(amount)) return "insufficient funds";
      balances.set(from, balances.get(from)! - toCents(amount));
      balances.set(to, balances.get(to)! + toCents(amount));
    }
    return undefined;
  }

  for (const command of commands) {
    const reason = execute(command);
    if (reason) out.push(`line ${command.line}: ${reason}`);
  }
  const names = [...balances.keys()].sort();
  if (names.length === 0) out.push("no accounts");
  for (const name of names) {
    out.push(`${name}: balance ${money(balances.get(name)!)}, held ${money(held(name))}`);
  }
  return out;
}

try {
  console.log(run(parse(readFileSync(process.argv[2], "utf8"))).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
