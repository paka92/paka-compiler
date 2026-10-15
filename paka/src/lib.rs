//! The front end of the compiler: from source text to the checked core form. The compiler's
//! back ends and the tools that read programs without compiling them use it as a library.
#![deny(clippy::float_arithmetic)]

pub mod ast;
mod body;
pub mod builtins;
pub mod core_form;
pub mod coverage;
pub mod diagnostic;
mod json;
mod lexer;
mod lower;
pub mod parser;
mod resolve;
pub mod sha256;
mod sig;
pub mod source;

use ast::Def;
use core_form::Program;
use diagnostic::Diagnostic;
use source::Source;

/// A hole in a checked program: `paka check` notes it, `paka build` refuses it unless an example
/// holds it (M2-06 section 1.5).
pub struct Hole {
    pub note: Diagnostic,
    pub refusal: Option<Diagnostic>,
}

/// A checked program's core form, its holes and its `unused-name` warnings; or its first error.
pub type Checked = Result<(Program, Vec<Hole>, Vec<Diagnostic>), Diagnostic>;

/// Parses and checks a program, then lowers it to the core form. Positions count lines across
/// the files until `Source::locate` places them.
pub fn check(path: &str, source: &Source) -> Checked {
    let files = source.files.iter().map(|f| (f.bytes.clone(), f.line));
    let program = parser::parse_files(path, &source.text, files)?;
    let resolved = resolve::resolve(path, &program)?;
    let mut table = sig::Table::build(path, &program, &resolved);
    if let Some(d) = second_app(path, &program) {
        return Err(d);
    }
    let bodies = body::check(path, &program, &resolved, &mut table);
    if let Some(d) = bodies.defs.iter().flatten().next() {
        return Err(d.clone());
    }
    let files = source
        .files
        .iter()
        .map(|f| (f.name.clone(), f.line))
        .collect();
    let core = lower::lower(files, &source.text, &program, &resolved, &table, &bodies);
    let holes = holes(path, &program, &table, &bodies, &core);
    Ok((core, holes, resolved.unused))
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
