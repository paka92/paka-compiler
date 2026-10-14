//! How expressions and types are spelled inside messages.

use super::{Span, span};
use crate::ast::{Arg, Expr, ExprKind as K};

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

/// `e` as it was written when that is short and plain; `..` stands for what is not.
pub(super) fn render(e: &Expr) -> String {
    let mut suffixes = Vec::new();
    let mut cur = e;
    loop {
        let (suffix, inner) = match &cur.kind {
            K::Field(x, n) => (format!(".{}", n.text), x),
            K::Index(x, i) => (format!("[{}]", atom(i)), x),
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
