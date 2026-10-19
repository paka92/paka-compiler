//! find and view (M4-18, docs/decisions/M4-03.md sections 3.1 and 3.2): lookups by name, user,
//! kind and state, each reply cut to a token budget of its whole line, bytes divided by 3.

use super::{Session, version};
use crate::builtins::{self, ROWS};
use crate::lexer::{Kind, Lexer};
use crate::serve::{Call, Failure, Reply, line_len};
use paka_json::Value;

/// One definition or built-in as find lists it.
struct Entry<'a> {
    name: &'a str,
    kind: &'a str,
    sig: &'a str,
    version: Option<String>,
}

impl Entry<'_> {
    fn value(&self) -> Value {
        Value::object([
            ("name", self.name.into()),
            ("kind", self.kind.into()),
            ("sig", self.sig.into()),
            ("version", self.version.clone().into()),
        ])
    }
}

impl Session {
    pub(super) fn find(&self, call: &Call) -> Reply {
        if call.get("takes").is_some() || call.get("gives").is_some() {
            let message = "find by takes and gives is not served yet";
            return Err(Failure::new("unknown_call", message));
        }
        let failing = self.failing();
        let name = call.str("name");
        let mut found: Vec<Entry> = (self.workspace.defs())
            .filter(|(names, _)| {
                name.is_none_or(|n| matches(n, &names[0]) || names.contains(&n.into()))
            })
            .filter(|(names, _)| {
                call.bool("failing")
                    .is_none_or(|f| f == failing.contains(&names[0]))
            })
            .filter(|(names, _)| call.str("users_of").is_none_or(|u| self.uses(names, u)))
            .map(|(names, text)| entry(&names[0], text))
            .collect();
        let exact = name.filter(|n| !n.contains('*'));
        if call.get("users_of").is_none() && call.bool("failing") != Some(true) {
            let builtin = |row: &&'static str| {
                exact.map_or(call.str("kind") == Some("builtin"), |n| {
                    builtins::name(row) == n
                })
            };
            found.extend(
                ROWS.iter()
                    .map(|&(_, row)| row)
                    .filter(builtin)
                    .map(|row| Entry {
                        name: builtins::name(row),
                        kind: "builtin",
                        sig: row,
                        version: None,
                    }),
            );
        }
        found.retain(|e| call.str("kind").is_none_or(|k| k == e.kind));
        found.retain(|e| call.str("after").is_none_or(|a| e.name > a));
        found.sort_by(|a, b| a.name.cmp(b.name));
        let entries = found.iter().map(Entry::value).collect();
        let reply =
            |found: Vec<Value>, more: usize| vec![("found", found.into()), ("more", more.into())];
        Ok(
            fitted(call, entries, reply, call.int("budget").unwrap_or(1000))
                .unwrap_or_else(|| reply(vec![], found.len())),
        )
    }

    pub(super) fn view(&self, call: &Call) -> Reply {
        let name = call.str("name").expect("a view names its definition");
        let own = |n: &str| {
            self.workspace
                .defs()
                .find(|(names, _)| names.contains(&n.into()))
        };
        let Some((names, text)) = own(name) else {
            return Err(Failure::new("not_found", format!("no definition `{name}`")));
        };
        let name = &names[0];
        let (_, check) = self.workspace.def(name).expect("an own definition");
        let mut uses: Vec<&str> = vec![];
        for id in idents(text) {
            let Some((names, _)) = own(id) else { continue };
            let used = names[0].as_str();
            if used != name && check.reads.contains(used) && !uses.contains(&used) {
                uses.push(used);
            }
        }
        let uses: Vec<Value> = (uses.iter())
            .map(|u| entry(u, self.workspace.def(u).expect("a used definition").0).value())
            .collect();
        let errors: Vec<Value> = check
            .error
            .iter()
            .map(|d| d.value(text.as_bytes(), None))
            .collect();
        let reply = |uses: Vec<Value>, more: usize| {
            vec![
                ("name", name.as_str().into()),
                ("kind", kind(text).into()),
                ("version", version(text).into()),
                ("text", text.into()),
                ("uses", uses.into()),
                ("more", more.into()),
                ("errors", errors.clone().into()),
            ]
        };
        let count = uses.len();
        fitted(call, uses, reply, call.int("budget").unwrap_or(2000)).ok_or_else(|| {
            let needs = tokens(line_len(call, &reply(vec![], count)));
            let mut failure =
                Failure::new("budget", format!("the body alone needs {needs} tokens"));
            failure.extra = vec![("needs", needs.into())];
            failure
        })
    }

    /// Whether the definition named by `user` uses `name`: another definition, a dangling name, a
    /// variant or a built-in.
    fn uses(&self, user: &[String], name: &str) -> bool {
        let defined = self.workspace.defs().any(|(names, _)| names[0] == name);
        let variant =
            (self.workspace.defs()).any(|(names, _)| names[1..].iter().any(|n| n == name));
        let builtin = ROWS.iter().any(|(_, row)| builtins::name(row) == name);
        if user.iter().any(|n| n == name) {
            false
        } else if variant || (builtin && !defined) {
            let (text, _) = self.workspace.def(&user[0]).expect("an own definition");
            idents(text).any(|id| id == name)
        } else {
            self.workspace.index().users(name).any(|u| u == user[0])
        }
    }
}

/// The reply that holds the longest prefix of `entries` that fits `budget`, and how many it left
/// out; `None` when even none fit.
fn fitted(
    call: &Call,
    entries: Vec<Value>,
    reply: impl Fn(Vec<Value>, usize) -> Vec<(&'static str, Value)>,
    budget: u64,
) -> Option<Vec<(&'static str, Value)>> {
    let digits = |n: usize| n.to_string().len();
    let fits = |len: usize| tokens(len) as u64 <= budget;
    let mut len = line_len(call, &reply(vec![], 0)) - 1;
    if !fits(len + digits(entries.len())) {
        return None;
    }
    let mut taken = 0;
    for entry in &entries {
        let added = len + entry.to_string().len() + usize::from(taken > 0);
        if !fits(added + digits(entries.len() - taken - 1)) {
            break;
        }
        len = added;
        taken += 1;
    }
    let more = entries.len() - taken;
    Some(reply(entries.into_iter().take(taken).collect(), more))
}

fn tokens(bytes: usize) -> usize {
    bytes.div_ceil(3)
}

fn entry<'a>(name: &'a str, text: &'a str) -> Entry<'a> {
    let kind = kind(text);
    let sig = match kind {
        "type" | "enum" => text,
        _ => text.split_once(" {").map_or(text, |(head, _)| head),
    };
    Entry {
        name,
        kind,
        sig,
        version: Some(version(text)),
    }
}

fn kind(text: &str) -> &str {
    text.split_once(' ').map_or(text, |(word, _)| word)
}

/// Whether `name` matches `pattern`, where `*` stands for any run of characters.
fn matches(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or("");
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<_> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(i) => rest = &rest[i + part.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

/// The identifiers of `text`, in order.
fn idents(text: &str) -> impl Iterator<Item = &str> {
    let mut lexer = Lexer::new("", text.as_bytes());
    std::iter::from_fn(move || lexer.next().ok().filter(|t| t.kind != Kind::End))
        .filter(|t| matches!(t.kind, Kind::Upper | Kind::Lower))
        .map(|t| std::str::from_utf8(t.text).expect("a slice of a str"))
}
