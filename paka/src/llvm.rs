//! The LLVM IR text emitter for the M1 view of the core form, written to docs/abi/m1.md
//! sections 3, 4 and 11. The view is all the back end builds until it takes the whole core form.

use crate::core_form::{self, Expr, ExprKind, Grant, Program, Tag, Type};

/// What the back end builds: the effects of an `update` whose body is a constant list.
#[derive(Debug, PartialEq)]
pub struct M1 {
    pub app: String,
    /// The start variant's position in the message enum.
    pub start: usize,
    pub console: bool,
    /// The Paka name of the update function.
    pub update_name: String,
    /// The effects `update` returns, in order.
    pub update: Vec<Effect>,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Print(String),
    Exit,
}

/// The M1 view of `core`, or `None` when it holds what the back end cannot build.
pub fn view(core: &Program) -> Option<M1> {
    let app = core.app.as_ref()?;
    let update = &core.fns[app.update];
    if app.state.is_some() || !update.body.stmts.is_empty() {
        return None;
    }
    let Type::Enum(msg) = core.types[update.params.last()?.ty] else {
        return None;
    };
    if !core.enums[msg].variants[app.start].fields.is_empty() {
        return None;
    }
    let ExprKind::List(items) = &update.body.value.as_ref()?.kind else {
        return None;
    };
    Some(M1 {
        app: app.name.clone(),
        start: app.start,
        console: app.grants.contains(&Grant::Console),
        update_name: update.name.clone(),
        update: items.iter().map(effect).collect::<Option<_>>()?,
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
    let texts: Vec<&str> = core
        .update
        .iter()
        .filter_map(|e| match e {
            Effect::Print(text) if !text.is_empty() => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let mut constants = constant("name", &core.app);
    for (i, text) in texts.iter().enumerate() {
        constants += &constant(&format!("s{i}"), text);
    }
    constants += &effect_list(&core.update);
    let (list, count) = match core.update.is_empty() {
        true => ("null", 0),
        false => ("@fx", core.update.len()),
    };
    let grants = u64::from(core.console); // bit 0: Console
    let update = format!("p_{}", core.update_name);
    format!(
        "target triple = \"x86_64-pc-linux-gnu\"

{constants}
declare i32 @paka_rt1_run(ptr, i64, ptr, i64, i64)

define internal i32 @{update}(i64 %m, ptr %out) {{
  store {{ ptr, i64 }} {{ ptr {list}, i64 {count} }}, ptr %out
  ret i32 0
}}

define i32 @main() {{
  %st = call i32 @paka_rt1_run(ptr @name, i64 {}, ptr @{update}, i64 {}, i64 {grants})
  ret i32 %st
}}
",
        core.app.len(),
        core.start
    )
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

    const BANNED: [&str; 12] = [
        "nsw", "nuw", "inbounds", "noalias", "fast", "nnan", "ninf", "nsz", "arcp", "contract",
        "afn", "reassoc",
    ];

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// The view of a program the front end accepts and the back end can build.
    fn accepted(file: &Path) -> Option<M1> {
        view(&crate::check("main.paka", &fs::read(file).unwrap()).ok()?)
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

    /// The banned words found as whole words, outside the hex of string constants.
    fn banned_in(ir: &str) -> Vec<&str> {
        let code = ir.split("c\"").enumerate().map(|(i, part)| match i {
            0 => part,
            _ => part.split_once('"').map_or("", |(_, rest)| rest),
        });
        code.flat_map(|part| part.split(|c: char| !c.is_ascii_alphanumeric() && c != '_'))
            .filter(|word| BANNED.contains(word))
            .collect()
    }

    fn core(update: Vec<Effect>) -> M1 {
        M1 {
            app: "app".into(),
            start: 1,
            console: true,
            update_name: "go".into(),
            update,
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
        crate::check("main.paka", source.as_bytes()).unwrap()
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
            update: vec![Effect::Print("let there be light".into()), Effect::Exit],
        };
        assert_eq!(view(&lowered(LIGHT)), Some(by_hand));
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
        let unused = typed(1, ExprKind::Str("unused".into()));
        statement.fns[0]
            .body
            .stmts
            .push(core_form::Stmt::Expr(unused));
        let mut computed = lowered(LIGHT);
        computed.fns[0].body.value = Some(Box::new(typed(2, ExprKind::Hole(0))));
        let mut none = lowered(LIGHT);
        none.app = None;
        let payload = lowered(&LIGHT.replace("Start }", "Start(path: String) }"));
        for core in [state, statement, computed, none, payload] {
            assert_eq!(view(&core), None);
        }
    }

    #[test]
    fn grants_beyond_console_do_not_change_the_view() {
        let mut core = lowered(LIGHT);
        let plain = view(&core);
        core.app.as_mut().unwrap().grants.insert(Grant::ReadFiles);
        assert!(plain.is_some());
        assert_eq!(view(&core), plain);
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
    fn no_banned_word_in_any_run_case() {
        let cases = run_cases();
        assert!(!cases.is_empty(), "no runnable case found");
        for (folder, ir) in cases {
            assert!(!ir.is_empty(), "{}", folder.display());
            assert_eq!(banned_in(&ir), Vec::<&str>::new(), "{}", folder.display());
        }
    }

    #[test]
    fn banned_word_scan_sees_words_and_skips_constants() {
        assert_eq!(banned_in("add nsw i64 %a, %b"), ["nsw"]);
        assert_eq!(banned_in("fadd fast double"), ["fast"]);
        assert!(banned_in("call @faster(i64 1) @inbounds_x").is_empty());
        assert!(banned_in("@s0 = private constant [4 x i8] c\"fast\"").is_empty());
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
}
