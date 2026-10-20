use super::Emitter;
use crate::source::Source;
use cranelift_codegen::ir::{Function, InstructionData, Opcode, ValueDef};

/// Every construct M4-26 and M4-27 compile, with each check they emit.
const PROGRAM: &str = "\
fn arith(a: Int, b: Int, x: Float) -> Int {
  var n = a + b * a - b
  n = n / b + n % b
  n += -a
  n = abs(n) + shift_left(n, b) + shift_right(n, 3) + to_int(round(x) + floor(x))
  n
}

fn flow(limit: Int, on: Bool) -> Bool {
  var total = 0
  for i in 0..<limit {
    if i == 3 { continue }
    if i > 10 && on { break }
    total += i
  }
  while total > 0 { total -= 1 }
  assert(total == 0)
  !on || total < 1
}

fn calls(n: Int) -> Int { arith(n, n, 0.5) + if flow(n, true) { 1 } else { 0 } }

example arithmetic {
  7 / 2 == 3 && to_float(2) < 2.5
}
";

fn ir(def: usize) -> Function {
    let (p, ..) = crate::check("main.paka", &Source::file("main.paka", PROGRAM.into())).unwrap();
    Emitter::new().unwrap().ir(&p, def).unwrap()
}

/// The instructions that may fault or end the process, and divides not guarded by a select.
fn banned(f: &Function) -> Vec<String> {
    let mut found = vec![];
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            let data = &f.dfg.insts[inst];
            let op = data.opcode();
            let bad = match op {
                Opcode::Trap | Opcode::Trapz | Opcode::Trapnz => true,
                Opcode::FcvtToSint | Opcode::FcvtToUint => true,
                Opcode::Sdiv | Opcode::Srem | Opcode::Udiv | Opcode::Urem => {
                    let divisor = f.dfg.inst_args(inst)[1];
                    !matches!(f.dfg.value_def(divisor), ValueDef::Result(i, _)
                        if matches!(f.dfg.insts[i], InstructionData::Ternary { opcode: Opcode::Select, .. }))
                }
                _ => false,
            };
            if bad {
                found.push(format!("{op}"));
            }
        }
    }
    found
}

#[test]
fn no_emitted_instruction_faults() {
    for def in 0..4 {
        let f = ir(def);
        assert_eq!(
            banned(&f),
            Vec::<String>::new(),
            "definition {def}:\n{}",
            f.display()
        );
    }
}

#[test]
fn the_scan_finds_an_unguarded_divide() {
    let mut f = ir(0);
    let divide = f
        .layout
        .blocks()
        .flat_map(|b| f.layout.block_insts(b))
        .find(|&i| f.dfg.insts[i].opcode() == Opcode::Sdiv);
    let divide = divide.expect("arith divides");
    let x = f.dfg.inst_args(divide)[0];
    f.dfg.inst_args_mut(divide)[1] = x;
    assert_eq!(banned(&f), ["sdiv"]);
}

#[test]
fn every_definition_compiles_to_code() {
    let (p, ..) = crate::check("main.paka", &Source::file("main.paka", PROGRAM.into())).unwrap();
    let mut emitter = Emitter::new().unwrap();
    for def in 0..4 {
        assert!(!emitter.code(&p, def).unwrap().is_empty());
    }
}
