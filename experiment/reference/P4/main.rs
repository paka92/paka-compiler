use std::collections::HashMap;
use std::process::exit;

const MAX: i64 = 9_999_999;

struct Op {
    line: usize,
    name: String,
    args: Vec<String>,
}

// The operand kinds of an instruction, or None when there is no such instruction.
fn kinds(name: &str) -> Option<&'static str> {
    Some(match name {
        "set" | "add" | "sub" | "mul" | "div" | "mod" => "RV",
        "jmp" | "call" => "L",
        "jz" => "VL",
        "jlt" => "VVL",
        "out" => "V",
        "ret" | "halt" => "",
        _ => return None,
    })
}

fn is_register(w: &str) -> bool {
    w.len() == 1 && ("a"..="f").contains(&w)
}

fn is_label(w: &str) -> bool {
    let mut cs = w.bytes();
    cs.next().is_some_and(|c| c.is_ascii_lowercase())
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn is_number(w: &str) -> bool {
    let digits = w.strip_prefix('-').unwrap_or(w);
    (1..=7).contains(&digits.len()) && digits.bytes().all(|c| c.is_ascii_digit())
}

fn fits(kind: u8, w: &str) -> bool {
    match kind {
        b'R' => is_register(w),
        b'V' => is_register(w) || is_number(w),
        _ => is_label(w),
    }
}

fn fail(message: String) -> ! {
    println!("{message}");
    exit(20);
}

// The instructions and the labels with the instruction index each names.
fn assemble(text: &str) -> (Vec<Op>, HashMap<String, usize>) {
    let mut ops = Vec::new();
    let mut labels = HashMap::new();
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    for (i, raw) in lines.iter().enumerate() {
        let line = i + 1;
        let code = raw.split(';').next().unwrap();
        let mut words: Vec<&str> = code.split(' ').filter(|w| !w.is_empty()).collect();
        if let Some(name) = words.first().and_then(|w| w.strip_suffix(':')) {
            if !is_label(name) {
                fail(format!("error: line {line}: bad label"));
            }
            if labels.insert(name.to_string(), ops.len()).is_some() {
                fail(format!("error: line {line}: duplicate label"));
            }
            words.remove(0);
        }
        let Some((&name, args)) = words.split_first() else {
            continue;
        };
        let Some(kinds) = kinds(name) else {
            fail(format!("error: line {line}: unknown instruction"));
        };
        if kinds.len() != args.len() {
            fail(format!("error: line {line}: wrong count"));
        }
        if !kinds.bytes().zip(args).all(|(k, w)| fits(k, w)) {
            fail(format!("error: line {line}: bad operand"));
        }
        let args = args.iter().map(|w| w.to_string()).collect();
        ops.push(Op {
            line,
            name: name.to_string(),
            args,
        });
    }
    for op in &ops {
        let kinds = kinds(&op.name).unwrap().bytes();
        for (_, l) in kinds.zip(&op.args).filter(|&(k, _)| k == b'L') {
            if !labels.contains_key(l) {
                fail(format!("error: line {}: unknown label {l}", op.line));
            }
        }
    }
    (ops, labels)
}

struct Machine {
    regs: [i64; 6],
    stack: Vec<usize>,
}

impl Machine {
    fn value(&self, w: &str) -> i64 {
        if is_register(w) {
            self.regs[(w.as_bytes()[0] - b'a') as usize]
        } else {
            w.parse().unwrap()
        }
    }

    // Runs one instruction; gives the next place, or an error.
    fn step(
        &mut self,
        op: &Op,
        pc: usize,
        labels: &HashMap<String, usize>,
    ) -> Result<usize, &'static str> {
        let a = &op.args;
        let label = |w: &String| labels[w];
        match op.name.as_str() {
            "jmp" => return Ok(label(&a[0])),
            "jz" if self.value(&a[0]) == 0 => return Ok(label(&a[1])),
            "jlt" if self.value(&a[0]) < self.value(&a[1]) => return Ok(label(&a[2])),
            "call" if self.stack.len() == 16 => return Err("stack overflow"),
            "call" => {
                self.stack.push(pc + 1);
                return Ok(label(&a[0]));
            }
            "ret" => return self.stack.pop().ok_or("ret without call"),
            "out" => println!("{}", self.value(&a[0])),
            "halt" => return Ok(usize::MAX),
            "jz" | "jlt" => {}
            name => {
                let (x, v) = (self.value(&a[0]), self.value(&a[1]));
                if (name == "div" || name == "mod") && v == 0 {
                    return Err("division by zero");
                }
                let r = match name {
                    "set" => v,
                    "add" => x + v,
                    "sub" => x - v,
                    "mul" => x * v,
                    "div" => x / v,
                    _ => x % v,
                };
                if r.abs() > MAX {
                    return Err("overflow");
                }
                self.regs[(a[0].as_bytes()[0] - b'a') as usize] = r;
            }
        }
        Ok(pc + 1)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let limit = &args[2];
    let good =
        limit.len() <= 6 && !limit.starts_with('0') && limit.bytes().all(|c| c.is_ascii_digit());
    let limit: usize = match limit.parse() {
        Ok(n) if good && (1..=100_000).contains(&n) => n,
        _ => fail("error: bad limit".to_string()),
    };
    let (ops, labels) = assemble(&std::fs::read_to_string(&args[1]).unwrap());
    let mut m = Machine {
        regs: [0; 6],
        stack: Vec::new(),
    };
    let (mut pc, mut steps) = (0, 0);
    while pc < ops.len() {
        if steps == limit {
            fail(format!("error: line {}: step limit", ops[pc].line));
        }
        steps += 1;
        match m.step(&ops[pc], pc, &labels) {
            Ok(next) => pc = next,
            Err(what) => fail(format!("error: line {}: {what}", ops[pc].line)),
        }
    }
}
