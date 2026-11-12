//! Reads the vector file `tests/vectors/ops.vec` and checks any back end against it.
//!
//! A back end is a function from an operation name and its inputs to an outcome, so the same
//! file can drive the interpreter now and a compiled back end later.

use super::Trap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    /// The exact bits.
    Float(u64),
    Bool(bool),
    Str(String),
    None,
    Some(Box<Value>),
}

pub type Outcome = Result<Value, Trap>;

#[derive(Debug)]
pub struct Vector {
    pub line: usize,
    pub op: String,
    pub inputs: Vec<Value>,
    pub want: Outcome,
}

const FLOAT_SIGN: u64 = 1 << 63;
const FLOAT_INF: u64 = 0x7FF0_0000_0000_0000;
const FLOAT_NAN: u64 = 0x7FF8_0000_0000_0000;

fn is_nan(bits: u64) -> bool {
    bits & !FLOAT_SIGN > FLOAT_INF
}

/// Floats match bit for bit; any NaN matches any NaN (M2-03 1.4).
pub fn same(got: &Outcome, want: &Outcome) -> bool {
    match (got, want) {
        (Ok(Value::Float(a)), Ok(Value::Float(b))) => a == b || is_nan(*a) && is_nan(*b),
        (Ok(Value::Some(a)), Ok(Value::Some(b))) => same(&Ok((**a).clone()), &Ok((**b).clone())),
        _ => got == want,
    }
}

/// The operations named in a rows file: one per line, `#` starts a comment line.
pub fn rows(file: &str) -> Vec<&str> {
    lines(file).map(|(_, line)| line).collect()
}

/// Every vector in the file; the error names the line that does not parse.
pub fn parse(file: &str) -> Result<Vec<Vector>, String> {
    lines(file)
        .map(|(n, line)| vector(n, line).map_err(|e| format!("line {n}: {e}")))
        .collect()
}

/// A description of each vector the back end gets wrong.
pub fn failures(vectors: &[Vector], run: impl Fn(&str, &[Value]) -> Outcome) -> Vec<String> {
    let wrong = |v: &&Vector| !same(&run(&v.op, &v.inputs), &v.want);
    let show = |v: &Vector| {
        let got = run(&v.op, &v.inputs);
        format!(
            "line {}: {} {:?}: got {got:?}, want {:?}",
            v.line, v.op, v.inputs, v.want
        )
    };
    vectors.iter().filter(wrong).map(show).collect()
}

fn lines(file: &str) -> impl Iterator<Item = (usize, &str)> {
    (1..)
        .zip(file.lines())
        .map(|(n, line)| (n, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
}

fn vector(line: usize, text: &str) -> Result<Vector, String> {
    let [op, inputs, want] = text.split('|').collect::<Vec<_>>()[..] else {
        return Err("want `operation | inputs | expected`".into());
    };
    let inputs = words(inputs)?
        .iter()
        .map(|w| value(w))
        .collect::<Result<_, _>>()?;
    Ok(Vector {
        line,
        op: op.trim().into(),
        inputs,
        want: outcome(&words(want)?)?,
    })
}

/// Splits on spaces outside double quotes; a word keeps its quotes.
fn words(text: &str) -> Result<Vec<String>, String> {
    let (mut words, mut word, mut quoted, mut escaped) = (vec![], String::new(), false, false);
    for c in text.chars() {
        if c.is_whitespace() && !quoted {
            words.extend((!word.is_empty()).then(|| std::mem::take(&mut word)));
            continue;
        }
        word.push(c);
        (quoted, escaped) = match c {
            _ if escaped => (quoted, false),
            '\\' => (quoted, true),
            '"' => (!quoted, false),
            _ => (quoted, false),
        };
    }
    words.extend((!word.is_empty()).then_some(word));
    if quoted {
        Err("unterminated string".into())
    } else {
        Ok(words)
    }
}

fn outcome(words: &[String]) -> Result<Outcome, String> {
    match words {
        [w, name] if w == "trap" => Trap::from_name(name)
            .map(Err)
            .ok_or(format!("no trap {name}")),
        [w, v] if w == "some" => Ok(Ok(Value::Some(Box::new(value(v)?)))),
        [v] => value(v).map(Ok),
        _ => Err("want a value, `none`, `some` and a value, or `trap` and a name".into()),
    }
}

fn value(word: &str) -> Result<Value, String> {
    let bad = || format!("cannot read `{word}`");
    Ok(match word {
        "none" => Value::None,
        "true" | "false" => Value::Bool(word == "true"),
        "MIN" => Value::Int(i64::MIN),
        "MAX" => Value::Int(i64::MAX),
        "nan" => Value::Float(FLOAT_NAN),
        "inf" => Value::Float(FLOAT_INF),
        "-inf" => Value::Float(FLOAT_INF | FLOAT_SIGN),
        _ if word.starts_with('"') => Value::Str(string(word).ok_or_else(bad)?),
        _ if word.starts_with("0x") => {
            Value::Float(u64::from_str_radix(&word[2..], 16).map_err(|_| bad())?)
        }
        _ if word.contains('.') => Value::Float(word.parse::<f64>().map_err(|_| bad())?.to_bits()),
        _ => Value::Int(word.parse().map_err(|_| bad())?),
    })
}

/// The text between the quotes with `\n \t \" \\ \u{hex}` resolved.
fn string(word: &str) -> Option<String> {
    let mut chars = word.strip_prefix('"')?.strip_suffix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        out.push(if c != '\\' {
            c
        } else {
            match chars.next()? {
                'n' => '\n',
                't' => '\t',
                c @ ('"' | '\\') => c,
                'u' => {
                    let hex: String = chars.by_ref().skip(1).take_while(|c| *c != '}').collect();
                    char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?
                }
                _ => return None,
            }
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn float(bits: u64) -> Outcome {
        Ok(Value::Float(bits))
    }

    #[test]
    fn floats_match_bit_for_bit_but_any_nan_matches() {
        assert!(same(&float(0), &float(0)));
        assert!(!same(&float(0), &float(FLOAT_SIGN)));
        assert!(same(&float(FLOAT_NAN), &float(FLOAT_SIGN | FLOAT_NAN | 1)));
        assert!(!same(&float(FLOAT_INF), &float(FLOAT_NAN)));
        assert!(!same(&float(FLOAT_NAN), &Ok(Value::Int(0))));
    }

    #[test]
    fn a_vector_is_read_whole() {
        let file = "# note\n\nop | 1 -0.0 \"a b:\\u{E9}\\\"\" | some MAX\nop | | trap overflow\n";
        let v = parse(file).unwrap();
        assert_eq!(v[0].line, 3);
        assert_eq!(
            v[0].inputs,
            [
                Value::Int(1),
                Value::Float(FLOAT_SIGN),
                Value::Str("a b:é\"".into())
            ]
        );
        assert_eq!(v[0].want, Ok(Value::Some(Box::new(Value::Int(i64::MAX)))));
        assert_eq!((v[1].inputs.len(), &v[1].want), (0, &Err(Trap::Overflow)));
    }

    #[test]
    fn a_bad_line_is_named() {
        for bad in [
            "op | 1",
            "op | 1 | 0x",
            "op | \"x | 1",
            "op | 1 | trap nope",
            "op | 1 | 2 3",
        ] {
            let e = parse(&format!("\n{bad}\n")).unwrap_err();
            assert!(e.starts_with("line 2:"), "{bad}: {e}");
        }
    }
}
