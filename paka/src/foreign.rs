//! The host of a `foreign` declaration (docs/decisions/M7-02.md section 1.6): the pinned clang
//! builds the fixed loop in `foreign/host.c` with C glue generated for the declaration, from
//! copies of its named files in an empty directory.

mod pipe;

pub use pipe::{Failure, Host, Kind, Value};

use crate::ast::{ForeignLine, Param, Type};
use crate::clang;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HOST: &str = include_str!("foreign/host.c");

/// Builds the host of the checked declaration `lines`, whose files are in `dir`, inside `out`,
/// which must not exist yet: the executable's path, or why there is none.
pub fn build(dir: &Path, lines: &[ForeignLine], out: &Path) -> Result<PathBuf, String> {
    clang::check("").map_err(|d| d.message)?;
    let failed = |e: std::io::Error| format!("{}: {e}", out.display());
    fs::create_dir(out).map_err(failed)?;
    fs::create_dir(out.join("lib")).map_err(failed)?;
    fs::write(out.join("paka_host.c"), HOST).map_err(failed)?;
    fs::write(out.join("glue.c"), glue(lines)).map_err(failed)?;
    let (mut sources, mut libraries) = (vec![], vec![]);
    for line in lines {
        let ForeignLine::File(key, file) = line else {
            continue;
        };
        match key.text {
            "library" => libraries.push(format!("-l{}", file.text)),
            key => {
                copy(&dir.join(file.text), &out.join("lib").join(file.text))?;
                if key == "source" {
                    sources.push(Path::new("lib").join(file.text));
                }
            }
        }
    }
    let built = Command::new("clang")
        .args(["-O2", "-Wall", "-Wextra", "-Werror", "-o", "host", "glue.c"])
        .args(sources)
        .args(libraries)
        .current_dir(out)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| clang::start_failed("", e).message)?;
    match built.status.success() {
        true => Ok(out.join("host")),
        false => Err(format!(
            "clang failed ({}): {}",
            built.status,
            String::from_utf8_lossy(&built.stderr).trim_end()
        )),
    }
}

/// Copies a regular file, never what a link points at.
fn copy(from: &Path, to: &Path) -> Result<(), String> {
    let regular = fs::symlink_metadata(from).is_ok_and(|m| m.is_file());
    let bytes = (fs::read(from).ok())
        .filter(|_| regular)
        .ok_or_else(|| format!("{} is not a regular file", from.display()))?;
    fs::write(to, bytes).map_err(|e| format!("{}: {e}", to.display()))
}

/// The generated half: for function `k`, a line that fails the build unless the header's type
/// is exactly the declared one, and `paka_call_k`, which decodes its arguments and calls it.
fn glue(lines: &[ForeignLine]) -> String {
    let mut c = String::from("#include \"paka_host.c\"\n");
    let mut cases = String::new();
    for line in lines {
        let ForeignLine::File(key, file) = line else {
            continue;
        };
        if key.text == "header" {
            writeln!(c, "#include \"lib/{}\"", file.text).unwrap();
        }
    }
    let functions = lines.iter().filter_map(|line| match line {
        ForeignLine::Function {
            params,
            result,
            symbol,
            ..
        } => Some((params, result, symbol.text)),
        ForeignLine::File(..) => None,
    });
    for (k, (params, result, symbol)) in functions.enumerate() {
        function(&mut c, k, params, result.as_ref(), symbol);
        writeln!(cases, "  case {k}: return paka_call_{k};").unwrap();
    }
    c + "static paka_call paka_function(uint32_t n) {\n  switch (n) {\n"
        + &cases
        + "  default: return NULL;\n  }\n}\n"
}

fn function(c: &mut String, k: usize, params: &[Param], result: Option<&Type>, symbol: &str) {
    let ret = result.map_or("void", |t| plain(t).0);
    let types: Vec<_> = (params.iter())
        .map(|p| match plain(&p.ty) {
            (_, "str") => "const char *, int64_t",
            (ty, _) => ty,
        })
        .collect();
    let types = match types.is_empty() {
        true => "void".into(),
        false => types.join(", "),
    };
    writeln!(c, "static {ret} (*const paka_f{k})({types}) = {symbol};").unwrap();
    writeln!(c, "static paka_rep paka_call_{k}(paka_req *r) {{").unwrap();
    let mut args = vec![];
    for (i, p) in params.iter().enumerate() {
        let (ty, codec) = plain(&p.ty);
        writeln!(c, "  {ty} a{i} = paka_get_{codec}(r);").unwrap();
        args.push(match codec {
            "str" => format!("a{i}.ptr, a{i}.len"),
            _ => format!("a{i}"),
        });
    }
    let call = format!("paka_f{k}({})", args.join(", "));
    c.push_str("  if (!paka_done(r))\n    return paka_refused();\n");
    match result {
        Some(t) => writeln!(c, "  return paka_{}({call});", plain(t).1),
        None => writeln!(c, "  {call};\n  return paka_none();"),
    }
    .unwrap();
    c.push_str("}\n");
}

/// A crossing type's C type and the suffix of its glue codec.
fn plain(t: &Type) -> (&'static str, &'static str) {
    match t {
        Type::Name { name, .. } => match name.text {
            "Int" => ("int64_t", "int"),
            "Float" => ("double", "float"),
            "Bool" => ("bool", "bool"),
            "String" => ("paka_str", "str"),
            other => unreachable!("{other} does not cross; the checker refuses it"),
        },
        Type::Fn { .. } => unreachable!("a function does not cross; the checker refuses it"),
    }
}
