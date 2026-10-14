use std::{env, fs, process};

#[derive(Clone, Copy)]
enum Command<'a> {
    Add(&'a str),
    Set(&'a str),
    Del,
    Up,
    Down,
    Undo,
    Redo,
    Print,
}

fn parse(line: &str) -> Option<Command<'_>> {
    if let Some(text) = line.strip_prefix("add ") {
        return Some(Command::Add(text));
    }
    if let Some(text) = line.strip_prefix("set ") {
        return Some(Command::Set(text));
    }
    Some(match line {
        "del" => Command::Del,
        "up" => Command::Up,
        "down" => Command::Down,
        "undo" => Command::Undo,
        "redo" => Command::Redo,
        "print" => Command::Print,
        _ => return None,
    })
}

#[derive(Clone, Default)]
struct State {
    lines: Vec<String>,
    current: usize,
}

struct Step {
    before: State,
    after: State,
}

#[derive(Default)]
struct Editor {
    state: State,
    undo: Vec<Step>,
    redo: Vec<Step>,
}

impl Editor {
    /// Runs one command; gives the rejection reason or the printed text.
    fn run(&mut self, command: Command, after_add: bool) -> Result<String, &'static str> {
        let before = self.state.clone();
        let s = &mut self.state;
        let empty = s.lines.is_empty();
        match command {
            Command::Add(text) => {
                s.lines.insert(s.current, text.to_string());
                s.current += 1;
            }
            Command::Set(_) | Command::Del | Command::Up | Command::Down if empty => {
                return Err("empty buffer");
            }
            Command::Set(text) => {
                if s.lines[s.current - 1] == text {
                    return Ok(String::new());
                }
                s.lines[s.current - 1] = text.to_string();
            }
            Command::Del => {
                s.lines.remove(s.current - 1);
                s.current = s.current.min(s.lines.len());
            }
            Command::Up if s.current == 1 => return Err("at top"),
            Command::Up => s.current -= 1,
            Command::Down if s.current == s.lines.len() => return Err("at bottom"),
            Command::Down => s.current += 1,
            Command::Undo => {
                let step = self.undo.pop().ok_or("nothing to undo")?;
                self.state = step.before.clone();
                self.redo.push(step);
                return Ok(String::new());
            }
            Command::Redo => {
                let step = self.redo.pop().ok_or("nothing to redo")?;
                self.state = step.after.clone();
                self.undo.push(step);
                return Ok(String::new());
            }
            Command::Print => return Ok(self.print()),
        }
        if matches!(command, Command::Up | Command::Down) {
            return Ok(String::new());
        }
        let after = self.state.clone();
        match self.undo.last_mut() {
            Some(step) if after_add && matches!(command, Command::Add(_)) => step.after = after,
            _ => {
                self.undo.push(Step { before, after });
                if self.undo.len() > 3 {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
        }
        Ok(String::new())
    }

    fn print(&self) -> String {
        let s = &self.state;
        let n = s.lines.len();
        let width = n.to_string().len();
        let mut out = format!("[{}/{n}]\n", s.current);
        for (i, line) in s.lines.iter().enumerate() {
            let mark = if i + 1 == s.current { '>' } else { ' ' };
            out += &format!("{mark}{:>width$}|{line}\n", i + 1);
        }
        out
    }
}

fn main() {
    let path = env::args().nth(1).expect("usage: main SCRIPT");
    let text = fs::read_to_string(path).expect("cannot read the script");
    let mut commands = vec![];
    for (i, line) in text.lines().enumerate() {
        let Some(command) = parse(line) else {
            println!("error: line {}: bad command", i + 1);
            process::exit(20);
        };
        commands.push(command);
    }
    let mut editor = Editor::default();
    let mut out = String::new();
    for (i, &command) in commands.iter().enumerate() {
        let after_add = i > 0 && matches!(commands[i - 1], Command::Add(_));
        match editor.run(command, after_add) {
            Ok(text) => out += &text,
            Err(reason) => out += &format!("line {}: {reason}\n", i + 1),
        }
    }
    out += &format!("undo {}, redo {}\n", editor.undo.len(), editor.redo.len());
    print!("{out}");
}
