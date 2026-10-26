//! The LLVM IR text emitter for the M1 view of the core form, written to docs/abi/m1.md
//! sections 3, 4 and 11. The view is all the back end builds until it takes the whole core form.

use crate::core_form::{self, Expr, ExprKind, Grant, Program, Tag, Type};
use paka_rt::abi::table::{APP, RUN, VISUAL};

mod code;
pub mod lint;
#[cfg(test)]
mod views;

/// What the back end builds: an `update` whose body gives a list of effects, after statements
/// that may call other functions.
#[derive(Debug, PartialEq)]
pub struct M1 {
    pub app: String,
    /// The start variant's position in the message enum.
    pub start: usize,
    pub console: bool,
    /// The Paka name of the update function.
    pub update_name: String,
    /// The effects `update` returns, in order, when they are constants; `None` when the body
    /// builds them.
    pub update: Option<Vec<Effect>>,
    /// The start of `update`'s body, which leaves its block open, or all of it when it builds its
    /// effects; empty when it has no statements.
    pub body: String,
    /// The other functions with their constants and declarations.
    pub code: String,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Print(String),
    Exit,
}

/// The M1 view of `core`, or `None` when it holds what the back end cannot build.
pub fn view(core: &mut Program) -> Option<M1> {
    let app = core.app.as_ref()?;
    let (start, id) = (app.start?, app.update);
    let update = &core.fns[id];
    if app.state.is_some() {
        return None;
    }
    let Type::Enum(msg) = core.types[update.params.last()?.ty] else {
        return None;
    };
    if !core.enums[msg].variants[start].fields.is_empty() {
        return None;
    }
    let literal = match &update.body.value.as_ref()?.kind {
        ExprKind::List(items) => items.iter().map(effect).collect::<Option<Vec<_>>>(),
        _ => None,
    };
    let (body, code) = code::lower(core, id, literal.is_none())?;
    let (app, update) = (core.app.as_ref()?, &core.fns[id]);
    Some(M1 {
        app: app.name.clone(),
        start,
        console: app.grants.contains(&Grant::Console),
        update_name: update.name.clone(),
        update: literal,
        body,
        code,
    })
}

fn effect(e: &Expr) -> Option<Effect> {
    match &e.kind {
        ExprKind::Variant(Tag::Effect(core_form::Effect::Exit), _) => Some(Effect::Exit),
        ExprKind::Variant(Tag::Effect(core_form::Effect::Print), fields) => match &fields[..] {
            [
                Expr {
                    kind: ExprKind::Str(text),
                    ..
                },
            ] => Some(Effect::Print(text.clone())),
            _ => None,
        },
        _ => None,
    }
}

/// The IR text of the program; layout and wording follow docs/abi/m1.md section 11.
pub fn emit(core: &M1) -> String {
    let effects = core.update.as_deref().unwrap_or_default();
    let texts = effects.iter().filter_map(|e| match e {
        Effect::Print(text) if !text.is_empty() => Some(text.as_str()),
        _ => None,
    });
    let mut constants = constant("name", &core.app);
    for (i, text) in texts.enumerate() {
        constants += &constant(&format!("s{i}"), text);
    }
    constants += &effect_list(effects);
    let (list, count) = match effects.is_empty() {
        true => ("null", 0),
        false => ("@fx", effects.len()),
    };
    let body = match core.update {
        Some(_) => format!(
            "{}  store {{ ptr, i64 }} {{ ptr {list}, i64 {count} }}, ptr %out\n  ret i32 0\n",
            core.body
        ),
        None => core.body.clone(),
    };
    let grants = u64::from(core.console); // bit 0: Console
    let update = format!("p_{}", core.update_name);
    format!(
        "target triple = \"x86_64-pc-linux-gnu\"

{constants}
{RUN}

define internal i32 @{update}(i64 %m, ptr %out) {{
{body}}}

define i32 @main() {{
  %st = call i32 @paka_rt1_run(ptr @name, i64 {}, ptr @{update}, i64 {}, i64 {grants})
  ret i32 %st
}}
{}",
        core.app.len(),
        core.start,
        core.code
    )
}

/// The IR text of an app the M1 view cannot hold, on the entry of docs/abi/m1.md sections 13
/// and 14; `None` for one granted foreign effects, or what the back end cannot build.
pub fn app(core: &mut Program) -> Option<String> {
    let app = core.app.as_ref()?;
    if !app.foreign.is_empty() {
        return None;
    }
    let (entry, name) = match app.view {
        Some(_) => (VISUAL, "paka_rt1_visual"),
        None => (APP, "paka_rt1_app"),
    };
    let code = code::lower_app(core)?;
    Some(format!(
        "target triple = \"x86_64-pc-linux-gnu\"

{entry}

define i32 @main() {{
  %st = call i32 @{name}(ptr @app)
  ret i32 %st
}}

{code}"
    ))
}

/// The IR text of `core`: the M1 view's when it holds the program, else the headless app's.
pub fn ir(core: &mut Program) -> Option<String> {
    match view(core) {
        Some(view) => Some(emit(&view)),
        None => app(core),
    }
}

/// A byte-string constant, every byte hex-escaped.
fn constant(name: &str, text: &str) -> String {
    let hex: String = text.bytes().map(|b| format!("\\{b:02X}")).collect();
    format!(
        "@{name} = private constant [{} x i8] c\"{hex}\"\n",
        text.len()
    )
}

/// The `@fx` constant; empty for an empty list.
fn effect_list(effects: &[Effect]) -> String {
    let mut next_text = 0;
    let records: Vec<String> = effects
        .iter()
        .map(|effect| {
            let fields = match effect {
                Effect::Print(text) if !text.is_empty() => {
                    next_text += 1;
                    format!("i64 0, ptr @s{}, i64 {}", next_text - 1, text.len())
                }
                Effect::Print(_) => "i64 0, ptr null, i64 0".into(),
                Effect::Exit => "i64 1, ptr null, i64 0".into(),
            };
            format!("  {{ i64, ptr, i64 }} {{ {fields} }}")
        })
        .collect();
    match records.is_empty() {
        true => String::new(),
        false => format!(
            "@fx = private constant [{} x {{ i64, ptr, i64 }}] [\n{} ]\n",
            records.len(),
            records.join(",\n")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// The view of a program the front end accepts and the back end can build.
    fn accepted(file: &Path) -> Option<M1> {
        view(
            &mut crate::check(
                "main.paka",
                &crate::source::Source::file("main.paka", fs::read(file).unwrap()),
            )
            .ok()?
            .0,
        )
    }

    /// The IR of every runnable case the front end accepts.
    fn run_cases() -> Vec<(PathBuf, String)> {
        let mut folders: Vec<_> = fs::read_dir(root().join("tests/cases/run"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        folders.sort();
        folders
            .into_iter()
            .filter_map(|f| Some((f.clone(), emit(&accepted(&f.join("main.paka"))?))))
            .collect()
    }

    fn clang_accepts(ir: &str) -> Result<(), String> {
        let mut clang = Command::new("clang")
            .args(["-c", "-x", "ir", "-", "-o", "/dev/null"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("clang (version 21) must be on PATH to run the emitter tests");
        let _ = clang.stdin.take().unwrap().write_all(ir.as_bytes());
        let out = clang.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        match out.status.success() && stderr.is_empty() {
            true => Ok(()),
            false => Err(format!("{}: {stderr}", out.status)),
        }
    }

    fn core(update: Vec<Effect>) -> M1 {
        M1 {
            app: "app".into(),
            start: 1,
            console: true,
            update_name: "go".into(),
            update: Some(update),
            body: String::new(),
            code: String::new(),
        }
    }

    fn print(text: &str) -> Effect {
        Effect::Print(text.into())
    }

    const LIGHT: &str = "enum Msg { Start }

fn update(m: Msg) -> List<Effect<Msg>> {
  [Print(\"let there be light\"), Exit]
}

app light {
  start:  Start
  update: update
  grants: [Console]
}
";

    fn lowered(source: &str) -> Program {
        crate::check(
            "main.paka",
            &crate::source::Source::file("main.paka", source.into()),
        )
        .unwrap()
        .0
    }

    fn typed(ty: usize, kind: ExprKind) -> Expr {
        Expr { ty: Some(ty), kind }
    }

    #[test]
    fn light_reaches_the_back_end_as_before() {
        let by_hand = M1 {
            app: "light".into(),
            start: 0,
            console: true,
            update_name: "update".into(),
            update: Some(vec![print("let there be light"), Effect::Exit]),
            body: String::new(),
            code: String::new(),
        };
        assert_eq!(view(&mut lowered(LIGHT)), Some(by_hand));
    }

    #[test]
    fn view_refuses_what_the_back_end_cannot_build() {
        let mut state = lowered(LIGHT);
        state.records.push(core_form::Record {
            name: "State".into(),
            fields: vec![],
        });
        let ty = state.ty(Type::Record(0));
        state.app.as_mut().unwrap().state = Some(typed(ty, ExprKind::Record(0, vec![])));
        let mut statement = lowered(LIGHT);
        let unused = typed(1, ExprKind::Hole(0));
        statement.fns[0]
            .body
            .stmts
            .push(core_form::Stmt::Expr(unused));
        let mut computed = lowered(LIGHT);
        computed.fns[0].body.value = Some(Box::new(typed(2, ExprKind::Hole(0))));
        let mut none = lowered(LIGHT);
        none.app = None;
        let payload = lowered(&LIGHT.replace("Start }", "Start(path: String) }"));
        for mut core in [state, statement, computed, none, payload] {
            assert_eq!(view(&mut core), None);
        }
    }

    #[test]
    fn grants_beyond_console_do_not_change_the_view() {
        let mut core = lowered(LIGHT);
        let plain = view(&mut core);
        core.app.as_mut().unwrap().grants.insert(Grant::ReadFiles);
        assert!(plain.is_some());
        assert_eq!(view(&mut core), plain);
    }

    #[test]
    fn light_matches_the_abi_document() {
        let light = accepted(&root().join("examples/light.paka")).unwrap();
        let reference = fs::read_to_string(root().join("docs/abi/light.ll")).unwrap();
        let (comment, rest) = reference.split_once('\n').unwrap();
        assert!(comment.starts_with(';'));
        assert_eq!(emit(&light), rest);
    }

    #[test]
    fn clang_accepts_every_run_case() {
        let cases = run_cases();
        assert!(!cases.is_empty(), "no runnable case found");
        let failures: Vec<_> = cases
            .iter()
            .filter_map(|(f, ir)| Some(format!("{}: {}", f.display(), clang_accepts(ir).err()?)))
            .collect();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn every_run_case_and_the_abi_document_pass_the_linter() {
        let cases = run_cases();
        assert!(!cases.is_empty(), "no runnable case found");
        let reference = fs::read_to_string(root().join("docs/abi/light.ll")).unwrap();
        let document = (root().join("docs/abi/light.ll"), reference);
        for (file, ir) in cases.into_iter().chain([document]) {
            assert!(!ir.is_empty(), "{}", file.display());
            assert_eq!(lint::lint(&ir), Vec::<String>::new(), "{}", file.display());
        }
    }

    /// Release IR has no slot table: every call names its callee, and the only global a load
    /// reads is the runtime's stack limit.
    #[test]
    fn every_call_in_any_run_case_is_direct() {
        let cases = run_cases();
        assert!(!cases.is_empty(), "no runnable case found");
        for (folder, ir) in cases {
            let calls = ir
                .lines()
                .filter_map(|line| line.split_once(" call ")?.1.split_once(' '));
            for (_, callee) in calls {
                assert!(callee.starts_with('@'), "{}: {callee}", folder.display());
            }
            for line in ir.lines().filter(|l| l.contains(" = load ")) {
                let at = line
                    .rsplit_once("ptr ")
                    .unwrap()
                    .1
                    .split(',')
                    .next()
                    .unwrap();
                let global = at.starts_with('@');
                let limit = at == "@paka_rt1_stack_limit";
                assert!(!global || limit, "{}: {line}", folder.display());
            }
        }
    }

    #[test]
    fn string_bytes_are_hex_escaped() {
        let ir = emit(&core(vec![print("a\"b\\c\n \u{e9}"), Effect::Exit]));
        let want = "@s0 = private constant [9 x i8] c\"\\61\\22\\62\\5C\\63\\0A\\20\\C3\\A9\"\n";
        assert!(ir.contains(want), "{ir}");
        assert_eq!(clang_accepts(&ir), Ok(()));
    }

    #[test]
    fn empty_text_takes_no_constant_number() {
        let ir = emit(&core(vec![print("a"), print(""), print("b")]));
        assert!(ir.contains("c\"\\61\"\n@s1 = private constant [1 x i8] c\"\\62\"\n"));
        assert!(ir.contains("  { i64, ptr, i64 } { i64 0, ptr @s0, i64 1 },\n"));
        assert!(ir.contains("  { i64, ptr, i64 } { i64 0, ptr null, i64 0 },\n"));
        assert!(ir.contains("  { i64, ptr, i64 } { i64 0, ptr @s1, i64 1 } ]\n"));
        assert!(!ir.contains("@s2"));
    }

    #[test]
    fn empty_list_has_no_fx_and_stores_null() {
        let mut empty = core(vec![]);
        empty.console = false;
        let ir = emit(&empty);
        assert!(!ir.contains("@fx"));
        assert!(ir.contains("  store { ptr, i64 } { ptr null, i64 0 }, ptr %out\n"));
        assert!(ir.contains("define internal i32 @p_go(i64 %m, ptr %out) {\n"));
        assert!(ir.contains("(ptr @name, i64 3, ptr @p_go, i64 1, i64 0)\n"));
        assert_eq!(clang_accepts(&ir), Ok(()));
    }

    /// Every defined function checks its frame against the runtime's limit before anything else;
    /// only a large frame is kept from being inlined.
    #[test]
    fn every_function_checks_the_stack_and_a_large_frame_is_not_inlined() {
        let fields: Vec<String> = (0..1024).map(|i| format!("f{i}: Int")).collect();
        let values: Vec<String> = (0..1024).map(|i| format!("f{i}: n")).collect();
        let source = format!(
            "enum Msg {{ Start }}\ntype Big {{ {} }}\n\
             fn down(n: Int) -> Int {{ down(n + 1) + 1 }}\n\
             fn wide(n: Int) -> Int {{ let b = Big {{ {} }}\n b.f0 + down(n) }}\n\
             fn update(m: Msg) -> List<Effect<Msg>> {{ let r = wide(0)\n [Exit] }}\n\
             app a {{\n start: Start\n update: update\n grants: []\n}}\n",
            fields.join(", "),
            values.join(", ")
        );
        let ir = emit(&view(&mut lowered(&source)).unwrap());
        assert_eq!(lint::lint(&ir), Vec::<String>::new());
        assert_eq!(clang_accepts(&ir), Ok(()));
        let defines: Vec<_> = ir.split("\ndefine internal i32 @p_").skip(2).collect();
        assert_eq!(defines.len(), 2, "{ir}");
        for define in defines {
            let (head, body) = define.split_once("\nentry:\n").unwrap();
            let first = body.lines().find(|l| !l.contains(" = alloca ")).unwrap();
            assert_eq!(first, "  %mark.at = ptrtoint ptr %mark to i64", "{define}");
            let large = head.starts_with("wide(");
            assert_eq!(head.ends_with(" noinline {"), large, "{head}");
            let frame = if large { "16400" } else { "16" };
            assert!(
                body.contains(&format!("%room = sub i64 %mark.at, {frame}\n")),
                "{body}"
            );
        }
    }

    /// A loop over a `List<Float>` through the generic rows keeps its items as doubles at full
    /// optimization: no call through a pointer, no item copied by the runtime.
    #[test]
    fn a_generic_loop_over_floats_is_specialized() {
        let source = "enum Msg { Start }
fn scaled(xs: List<Float>, k: Float) -> List<Float> {
  var out: List<Float> = []
  for x in xs { append(&out, x * k) }
  sort(&out)
  out
}
fn update(m: Msg) -> List<Effect<Msg>> {
  var ys = scaled(list_of(1.5, count: 1000), 2.0)
  [Print(\"{unwrap_or(find(ys, 3.0), -1)} {to_int(pop(&ys))}\"), Exit]
}
app a {
  start: Start
  update: update
  grants: [Console]
}
";
        let ir = emit(&view(&mut lowered(source)).unwrap());
        assert_eq!(lint::lint(&ir), Vec::<String>::new());
        let mut clang = Command::new("clang")
            .args(["-O3", "-S", "-emit-llvm", "-x", "ir", "-", "-o", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = clang.stdin.take().unwrap().write_all(ir.as_bytes());
        let out = clang.wait_with_output().unwrap();
        assert!(out.status.success());
        let optimized = String::from_utf8(out.stdout).unwrap();
        let calls = optimized
            .lines()
            .filter_map(|line| line.split_once(" call "));
        for (_, call) in calls {
            let callee = call.split('(').next().unwrap().rsplit(' ').next().unwrap();
            assert!(callee.starts_with('@'), "{call}");
        }
        assert!(!optimized.contains("@paka_rt1_copy"), "{optimized}");
        assert!(optimized.contains("store double"), "{optimized}");
    }
}
