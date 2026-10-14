//! How expressions and types are spelled inside messages.

use super::{Span, span};
use crate::ast::{Arg, AssignOp, BinOp, Expr, ExprKind as K, StrPart, UnOp};

/// The name of the function a call names.
pub(super) fn named<'a>(e: &Expr<'a>) -> Option<&'a str> {
    match &e.kind {
        K::Call(f, _) => match f.kind {
            K::Name(name) => Some(name),
            _ => None,
        },
        _ => None,
    }
}

/// How a message names `e`: the function a call names, else the expression as written.
pub(super) fn called(e: &Expr) -> String {
    named(e).map_or_else(|| render(e), str::to_string)
}

/// `e` as it was written when that is short and plain; `..` stands for what is not. An index
/// is spelled out whenever it fits.
pub(super) fn render(e: &Expr) -> String {
    let mut suffixes = Vec::new();
    let mut cur = e;
    loop {
        let (suffix, inner) = match &cur.kind {
            K::Field(x, n) => (format!(".{}", n.text), x),
            K::Index(x, i) => (format!("[{}]", fit(i).unwrap_or_else(|| "..".into())), x),
            K::Call(x, args) => (format!("({})", arguments(args)), x),
            K::Try(x, _) => ("?".to_string(), x),
            _ => break,
        };
        suffixes.push(suffix);
        cur = inner;
    }
    suffixes.reverse();
    atom(cur) + &suffixes.concat()
}

fn arguments(args: &[Arg]) -> String {
    let each = |a: &Arg| match &a.label {
        Some(l) => format!("{}: {}", l.text, atom(&a.value)),
        None => atom(&a.value),
    };
    args.iter().map(each).collect::<Vec<_>>().join(", ")
}

fn atom(e: &Expr) -> String {
    match &e.kind {
        K::Name(text) | K::Int(text) | K::Float(text) => text.to_string(),
        K::Bool(b) => b.to_string(),
        K::Hole(name) => format!("??{name}"),
        K::Inout(x) => format!("&{}", render(x)),
        K::List(items) if items.is_empty() => "[]".to_string(),
        K::Map(pairs) if pairs.is_empty() => "[:]".to_string(),
        _ => "..".to_string(),
    }
}

/// Most bytes of source an expression may take and still be spelled out in a message.
const FIT: usize = 40;

/// `e` in full on one line: none when it is longer than `FIT`, holds a block or is a literal list,
/// map or record. A parenthesis that the source had is written back only where it is needed.
pub(super) fn fit(e: &Expr) -> Option<String> {
    if e.len > FIT {
        return None;
    }
    Some(match &e.kind {
        K::Name(text) | K::Int(text) | K::Float(text) => text.to_string(),
        K::Bool(b) => b.to_string(),
        K::Str(parts) => format!("\"{}\"", spelled(parts)?),
        K::Unary(op, x) => format!(
            "{}{}",
            if *op == UnOp::Neg { "-" } else { "!" },
            tight(x, 8)?
        ),
        K::Binary(op, _, l, r) => {
            let level = level(e);
            let left = tight(l, level + usize::from(level == 2))?;
            format!("{left} {} {}", op.text(), tight(r, level + 1)?)
        }
        K::Range(a, b) => format!("{}..<{}", tight(a, 3)?, tight(b, 3)?),
        K::Index(x, i) => format!("{}[{}]", tight(x, 9)?, fit(i)?),
        K::Field(x, name) => format!("{}.{}", tight(x, 9)?, name.text),
        K::Try(x, _) => format!("{}?", tight(x, 9)?),
        K::Inout(x) => format!("&{}", fit(x)?),
        K::Call(f, args) => {
            let each = |a: &Arg| match &a.label {
                Some(l) => Some(format!("{}: {}", l.text, fit(&a.value)?)),
                None => fit(&a.value),
            };
            let texts = args.iter().map(each).collect::<Option<Vec<_>>>()?;
            format!("{}({})", tight(f, 9)?, texts.join(", "))
        }
        _ => return None,
    })
}

/// How tightly an expression binds: binary operators from `||` (0) to `*` (4), a range (3), a
/// prefix operator (8), everything else (9).
fn level(e: &Expr) -> usize {
    match &e.kind {
        K::Binary(op, ..) => match op {
            BinOp::Or => 0,
            BinOp::And => 1,
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 2,
            BinOp::Add | BinOp::Sub => 3,
            BinOp::Mul | BinOp::Div | BinOp::Rem => 4,
        },
        K::Range(..) => 3,
        K::Unary(..) => 8,
        _ => 9,
    }
}

/// `e` in full, in parentheses when it binds looser than `at`.
fn tight(e: &Expr, at: usize) -> Option<String> {
    let text = fit(e)?;
    Some(if level(e) < at {
        format!("({text})")
    } else {
        text
    })
}

/// The text of a string literal between its quotes: escapes written back, parts in braces.
fn spelled(parts: &[StrPart]) -> Option<String> {
    let part = |p: &StrPart| match p {
        StrPart::Expr(x) => Some(format!("{{{}}}", fit(x)?)),
        StrPart::Text(text) => Some(text.chars().map(escaped).collect()),
    };
    Some(
        parts
            .iter()
            .map(part)
            .collect::<Option<Vec<String>>>()?
            .concat(),
    )
}

fn escaped(c: char) -> String {
    match c {
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        '\\' | '"' | '{' | '}' => format!("\\{c}"),
        _ => c.to_string(),
    }
}

/// The operator of an assignment as written.
pub(super) fn assign_op(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Set => "=",
        AssignOp::Add => "+=",
        AssignOp::Sub => "-=",
        AssignOp::Mul => "*=",
        AssignOp::Div => "/=",
        AssignOp::Rem => "%=",
    }
}

/// Where `e` stands without the parentheses around it. The parser gives a parenthesised
/// expression the span of the parentheses, so this reads the span back from the leaf it holds;
/// `e`'s own span when that is not found.
pub(super) fn plain(e: &Expr) -> Span {
    let mut head = e;
    while let K::Call(x, _) | K::Field(x, _) | K::Index(x, _) | K::Try(x, _) = &head.kind {
        head = x;
    }
    let width = match &head.kind {
        K::Name(t) | K::Int(t) | K::Float(t) => t.len(),
        K::Bool(b) => 4 + usize::from(!b),
        K::Hole(name) => name.len() + 2,
        _ => return span(e),
    };
    let ((line, col), (head_line, head_col)) = (e.at, head.at);
    if std::ptr::eq(head, e) {
        let outer = e.len.saturating_sub(width);
        return if outer > 0 && outer % 2 == 0 {
            ((line, col + outer / 2), width)
        } else {
            span(e)
        };
    }
    if head.len == width && head_line == line && head_col > col {
        return (head.at, e.len - 2 * (head_col - col));
    }
    span(e)
}

/// `text` with its article: "an Int", "a String".
pub(super) fn article(text: &str) -> String {
    let vowel = text.starts_with(['A', 'E', 'I', 'O']);
    format!("{} {text}", if vowel { "an" } else { "a" })
}

/// The repair for an Int that stands where a Float is needed.
pub(super) fn float_fix(e: &Expr) -> String {
    match &e.kind {
        K::Int(text) => format!("write {text}.0"),
        _ => format!("write to_float({})", render(e)),
    }
}
