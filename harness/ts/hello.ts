import { readFileSync } from "node:fs";

const name: string = readFileSync(0, "utf8").trim();
process.stdout.write(`hello, ${name}\n`);
