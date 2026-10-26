//! The front end of the compiler: from source text to the checked core form. The compiler's
//! back ends and the tools that read programs without compiling them use it as a library.
#![deny(clippy::float_arithmetic)]

pub mod alone;
pub mod approvals;
pub mod approve;
pub mod ast;
mod body;
pub mod builtins;
pub mod clang;
pub mod core_form;
pub mod coverage;
pub mod diagnostic;
pub mod emit;
pub mod foreign;
pub mod gate;
pub mod hash;
pub mod index;
pub mod interp;
pub mod layout;
mod lexer;
mod lower;
pub mod migrate;
pub mod owners;
pub mod parser;
pub mod pending;
pub mod print;
mod resolve;
pub mod results;
pub mod serve;
pub mod session;
pub mod sha256;
mod sig;
pub mod source;
pub mod workspace;

use ast::Def;
use core_form::Program;
use diagnostic::Diagnostic;
use source::Source;
use std::collections::BTreeSet;

/// A hole in a checked program: `paka check` notes it, `paka build` refuses it unless an example
/// holds it (M2-06 section 1.5).
pub struct Hole {
    pub note: Diagnostic,
    pub refusal: Option<Diagnostic>,
}

/// A checked program's core form, its holes and its `unused-name` warnings; or its first error.
pub type Checked = Result<Lowered, Diagnostic>;
pub type Lowered = (Program, Vec<Hole>, Vec<Diagnostic>);

/// One definition checked from its body and the signatures it uses (M4-09).
#[derive(Clone)]
pub struct DefCheck {
    pub name: String,
    pub error: Option<Diagnostic>,
    /// The other definitions it read, by name, and the names it looked up that name nothing.
    pub reads: BTreeSet<String>,
    /// Decides whether it is rechecked.
    pub body: hash::Hash,
    /// Decides whether its users are rechecked.
    pub interface: hash::Hash,
}

/// Parses and checks a program, then lowers it to the core form. Positions count lines across
/// the files until `Source::locate` places them.
pub fn check(path: &str, source: &Source) -> Checked {
    check_defs(path, source).map(|(lowered, _)| lowered)
}

/// As `check`, with each definition's check, which keys the example cache (M7-09).
pub fn check_defs(path: &str, source: &Source) -> Result<(Lowered, Vec<DefCheck>), Diagnostic> {
    let (defs, lowered) = check_all(path, source)?;
    match defs.iter().find_map(|d| d.error.clone()) {
        Some(d) => Err(d),
        None => Ok((lowered.expect("a program without errors is lowered"), defs)),
    }
}

/// Checks each definition alone. The error is one no definition owns: a parse error, a
/// declaration error or a second app.
pub fn check_each(path: &str, source: &Source) -> Result<Vec<DefCheck>, Diagnostic> {
    Ok(check_all(path, source)?.0)
}

/// Each definition's check, and the lowered program when none has an error; lowering adds its
/// reads to each definition's.
fn check_all(path: &str, source: &Source) -> Result<(Vec<DefCheck>, Option<Lowered>), Diagnostic> {
    let files = source.files.iter().map(|f| (f.bytes.clone(), f.line));
    let program = parser::parse_files(path, &source.text, files)?;
    let resolved = resolve::resolve(path, &program)?;
    let mut table = sig::Table::build(path, &program, &resolved);
    if let Some(d) = second_app(path, &program) {
        return Err(d);
    }
    let mut bodies = body::check(path, &program, &resolved, &mut table);
    let mut lowered = None;
    let mut reads = std::mem::take(&mut bodies.reads);
    if bodies.defs.iter().all(Option::is_none) {
        let (core, more) = lower::lower(source, &program, &resolved, &table, &bodies);
        reads.iter_mut().zip(more).for_each(|(r, m)| r.extend(m));
        let holes = holes(path, &program, &table, &bodies, &core);
        lowered = Some((core, holes, resolved.unused));
    }
    let reads: Vec<_> = (program.defs.iter().zip(reads).zip(resolved.reads))
        .map(|((def, mut reads), more)| {
            reads.extend(more);
            reads.remove(def.name().text);
            reads
        })
        .collect();
    let hashes = hash::hashes(&program, &reads);
    let defs = (program.defs.iter().zip(bodies.defs).zip(reads).zip(hashes))
        .map(|(((def, error), reads), (body, interface))| {
            let name = def.name().text.into();
            DefCheck {
                name,
                error,
                reads,
                body,
                interface,
            }
        })
        .collect();
    Ok((defs, lowered))
}

/// The holes of a checked program, in position order.
fn holes(
    path: &str,
    program: &ast::Program,
    table: &sig::Table,
    bodies: &body::Bodies,
    core: &Program,
) -> Vec<Hole> {
    let mut sites: Vec<_> = (core.sites.iter())
        .filter_map(|s| Some(((s.line, s.col), s.hole.as_deref()?)))
        .collect();
    sites.sort();
    sites.dedup();
    let hole = |(at, name): (ast::Pos, &str)| {
        let def = program.defs.iter().rposition(|d| d.name().at <= at);
        let def = def.expect("a hole stands in a definition");
        let len = name.len() + 2;
        let takes =
            (bodies.types[def].get(&(at, len))).map_or("no value".into(), |&t| table.types.show(t));
        let note = Diagnostic::new(path, at, len, "hole", format!("??{name} takes {takes}"));
        let finish = format!("a release build has no holes; finish ??{name}");
        let refusal = Diagnostic::new(path, at, len, "hole-in-build", finish);
        Hole {
            note: note.in_definition(program.defs[def].name().text),
            refusal: (!matches!(program.defs[def], Def::Example { .. })).then_some(refusal),
        }
    };
    sites.into_iter().map(hole).collect()
}

fn second_app(path: &str, program: &ast::Program) -> Option<Diagnostic> {
    let mut apps = program.defs.iter().filter_map(|d| match d {
        Def::App { name, .. } => Some(name),
        _ => None,
    });
    let second = apps.nth(1)?;
    let message = "a program holds one app";
    let d = Diagnostic::new(path, second.at, second.text.len(), "multiple-apps", message);
    Some(d.in_definition(second.text))
}
