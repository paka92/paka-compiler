import { readFileSync } from "node:fs";

const [file, widthArg] = process.argv.slice(2);

// Words of each paragraph, across its lines.
function paragraphs(text: string): string[][] {
  const result: string[][] = [];
  let current: string[] | undefined;
  for (const line of text.split("\n")) {
    const words = line.split(" ").filter((word) => word !== "");
    if (words.length === 0) {
      current = undefined;
    } else if (current) {
      current.push(...words);
    } else {
      current = [...words];
      result.push(current);
    }
  }
  return result;
}

function cut(word: string, width: number): string[] {
  const pieces: string[] = [];
  for (let i = 0; i < word.length; i += width) pieces.push(word.slice(i, i + width));
  return pieces;
}

function fill(words: string[], width: number): string[][] {
  const lines: string[][] = [];
  let length = 0;
  for (const word of words) {
    const line = lines[lines.length - 1];
    if (line && length + 1 + word.length <= width) {
      line.push(word);
      length += 1 + word.length;
    } else {
      lines.push([word]);
      length = word.length;
    }
  }
  return lines;
}

function justify(words: string[], width: number): string {
  if (words.length === 1) return words[0];
  const gaps = words.length - 1;
  const spaces = width - words.join("").length;
  return words.reduce((line, word, i) => {
    const gap = Math.floor(spaces / gaps) + (i <= spaces % gaps ? 1 : 0);
    return line + " ".repeat(gap) + word;
  });
}

function wrap(text: string, width: number): string {
  return paragraphs(text)
    .map((words) => {
      const lines = fill(words.flatMap((word) => cut(word, width)), width);
      const last = lines.length - 1;
      return lines.map((line, i) => (i < last ? justify(line, width) : line.join(" "))).join("\n");
    })
    .join("\n\n");
}

if (/^[0-9]+$/.test(widthArg) && Number(widthArg) >= 1 && Number(widthArg) <= 100) {
  const text = wrap(readFileSync(file, "utf8"), Number(widthArg));
  if (text !== "") console.log(text);
} else {
  console.log("error: bad width");
  process.exitCode = 20;
}
