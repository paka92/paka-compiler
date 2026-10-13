//! A built-in row, written in the syntax of a signature, read into a `Sig`.

use super::{Node, Param, Sig, Ty, Types};

struct Row<'t, 'a> {
    tokens: Vec<&'static str>,
    at: usize,
    types: &'t mut Types<'a>,
}

fn tokens(row: &'static str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut rest = row.trim_start();
    while !rest.is_empty() {
        let word = rest.find(|c: char| !c.is_alphanumeric() && c != '_');
        let len = if rest.starts_with("->") {
            2
        } else {
            word.unwrap_or(rest.len()).max(1)
        };
        out.push(&rest[..len]);
        rest = rest[len..].trim_start();
    }
    out
}

impl Row<'_, '_> {
    fn eat(&mut self, token: &str) -> bool {
        let hit = self.tokens.get(self.at) == Some(&token);
        self.at += usize::from(hit);
        hit
    }

    fn next(&mut self) -> &'static str {
        self.at += 1;
        self.tokens[self.at - 1]
    }

    /// Items up to the `close` token, with commas between.
    fn until<T>(&mut self, close: &str, mut item: impl FnMut(&mut Self) -> T) -> Vec<T> {
        let mut items = Vec::new();
        while !self.eat(close) {
            self.eat(",");
            items.push(item(self));
        }
        items
    }

    fn ty(&mut self) -> Ty {
        let head = self.next();
        let node = if head == "fn" {
            self.eat("(");
            let params = self.until(")", Self::ty);
            Node::Fn(params.into(), self.eat("->").then(|| self.ty()))
        } else if head.len() == 1 {
            Node::Var(head.chars().next().unwrap())
        } else {
            let args = if self.eat("<") {
                self.until(">", Self::ty)
            } else {
                Vec::new()
            };
            Node::Name(head, args.into())
        };
        self.types.intern(node)
    }

    fn params(&mut self) -> Vec<Param<'static>> {
        let mut labelled = false;
        let mut params = Vec::new();
        while !self.eat(")") {
            if self.eat(";") {
                labelled = true;
            } else if !self.eat(",") {
                let name = self.next();
                self.eat(":");
                let inout = self.eat("inout");
                params.push(Param {
                    name,
                    inout,
                    labelled,
                    ty: self.ty(),
                });
            }
        }
        params
    }
}

/// The signature a `Fn` or `Variant` row gives.
pub(super) fn sig(types: &mut Types, row: &'static str) -> Sig<'static> {
    let mut row = Row {
        tokens: tokens(row),
        at: 1,
        types,
    };
    let params = if row.eat("(") {
        row.params()
    } else {
        Vec::new()
    };
    let result = row.eat("->").then(|| row.ty());
    Sig { params, result }
}
