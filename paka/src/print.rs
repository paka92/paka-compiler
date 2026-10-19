//! Prints a program in the one canonical layout of docs/decisions/M4-47.md section 2, keeping its
//! comments (M4-15). Names, numbers, strings and holes print as written, and so do parentheses.
//!
//! A comment is placed by lines. Definitions, statements, match arms and app keys are holders;
//! the file, blocks, matches and app bodies are containers. A comment strictly inside a container
//! goes above the next holder there, or at the container's end. One on a holder's own lines, but
//! inside none of the containers that holder has, goes above the holder. Comments after the last
//! definition go above it.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::lexer::{Kind, Lexer};
use crate::parser::parse;
use std::collections::BTreeMap;
use std::fmt::Write;

/// The canonical text of a program, or the first error that stops it parsing.
pub fn program(path: &str, src: &[u8]) -> Result<String, Diagnostic> {
    let ast = parse(path, src)?;
    let mut p = Printer::new(path, src)?;
    for (i, def) in ast.defs.iter().enumerate() {
        if i > 0 {
            p.out.push('\n');
        }
        let (first, last) = p.def_lines(def);
        let last = if i + 1 == ast.defs.len() {
            usize::MAX
        } else {
            last
        };
        p.holder(0, (first, last), |p| p.def(def));
        p.out.push('\n');
    }
    for comment in p.take(0, usize::MAX) {
        p.out += comment;
        p.out.push('\n');
    }
    Ok(p.out)
}

struct Printer<'s> {
    src: &'s [u8],
    /// The offset each line starts at.
    lines: Vec<usize>,
    /// The offset of each opening bracket's token, and its closing one's.
    closes: BTreeMap<usize, usize>,
    /// The offset of each token that is no bracket, with the bytes it spans.
    tokens: BTreeMap<usize, usize>,
    /// Each comment by its line; a line holds at most one.
    comments: BTreeMap<usize, &'s str>,
    out: String,
    depth: usize,
}

impl<'s> Printer<'s> {
    fn new(path: &str, src: &'s [u8]) -> Result<Self, Diagnostic> {
        let mut p = Printer {
            src,
            lines: std::iter::once(0)
                .chain((0..src.len()).filter(|i| src[*i] == b'\n').map(|i| i + 1))
                .collect(),
            closes: BTreeMap::new(),
            tokens: BTreeMap::new(),
            comments: BTreeMap::new(),
            out: String::new(),
            depth: 0,
        };
        let mut lexer = Lexer::new(path, src);
        let (mut end, mut open) = (0, vec![]);
        loop {
            let token = lexer.next()?;
            p.comment_in(end, token.start);
            match token.kind {
                Kind::End => return Ok(p),
                Kind::LParen | Kind::LBracket | Kind::LBrace => open.push(token.start),
                Kind::RParen | Kind::RBracket | Kind::RBrace => {
                    p.closes
                        .insert(open.pop().expect("a parsed program"), token.start);
                }
                _ => drop(p.tokens.insert(token.start, token.len())),
            }
            end = token.start + token.len();
        }
    }

    /// Records the comments between two tokens.
    fn comment_in(&mut self, from: usize, to: usize) {
        let mut at = from;
        while let Some(i) = self.src[at..to].windows(2).position(|w| w == b"//") {
            let start = at + i;
            let len = self.src[start..to].iter().position(|b| *b == b'\n');
            at = len.map_or(to, |len| start + len);
            let text = std::str::from_utf8(&self.src[start..at]).expect("the lexer yields UTF-8");
            self.comments.insert(self.line_of(start), text.trim_end());
        }
    }

    fn line_of(&self, offset: usize) -> usize {
        self.lines.partition_point(|start| *start <= offset)
    }

    fn offset(&self, (line, col): Pos) -> usize {
        self.lines[line - 1] + col - 1
    }

    fn end<K>(&self, node: &Node<K>) -> usize {
        self.offset(node.at) + node.len
    }

    fn last_line<K>(&self, node: &Node<K>) -> usize {
        self.line_of(self.end(node) - 1)
    }

    /// The offsets of the first `{` from `from` and of its `}`.
    fn braces(&self, from: usize) -> (usize, usize) {
        let open = self.closes.range(from..).map(|(o, c)| (*o, *c));
        open.into_iter()
            .find(|(o, _)| self.src[*o] == b'{')
            .expect("a parsed program")
    }

    /// Removes the comments on lines `from` up to, not including, `to`.
    fn take(&mut self, from: usize, to: usize) -> Vec<&'s str> {
        let keys: Vec<_> = match from < to {
            true => self.comments.range(from..to).map(|(k, _)| *k).collect(),
            false => vec![],
        };
        keys.iter()
            .map(|k| self.comments.remove(k).unwrap())
            .collect()
    }

    fn newline(&mut self) {
        self.out.push('\n');
        (0..self.depth).for_each(|_| self.out += "  ");
    }

    /// Prints one holder spanning `lines` in a container opened on line `open`, with the comments
    /// that go above it.
    fn holder(
        &mut self,
        open: usize,
        (first, last): (usize, usize),
        print: impl FnOnce(&mut Self),
    ) {
        let mut above = self.take(open + 1, first);
        let outer = std::mem::take(&mut self.out);
        print(self);
        let text = std::mem::replace(&mut self.out, outer);
        above.extend(self.take(first, last.saturating_add(1)));
        for comment in above {
            self.out += comment;
            self.newline();
        }
        self.out += &text;
    }

    /// Prints `{`, the holders one per line, the comments left before the `}`, and `}`.
    fn container<T>(
        &mut self,
        (open, close): (usize, usize),
        items: &[T],
        lines: impl Fn(&Self, &T) -> (usize, usize),
        print: impl Fn(&mut Self, &T),
    ) {
        let (open, close) = (self.line_of(open), self.line_of(close));
        if items.is_empty()
            && (close <= open + 1 || self.comments.range(open + 1..close).next().is_none())
        {
            self.out += "{}";
            return;
        }
        self.out.push('{');
        self.depth += 1;
        for item in items {
            self.newline();
            let span = lines(self, item);
            self.holder(open, span, |p| print(p, item));
        }
        for comment in self.take(open + 1, close) {
            self.newline();
            self.out += comment;
        }
        self.depth -= 1;
        self.newline();
        self.out.push('}');
    }

    fn def_lines(&self, def: &Def) -> (usize, usize) {
        let name = def.name();
        let close = match def {
            Def::Fn { body, .. } | Def::Example { body, .. } => self.offset(body.close),
            _ => self.braces(self.offset(name.at)).1,
        };
        (name.at.0, self.line_of(close))
    }

    fn def(&mut self, def: &Def) {
        let name = def.name();
        let after = self.offset(name.at) + name.text.len();
        match def {
            Def::Type { fields, .. } => {
                write!(self.out, "type {} ", name.text).unwrap();
                self.braced(fields, Self::field);
            }
            Def::Enum { variants, .. } => {
                write!(self.out, "enum {} ", name.text).unwrap();
                self.braced(variants, |p, v| {
                    p.out += v.name.text;
                    if !v.fields.is_empty() {
                        p.list("(", &v.fields, Self::field, ")");
                    }
                });
            }
            Def::Fn {
                params,
                result,
                body,
                ..
            } => {
                write!(self.out, "fn {}(", name.text).unwrap();
                let bare = params.iter().take_while(|p| !p.labelled).count();
                self.join(&params[..bare], Self::param);
                if bare < params.len() {
                    self.out += "; ";
                    self.join(&params[bare..], Self::param);
                }
                self.out.push(')');
                if let Some(result) = result {
                    write!(self.out, " -> {result}").unwrap();
                }
                self.out.push(' ');
                self.block(after, body);
            }
            Def::Example { body, .. } => {
                write!(self.out, "example {} ", name.text).unwrap();
                self.block(after, body);
            }
            Def::App { entries, .. } => {
                write!(self.out, "app {} ", name.text).unwrap();
                let lines = |p: &Self, e: &Entry| {
                    let last = match e {
                        Entry::State(_, n) | Entry::Update(_, n) => n.at.0,
                        Entry::Start(_, e) => p.last_line(e),
                        Entry::Grants(at, names) => names.last().map_or(at.0, |n| n.at.0),
                    };
                    (entry_at(e).0, last)
                };
                self.container(self.braces(after), entries, lines, Self::entry);
            }
        }
    }

    /// `{ a, b }`, or `{}` when empty.
    fn braced<T>(&mut self, items: &[T], print: impl Fn(&mut Self, &T)) {
        match items {
            [] => self.out += "{}",
            _ => self.list("{ ", items, print, " }"),
        }
    }

    fn list<T>(&mut self, open: &str, items: &[T], print: impl Fn(&mut Self, &T), close: &str) {
        self.out += open;
        self.join(items, print);
        self.out += close;
    }

    fn join<T>(&mut self, items: &[T], print: impl Fn(&mut Self, &T)) {
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                self.out += ", ";
            }
            print(self, item);
        }
    }

    fn field(&mut self, f: &Field) {
        write!(self.out, "{}: {}", f.name.text, f.ty).unwrap();
        if let Some(default) = &f.default {
            self.out += " = ";
            self.expr(default);
        }
    }

    fn param(&mut self, p: &Param) {
        let inout = if p.inout { "inout " } else { "" };
        write!(self.out, "{}: {inout}{}", p.name.text, p.ty).unwrap();
    }

    fn entry(&mut self, e: &Entry) {
        match e {
            Entry::State(_, n) => write!(self.out, "state: {}", n.text).unwrap(),
            Entry::Start(_, e) => {
                self.out += "start: ";
                self.expr(e);
            }
            Entry::Update(_, n) => write!(self.out, "update: {}", n.text).unwrap(),
            Entry::Grants(_, names) => {
                self.list("grants: [", names, |p, n| p.out += n.text, "]");
            }
        }
    }

    /// A block whose `{` is the first from offset `from`.
    fn block(&mut self, from: usize, b: &Block) {
        let lines = |p: &Self, s: &Stmt| (s.at.0, p.last_line(s));
        self.container(self.braces(from), &b.stmts, lines, Self::stmt);
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let {
                var,
                name,
                ty,
                value,
            } => {
                let word = if *var { "var" } else { "let" };
                write!(self.out, "{word} {}", name.text).unwrap();
                if let Some(ty) = ty {
                    write!(self.out, ": {ty}").unwrap();
                }
                self.out += " = ";
                self.expr(value);
            }
            StmtKind::Discard(value) => {
                self.out += "_ = ";
                self.expr(value);
            }
            StmtKind::Assign { target, op, value } => {
                self.expr(target);
                self.out += match op {
                    AssignOp::Set => " = ",
                    AssignOp::Add => " += ",
                    AssignOp::Sub => " -= ",
                    AssignOp::Mul => " *= ",
                    AssignOp::Div => " /= ",
                    AssignOp::Rem => " %= ",
                };
                self.expr(value);
            }
            StmtKind::Return(value) => {
                self.out += "return";
                if let Some(value) = value {
                    self.out.push(' ');
                    self.expr(value);
                }
            }
            StmtKind::Break => self.out += "break",
            StmtKind::Continue => self.out += "continue",
            StmtKind::For { name, list, body } => {
                write!(self.out, "for {} in ", name.text).unwrap();
                self.expr(list);
                self.out.push(' ');
                self.block(self.end(list), body);
            }
            StmtKind::While { cond, body } => {
                self.out += "while ";
                self.expr(cond);
                self.out.push(' ');
                self.block(self.end(cond), body);
            }
            StmtKind::Expr(e) => self.expr(e),
        }
    }

    fn expr(&mut self, e: &Expr) {
        let at = self.offset(e.at);
        let parens = self.src[at] == b'(' && self.closes.get(&at) == Some(&(self.end(e) - 1));
        if parens {
            self.out.push('(');
        }
        match &e.kind {
            ExprKind::Int(s) | ExprKind::Float(s) | ExprKind::Name(s) => self.out += s,
            ExprKind::Bool(b) => write!(self.out, "{b}").unwrap(),
            ExprKind::Str(_) => self.spelled(at),
            ExprKind::Hole(name) => write!(self.out, "??{name}").unwrap(),
            ExprKind::Record(name, fields) => {
                write!(self.out, "{name} ").unwrap();
                self.braced(fields, Self::arg);
            }
            ExprKind::List(items) => self.list("[", items, Self::expr, "]"),
            ExprKind::Map(pairs) if pairs.is_empty() => self.out += "[:]",
            ExprKind::Map(pairs) => {
                let pair = |p: &mut Self, (k, v): &(Expr, Expr)| {
                    p.expr(k);
                    p.out += ": ";
                    p.expr(v);
                };
                self.list("[", pairs, pair, "]");
            }
            ExprKind::Unary(op, x) => {
                self.out.push(if *op == UnOp::Neg { '-' } else { '!' });
                self.expr(x);
            }
            ExprKind::Binary(op, _, a, b) => {
                self.expr(a);
                write!(self.out, " {} ", op.text()).unwrap();
                self.expr(b);
            }
            ExprKind::Range(a, b) => {
                self.expr(a);
                self.out += "..<";
                self.expr(b);
            }
            ExprKind::Index(a, i) => {
                self.expr(a);
                self.list("[", std::slice::from_ref(&**i), Self::expr, "]");
            }
            ExprKind::Field(a, name) => {
                self.expr(a);
                write!(self.out, ".{}", name.text).unwrap();
            }
            ExprKind::Try(a, _) => {
                self.expr(a);
                self.out.push('?');
            }
            ExprKind::Call(f, args) => {
                self.expr(f);
                self.list("(", args, Self::arg, ")");
            }
            ExprKind::Inout(x) => {
                self.out.push('&');
                self.expr(x);
            }
            ExprKind::If {
                branches,
                otherwise,
            } => self.if_expr(branches, otherwise.as_ref()),
            ExprKind::Match { subject, arms } => {
                self.out += "match ";
                self.expr(subject);
                self.out.push(' ');
                let lines = |p: &Self, arm: &Arm| {
                    let last = match &arm.body {
                        ArmBody::Block(b) => b.close.0,
                        ArmBody::Line(s) => p.last_line(s),
                    };
                    (arm.patterns[0].at.0, last)
                };
                self.container(self.braces(self.end(subject)), arms, lines, Self::arm);
            }
        }
        if parens {
            self.out.push(')');
        }
    }

    /// The token at or after `at` as written, past any opening parentheses.
    fn spelled(&mut self, at: usize) {
        let (start, len) = self.tokens.range(at..).next().expect("a parsed program");
        let text = &self.src[*start..start + len];
        self.out += std::str::from_utf8(text).expect("the lexer yields UTF-8");
    }

    fn if_expr(&mut self, branches: &[(Expr, Block)], otherwise: Option<&Block>) {
        for (i, (cond, body)) in branches.iter().enumerate() {
            if i > 0 {
                self.out += " else ";
            }
            self.out += "if ";
            self.expr(cond);
            self.out.push(' ');
            self.block(self.end(cond), body);
        }
        if let Some(body) = otherwise {
            let (_, last) = branches.last().expect("an if has a branch");
            self.out += " else ";
            self.block(self.offset(last.close) + 1, body);
        }
    }

    fn arg(&mut self, a: &Arg) {
        if let Some(label) = &a.label {
            write!(self.out, "{}: ", label.text).unwrap();
        }
        self.expr(&a.value);
    }

    fn arm(&mut self, arm: &Arm) {
        for (i, pat) in arm.patterns.iter().enumerate() {
            if i > 0 {
                self.out += " | ";
            }
            self.pat(pat);
        }
        self.out += " => ";
        match &arm.body {
            ArmBody::Block(b) => {
                let last = arm.patterns.last().expect("an arm has a pattern");
                self.block(self.end(last), b);
            }
            ArmBody::Line(s) => self.stmt(s),
        }
    }

    fn pat(&mut self, pat: &Pat) {
        match &pat.kind {
            PatKind::Wild => self.out.push('_'),
            PatKind::Int(s) | PatKind::Bind(s) => self.out += s,
            PatKind::Str(_) => self.spelled(self.offset(pat.at)),
            PatKind::Bool(b) => write!(self.out, "{b}").unwrap(),
            PatKind::Variant(name, binds) => {
                self.out += name;
                if !binds.is_empty() {
                    self.list("(", binds, Self::pat, ")");
                }
            }
        }
    }
}

fn entry_at(e: &Entry) -> Pos {
    match e {
        Entry::State(at, _) | Entry::Start(at, _) | Entry::Update(at, _) | Entry::Grants(at, _) => {
            *at
        }
    }
}

#[cfg(test)]
mod tests;
