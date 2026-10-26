//! The host of a `foreign` declaration (docs/decisions/M7-02.md section 1.6): the pinned clang
//! builds the fixed loop in `foreign/host.c` with C glue generated for the declaration, from
//! copies of its named files in an empty directory. A run starts each declaration's host at its
//! first call and keeps it (section 2).

mod pipe;

pub use pipe::{Failure, Host, Kind, Value};

use crate::ast::{ForeignLine, Type};
use crate::core_form::{CFunction, Library, Plain};
use crate::hash::hex;
use crate::sha256::sha256;
use crate::{clang, results};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::{env, fs, process};

const HOST: &str = include_str!("foreign/host.c");
/// How long a call may take before its host is killed.
pub const LIMIT: Duration = Duration::from_secs(10);

/// The hosts of one run, by declaration, each with the declaration it was built from.
#[derive(Default)]
pub struct Hosts(Vec<Option<(Library, Host)>>);

impl Hosts {
    /// Calls function `number` of declaration `at`, `library`, with `args`: its result, or the
    /// reason it failed. A host that failed is dropped, so the next call starts a fresh one.
    pub fn call(
        &mut self,
        at: usize,
        library: &Library,
        number: usize,
        args: &[Value],
    ) -> Result<Option<Value<'static>>, &'static str> {
        if self.0.len() <= at {
            self.0.resize_with(at + 1, || None);
        }
        let slot = &mut self.0[at];
        if slot.as_ref().is_none_or(|(built, _)| built != library) {
            *slot = None;
            let exe = cached(library, &cache_dir()).map_err(|_| "no host")?;
            let host = Host::start(&exe, LIMIT).map_err(|_| "no host")?;
            *slot = Some((library.clone(), host));
        }
        let (_, host) = slot.as_mut().expect("started");
        let result = library.functions[number].result.map(kind);
        let answer = host.call(number as u32, args, result);
        if answer.is_err() {
            *slot = None;
        }
        answer.map_err(|f| f.reason())
    }
}

/// `$XDG_CACHE_HOME/paka/hosts` as the example cache finds it, else under the temporary directory.
fn cache_dir() -> PathBuf {
    results::cache_dir("hosts").unwrap_or_else(|| env::temp_dir().join("paka-hosts"))
}

/// The host of `library` in `cache`, built on a miss. It is keyed by the declaration, its files'
/// bytes, the fixed C file and the pinned clang; a failed build leaves nothing behind.
pub fn cached(library: &Library, cache: &Path) -> Result<PathBuf, String> {
    let dir = cache.join(key(library)?);
    let exe = dir.join("host");
    if exe.is_file() {
        return Ok(exe);
    }
    static BUILDS: AtomicUsize = AtomicUsize::new(0);
    let n = BUILDS.fetch_add(1, Ordering::Relaxed);
    let fresh = cache.join(format!("build-{}-{n}", process::id()));
    let _ = fs::remove_dir_all(&fresh);
    fs::create_dir_all(cache).map_err(|e| format!("{}: {e}", cache.display()))?;
    let built = build(library, &fresh);
    // A host another run placed first is as good as this one.
    if built.is_err() || fs::rename(&fresh, &dir).is_err() {
        let _ = fs::remove_dir_all(&fresh);
    }
    built.map(|_| exe)
}

fn key(library: &Library) -> Result<String, String> {
    let mut bytes = vec![];
    let mut add = |part: &[u8]| {
        bytes.extend((part.len() as u64).to_le_bytes());
        bytes.extend(part);
    };
    let glue = glue(&library.functions, &library.files);
    for part in [clang::VERSION, HOST, glue.as_str()] {
        add(part.as_bytes());
    }
    for (key, file) in &library.files {
        add(key.as_bytes());
        add(file.as_bytes());
        if key != "library" {
            add(&read_regular(&library.dir.join(file))?);
        }
    }
    Ok(hex(&sha256(&bytes)))
}

/// The declaration `name` of `lines`, its files in `dir`, as the core form holds it.
pub fn library(name: &str, dir: &Path, lines: &[ForeignLine]) -> Library {
    let mut library = Library {
        name: name.into(),
        dir: dir.into(),
        files: vec![],
        functions: vec![],
    };
    for line in lines {
        match line {
            ForeignLine::File(key, file) => {
                (library.files).push((key.text.into(), file.text.into()))
            }
            ForeignLine::Function {
                params,
                result,
                symbol,
                ..
            } => library.functions.push(CFunction {
                symbol: symbol.text.into(),
                params: params.iter().map(|p| plain(&p.ty)).collect(),
                result: result.as_ref().map(plain),
            }),
        }
    }
    library
}

/// Builds the host of `library` inside `out`, which must not exist yet: the executable's path,
/// or why there is none.
pub fn build(library: &Library, out: &Path) -> Result<PathBuf, String> {
    clang::check("").map_err(|d| d.message)?;
    let failed = |e: std::io::Error| format!("{}: {e}", out.display());
    fs::create_dir(out).map_err(failed)?;
    fs::create_dir(out.join("lib")).map_err(failed)?;
    fs::write(out.join("paka_host.c"), HOST).map_err(failed)?;
    let glue = glue(&library.functions, &library.files);
    fs::write(out.join("glue.c"), glue).map_err(failed)?;
    let (mut sources, mut libraries) = (vec![], vec![]);
    for (key, file) in &library.files {
        match key.as_str() {
            "library" => libraries.push(format!("-l{file}")),
            key => {
                let bytes = read_regular(&library.dir.join(file))?;
                fs::write(out.join("lib").join(file), bytes).map_err(failed)?;
                if key == "source" {
                    sources.push(Path::new("lib").join(file));
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

/// The bytes of a regular file, never what a link points at.
fn read_regular(path: &Path) -> Result<Vec<u8>, String> {
    let regular = fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
    (fs::read(path).ok())
        .filter(|_| regular)
        .ok_or_else(|| format!("{} is not a regular file", path.display()))
}

/// The generated half: for function `k`, a line that fails the build unless the header's type
/// is exactly the declared one, and `paka_call_k`, which decodes its arguments and calls it.
fn glue(functions: &[CFunction], files: &[(String, String)]) -> String {
    let mut c = String::from("#include \"paka_host.c\"\n");
    for (_, header) in files.iter().filter(|(key, _)| key == "header") {
        writeln!(c, "#include \"lib/{header}\"").unwrap();
    }
    let mut cases = String::new();
    for (k, f) in functions.iter().enumerate() {
        function(&mut c, k, f);
        writeln!(cases, "  case {k}: return paka_call_{k};").unwrap();
    }
    c + "static paka_call paka_function(uint32_t n) {\n  switch (n) {\n"
        + &cases
        + "  default: return NULL;\n  }\n}\n"
}

fn function(c: &mut String, k: usize, f: &CFunction) {
    let ret = f.result.map_or("void", |t| c_type(t).0);
    let types: Vec<_> = (f.params.iter())
        .map(|&p| match c_type(p) {
            (_, "str") => "const char *, int64_t",
            (ty, _) => ty,
        })
        .collect();
    let types = match types.is_empty() {
        true => "void".into(),
        false => types.join(", "),
    };
    let symbol = &f.symbol;
    writeln!(c, "static {ret} (*const paka_f{k})({types}) = {symbol};").unwrap();
    writeln!(c, "static paka_rep paka_call_{k}(paka_req *r) {{").unwrap();
    let mut args = vec![];
    for (i, &p) in f.params.iter().enumerate() {
        let (ty, codec) = c_type(p);
        writeln!(c, "  {ty} a{i} = paka_get_{codec}(r);").unwrap();
        args.push(match codec {
            "str" => format!("a{i}.ptr, a{i}.len"),
            _ => format!("a{i}"),
        });
    }
    let call = format!("paka_f{k}({})", args.join(", "));
    c.push_str("  if (!paka_done(r))\n    return paka_refused();\n");
    match f.result {
        Some(t) => writeln!(c, "  return paka_{}({call});", c_type(t).1),
        None => writeln!(c, "  {call};\n  return paka_none();"),
    }
    .unwrap();
    c.push_str("}\n");
}

/// A crossing type's C type and the suffix of its glue codec.
fn c_type(t: Plain) -> (&'static str, &'static str) {
    match t {
        Plain::Int => ("int64_t", "int"),
        Plain::Float => ("double", "float"),
        Plain::Bool => ("bool", "bool"),
        Plain::String => ("paka_str", "str"),
    }
}

/// The pipe's kind of a result; a result is never a String.
fn kind(t: Plain) -> Kind {
    match t {
        Plain::Int => Kind::Int,
        Plain::Float => Kind::Float,
        Plain::Bool => Kind::Bool,
        Plain::String => unreachable!("a String result does not cross; the checker refuses it"),
    }
}

fn plain(t: &Type) -> Plain {
    match t {
        Type::Name { name, .. } => match name.text {
            "Int" => Plain::Int,
            "Float" => Plain::Float,
            "Bool" => Plain::Bool,
            "String" => Plain::String,
            other => unreachable!("{other} does not cross; the checker refuses it"),
        },
        Type::Fn { .. } => unreachable!("a function does not cross; the checker refuses it"),
    }
}
