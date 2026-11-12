import { readFileSync } from "node:fs";

type State = { lines: string[]; current: number };
type Step = { before: State; after: State };

const WORDS = ["del", "up", "down", "undo", "redo", "print"];

class Editor {
  state: State = { lines: [], current: 0 };
  undo: Step[] = [];
  redo: Step[] = [];

  /** Runs one command; gives the printed text, or throws the reason it is rejected. */
  run(line: string, joins: boolean): string {
    const before = this.state;
    const s: State = { lines: [...before.lines], current: before.current };
    const [word, text] = line.startsWith("add ") || line.startsWith("set ")
      ? [line.slice(0, 3), line.slice(4)]
      : [line, ""];
    if (["set", "del", "up", "down"].includes(word) && s.lines.length === 0) throw "empty buffer";
    switch (word) {
      case "add":
        s.lines.splice(s.current, 0, text);
        s.current++;
        break;
      case "set":
        if (s.lines[s.current - 1] === text) return "";
        s.lines[s.current - 1] = text;
        break;
      case "del":
        s.lines.splice(s.current - 1, 1);
        s.current = Math.min(s.current, s.lines.length);
        break;
      case "up":
      case "down": {
        const edge = word === "up" ? 1 : s.lines.length;
        if (s.current === edge) throw word === "up" ? "at top" : "at bottom";
        s.current += word === "up" ? -1 : 1;
        this.state = s;
        return "";
      }
      case "undo":
      case "redo": {
        const [from, to] = word === "undo" ? [this.undo, this.redo] : [this.redo, this.undo];
        const step = from.pop();
        if (!step) throw `nothing to ${word}`;
        this.state = word === "undo" ? step.before : step.after;
        to.push(step);
        return "";
      }
      case "print":
        return this.print();
    }
    this.state = s;
    if (joins) {
      this.undo[this.undo.length - 1].after = s;
    } else {
      this.undo.push({ before, after: s });
      if (this.undo.length > 3) this.undo.shift();
      this.redo = [];
    }
    return "";
  }

  print(): string {
    const { lines, current } = this.state;
    const width = String(lines.length).length;
    const rows = lines.map((line, i) => {
      const mark = i + 1 === current ? ">" : " ";
      return `${mark}${String(i + 1).padStart(width)}|${line}\n`;
    });
    return `[${current}/${lines.length}]\n${rows.join("")}`;
  }
}

function main(): void {
  const lines = readFileSync(process.argv[2], "utf8").split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  const bad = lines.findIndex(
    (line) => !line.startsWith("add ") && !line.startsWith("set ") && !WORDS.includes(line),
  );
  if (bad >= 0) {
    console.log(`error: line ${bad + 1}: bad command`);
    process.exitCode = 20;
    return;
  }
  const editor = new Editor();
  let out = "";
  lines.forEach((line, i) => {
    const joins = i > 0 && line.startsWith("add ") && lines[i - 1].startsWith("add ");
    try {
      out += editor.run(line, joins);
    } catch (reason) {
      out += `line ${i + 1}: ${reason}\n`;
    }
  });
  process.stdout.write(`${out}undo ${editor.undo.length}, redo ${editor.redo.length}\n`);
}

main();
