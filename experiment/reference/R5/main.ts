import { readFileSync } from "node:fs";

class Fail extends Error {}

function fail(message: string): never {
  throw new Fail(message);
}

/** Best cost and number of roads to the end; null when there is no route. */
type Dist = [cost: number, roads: number] | null;
type Edge = [place: number, cost: number];

const isName = (s: string) => /^[A-Za-z0-9_-]{1,20}$/.test(s);
const ascii = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
const less = (a: Dist, b: Dist) => !!a && (!b || a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]));

class Net {
  names: string[];
  out: Edge[][];
  into: Edge[][];

  constructor(roads: [string, string, number][]) {
    this.names = [...new Set(roads.flatMap(([a, b]) => [a, b]))].sort(ascii);
    this.out = this.names.map(() => []);
    this.into = this.names.map(() => []);
    for (const [a, b, cost] of roads) {
      this.out[this.place(a)].push([this.place(b), cost]);
      this.into[this.place(b)].push([this.place(a), cost]);
    }
  }

  place(name: string): number {
    return this.names.indexOf(name);
  }

  /** Dijkstra from the end over the roads reversed. */
  toEnd(end: number): Dist[] {
    const dist: Dist[] = this.names.map(() => null);
    const done = this.names.map(() => false);
    dist[end] = [0, 0];
    for (;;) {
      let u = -1;
      dist.forEach((d, v) => {
        if (!done[v] && d && (u < 0 || less(d, dist[u]))) u = v;
      });
      if (u < 0) return dist;
      done[u] = true;
      const [cost, roads] = dist[u]!;
      for (const [v, w] of this.into[u]) {
        const d: Dist = [cost + w, roads + 1];
        if (!done[v] && less(d, dist[v])) dist[v] = d;
      }
    }
  }

  trip(from: string, to: string): string {
    const [start, end] = [this.place(from), this.place(to)];
    if (start < 0) return `unknown ${from}`;
    if (end < 0) return `unknown ${to}`;
    const dist = this.toEnd(end);
    const best = dist[start];
    if (!best) return "no route";
    const path = [from];
    for (let u = start; u !== end; ) {
      const [cost, roads] = dist[u]!;
      const next = this.out[u]
        .filter(([v, w]) => dist[v] && dist[v]![0] + w === cost && dist[v]![1] + 1 === roads)
        .map(([v]) => v);
      u = Math.min(...next);
      path.push(this.names[u]);
    }
    const word = best[1] === 1 ? "road" : "roads";
    return `cost ${best[0]}, ${best[1]} ${word}: ${path.join(" ")}`;
  }
}

function run(text: string): string[] {
  const roads: [string, string, number][] = [];
  const trips: [string, string][] = [];
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  lines.forEach((line, i) => {
    const w = line.split(" ").filter((word) => word !== "");
    if (w.length === 0) return;
    if (w.length === 3 && w[0] === "trip" && isName(w[1]) && isName(w[2])) {
      trips.push([w[1], w[2]]);
    } else if (
      w.length === 4 &&
      (w[0] === "one" || w[0] === "two") &&
      isName(w[1]) &&
      isName(w[2]) &&
      /^[0-9]{1,6}$/.test(w[3])
    ) {
      if (w[1] === w[2]) fail(`line ${i + 1}: loop road`);
      roads.push([w[1], w[2], Number(w[3])]);
      if (w[0] === "two") roads.push([w[2], w[1], Number(w[3])]);
    } else {
      fail(`line ${i + 1}: bad line`);
    }
  });
  if (trips.length === 0) fail("no trips");
  const net = new Net(roads);
  return trips.map(([a, b]) => `${a} to ${b}: ${net.trip(a, b)}`);
}

try {
  console.log(run(readFileSync(process.argv[2], "utf8")).join("\n"));
} catch (error) {
  if (!(error instanceof Fail)) throw error;
  console.log(`error: ${error.message}`);
  process.exitCode = 20;
}
