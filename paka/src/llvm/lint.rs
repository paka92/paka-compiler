//! The release IR linter: the promise table of docs/decisions/M6-01.md as an allow-list. Every
//! keyword, attribute, metadata name and intrinsic in the IR Paka writes must be a row or a plain
//! word, every use of an `inbounds` result must be a load or store address, no store may target a
//! global, an instruction undefined on some inputs must take the operand its check made, and only
//! intrinsics and the runtime's entries, line for line as its table gives them, are declared, so no
//! call names a maths function LLVM can fold.

use paka_rt::abi::table::ENTRIES;

/// The approved rows (M6-01 section 2). An entry starting `@llvm.` names an intrinsic and any of
/// its typed forms, such as `@llvm.expect.i1`.
const ROWS: &[&str] = &[
    "nounwind",
    "inbounds",
    "align",
    "!prof",
    "@llvm.expect",
    "cold",
    "hot",
    "noinline",
    "inlinehint",
    "alwaysinline",
    "internal",
    "private",
    "unnamed_addr",
    "constant",
];

/// Words that promise nothing (M6-01 section 3). Integer types `iN` are plain too.
const PLAIN: &[&str] = &[
    // Module structure and types.
    "target",
    "triple",
    "datalayout",
    "source_filename",
    "define",
    "declare",
    "attributes",
    "type",
    "void",
    "ptr",
    "double",
    "label",
    "x",
    "to",
    "null",
    "zeroinitializer",
    "true",
    "false",
    // Integer and float arithmetic and comparison.
    "add",
    "sub",
    "mul",
    "and",
    "or",
    "xor",
    "icmp",
    "eq",
    "ne",
    "slt",
    "sle",
    "sgt",
    "sge",
    "ult",
    "ule",
    "ugt",
    "uge",
    "fadd",
    "fsub",
    "fmul",
    "fdiv",
    "fneg",
    "fcmp",
    "oeq",
    "one",
    "olt",
    "ole",
    "ogt",
    "oge",
    "ord",
    "uno",
    "ueq",
    "une",
    // Control, aggregates, casts and memory.
    "select",
    "phi",
    "br",
    "switch",
    "ret",
    "getelementptr",
    "extractvalue",
    "insertvalue",
    "zext",
    "sext",
    "trunc",
    "sitofp",
    "ptrtoint",
    "load",
    "store",
    "call",
    "alloca",
    // Intrinsics.
    "@llvm.sadd.with.overflow",
    "@llvm.ssub.with.overflow",
    "@llvm.smul.with.overflow",
    "@llvm.memset",
    "@llvm.memmove",
    "@llvm.sqrt",
    "@llvm.floor",
    "@llvm.ceil",
    "@llvm.trunc",
    "@llvm.round",
    "@llvm.fabs",
];

/// Instructions undefined on some inputs, written only by the helpers that check first (M6-01
/// section 3).
const GUARDED: &[&str] = &["sdiv", "srem", "shl", "ashr", "fptosi"];

/// Whether `def` is how the helper of `word` makes its checked operand: a divisor selected to 1
/// where it is 0 or -1, a count masked to 0 to 63, a Float selected to 0.0 where it is out of range.
fn checked(word: &str, def: &str) -> bool {
    match word {
        "sdiv" | "srem" => def.starts_with("select i1 ") && def.contains(", i64 1, i64 "),
        "fptosi" => def.starts_with("select i1 ") && def.contains(", double 0.0, double "),
        _ => def.starts_with("and i64 ") && def.ends_with(", 63"),
    }
}

/// The checked operand of a guarded instruction: the last of `sdiv`, `srem`, `shl` and `ashr`, the
/// only one of `fptosi`.
fn operand<'a>(word: &str, inst: &'a str) -> Option<&'a str> {
    let rest = inst.strip_prefix(word)?;
    let operand = match word {
        "fptosi" => rest.strip_prefix(" double ")?.split_once(" to ")?.0,
        _ => rest.strip_prefix(" i64 ")?.split_once(", ")?.1,
    };
    Some(operand.trim())
}

/// Whether `word` is a guarded instruction whose last operand is made, earlier in `body`, in the
/// shape its check gives.
fn guarded(word: &str, inst: &str, body: &[(usize, &str)]) -> bool {
    if !GUARDED.contains(&word) {
        return false;
    }
    operand(word, inst).is_some_and(|operand| {
        let defines = |line: &str| {
            let (result, def) = line.trim().split_once(" = ")?;
            Some(result == operand && checked(word, def))
        };
        body.iter().any(|(_, line)| defines(line) == Some(true))
    })
}

/// What in `ir` breaks the table, one line per finding; empty when the IR passes.
pub fn lint(ir: &str) -> Vec<String> {
    let mut found = vec![];
    let mut body = vec![];
    for (n, line) in (1..).zip(ir.lines()) {
        if ENTRIES.contains(&line) {
            continue;
        }
        let words = keywords(line);
        let inst = instruction(line);
        let mut say = |why: String| found.push(format!("line {n}: {why}"));
        for word in words
            .iter()
            .filter(|w| !allowed(w) && !guarded(w, inst, &body))
        {
            let why = match GUARDED.contains(word) {
                true => "is not behind its check",
                false => "is not on the allow-list",
            };
            say(format!("`{word}` {why}"));
        }
        if words.contains(&"inbounds")
            && !(named(line) && inst.starts_with("getelementptr inbounds "))
        {
            say("`inbounds` only on a getelementptr whose result is named".into());
        }
        if inst.starts_with("store ") && address(inst).is_none_or(|a| a.starts_with('@')) {
            say("a store targets a global".into());
        }
        if line.starts_with("declare ") && !line.contains(" @llvm.") {
            say(
                "a declaration is neither a runtime entry as the runtime gives it nor an intrinsic"
                    .into(),
            );
        }
        if line.starts_with("define ") {
            body.clear();
        }
        body.push((n, line));
        if line == "}" {
            found.extend(inbounds_uses(&body));
        }
    }
    found
}

fn allowed(word: &str) -> bool {
    let integer = word
        .strip_prefix('i')
        .is_some_and(|bits| !bits.is_empty() && bits.bytes().all(|b| b.is_ascii_digit()));
    let listed = |entry: &&str| {
        *entry == word
            || entry.starts_with("@llvm.")
                && word.strip_prefix(entry).is_some_and(|t| t.starts_with('.'))
    };
    integer || ROWS.iter().chain(PLAIN).any(listed)
}

/// The keyword tokens of one line: bare words, metadata names such as `!prof`, and intrinsic
/// names. Strings, comments, numbers, labels and the program's own `%`, `@` and `#` names are
/// skipped.
fn keywords(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let run = |mut i: usize, more: fn(u8) -> bool| {
        while i < b.len() && more(b[i]) {
            i += 1;
        }
        i
    };
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let start = i;
        match b[i] {
            b';' => break,
            b'"' => i = run(i + 1, |c| c != b'"') + 1,
            sigil @ (b'%' | b'@' | b'!' | b'#') => {
                i = run(i + 1, name);
                let text = &line[start..i];
                let keyword = match sigil {
                    b'@' => text.starts_with("@llvm."),
                    b'!' => text[1..].starts_with(|c: char| c.is_ascii_alphabetic()),
                    _ => false,
                };
                if keyword {
                    out.push(text);
                }
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                i = run(i, word);
                // `c"..."` is a string and `entry:` a label.
                if !matches!(b.get(i), Some(b'"' | b':')) {
                    out.push(&line[start..i]);
                }
            }
            c if c.is_ascii_digit() => i = run(i, word),
            _ => i += 1,
        }
    }
    out
}

fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.'
}

/// A byte of a `%`, `@`, `!` or `#` name.
fn name(c: u8) -> bool {
    word(c) || c == b'-' || c == b'$'
}

/// The line's instruction, after any `%name = `.
fn instruction(line: &str) -> &str {
    let line = line.trim();
    match line.split_once(" = ") {
        Some((result, inst)) if result.starts_with('%') => inst,
        _ => line,
    }
}

fn named(line: &str) -> bool {
    line.trim().starts_with('%') && line.contains(" = ")
}

/// The address operand of a load, a store or a getelementptr: the last `ptr` operand.
fn address(inst: &str) -> Option<&str> {
    let (_, rest) = inst.rsplit_once(", ptr ")?;
    Some(rest.split(',').next()?.trim())
}

/// Every use of an `inbounds` result in one function that is not the address of a load, a store
/// or another `inbounds` getelementptr, the operand once on its line.
fn inbounds_uses(body: &[(usize, &str)]) -> Vec<String> {
    let results = body.iter().filter_map(|(_, line)| {
        let inst = instruction(line);
        let result = line.trim().split_once(" = ")?.0;
        (named(line) && inst.starts_with("getelementptr inbounds ")).then_some(result)
    });
    let mut found = vec![];
    for result in results {
        for (n, line) in body {
            let defines = line.trim().starts_with(&format!("{result} = "));
            let uses = uses(line, result);
            if defines || uses == 0 {
                continue;
            }
            let inst = instruction(line);
            let access = ["load ", "store ", "getelementptr inbounds "]
                .iter()
                .any(|op| inst.starts_with(op));
            if !(access && uses == 1 && address(inst) == Some(result)) {
                found.push(format!(
                    "line {n}: `inbounds` result {result} is used other than as an address"
                ));
            }
        }
    }
    found
}

/// How often `local` appears on `line` as a whole name.
fn uses(line: &str, local: &str) -> usize {
    let after = |at: usize| line.as_bytes().get(at + local.len()).copied();
    let whole = |(at, _): &(usize, &str)| !after(*at).is_some_and(name);
    line.match_indices(local).filter(whole).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IR that uses every row and many plain words as the table allows.
    const APPROVED: &str = r#"target triple = "x86_64-pc-linux-gnu"

@s0 = private unnamed_addr constant [4 x i8] c"fast" ; nsw tail in a comment

declare void @paka_rt1_trap(ptr, i64) cold nounwind
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64) nounwind
declare i1 @llvm.expect.i1(i1, i1) nounwind
declare double @llvm.sqrt.f64(double) nounwind
declare void @llvm.memmove.p0.p0.i64(ptr, ptr, i64, i1) nounwind

define internal i32 @p_sum(ptr %data, i64 %i, i64 %len, ptr %out) #0 {
entry:
  %slot = alloca i64, align 8
  %ok = icmp ult i64 %i, %len
  %hint = call i1 @llvm.expect.i1(i1 %ok, i1 true)
  br i1 %hint, label %in, label %trap7, !prof !0
in:
  %p = getelementptr inbounds double, ptr %data, i64 %i
  %q = getelementptr inbounds double, ptr %p, i64 1
  %x = load double, ptr %q, align 8
  %r = call double @llvm.sqrt.f64(double %x)
  store double %r, ptr %p, align 8
  %pair = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %i, i64 1)
  %sum = extractvalue { i64, i1 } %pair, 0
  store { ptr, i64 } { ptr @s0, i64 4 }, ptr %out, align 8
  call void @llvm.memmove.p0.p0.i64(ptr %out, ptr @s0, i64 4, i1 false)
  ret i32 0
trap7:
  ret i32 3
}

define internal i32 @p_other(ptr %data) cold noinline {
  call void @paka_rt1_trap(ptr %data, i64 1)
  ret i32 1
}

attributes #0 = { nounwind inlinehint }
!0 = !{!"branch_weights", i32 2000, i32 1}
"#;

    /// One line breaking the table, inside a function that is otherwise fine.
    fn function(line: &str) -> String {
        format!(
            "@g = private constant i64 0\n\
             define internal i32 @f(ptr %q, i64 %a, i64 %b, double %x, double %y, i1 %c) {{\n\
             {line}\n  ret i32 0\n}}\n"
        )
    }

    fn names(ir: &str, word: &str) -> bool {
        lint(ir).iter().any(|f| f.contains(&format!("`{word}`")))
    }

    #[test]
    fn approved_ir_passes() {
        assert_eq!(lint(APPROVED), Vec::<String>::new());
    }

    #[test]
    fn every_unapproved_flag_or_attribute_fails() {
        let cases = [
            ("nsw", "%r = add nsw i64 %a, %b"),
            ("nuw", "%r = mul nuw i64 %a, %b"),
            ("exact", "%r = sdiv exact i64 %a, %b"),
            ("disjoint", "%r = or disjoint i64 %a, %b"),
            ("nneg", "%r = zext nneg i32 1 to i64"),
            ("samesign", "%r = icmp samesign ult i64 %a, %b"),
            ("nusw", "%p = getelementptr nusw i8, ptr %q, i64 1"),
            ("undef", "%r = add i64 undef, 1"),
            ("poison", "%r = add i64 poison, 1"),
            ("unreachable", "unreachable"),
            ("@llvm.assume", "call void @llvm.assume(i1 %c)"),
            ("tail", "%r = tail call i32 @f(ptr %q)"),
            ("musttail", "%r = musttail call i32 @f(ptr %q)"),
            ("invoke", "invoke void @g() to label %a unwind label %b"),
            ("asm", "call void asm sideeffect \"nop\", \"\"()"),
            ("!tbaa", "store i64 1, ptr %q, !tbaa !0"),
            ("noalias", "%r = call i32 @f(ptr noalias %q)"),
            ("range", "%r = call range(i64 0, 5) i64 @h()"),
            ("nonnull", "%r = call nonnull ptr @h()"),
            ("noundef", "%r = call noundef i64 @h()"),
            (
                "dereferenceable",
                "%r = call i32 @f(ptr dereferenceable(8) %q)",
            ),
            ("readonly", "%r = call i32 @f(ptr readonly %q)"),
            ("readnone", "%r = call i32 @f(ptr readnone %q)"),
            ("memory", "%r = call i32 @f(ptr %q) memory(none)"),
            ("captures", "%r = call i32 @f(ptr captures(none) %q)"),
            (
                "@llvm.lifetime.start.p0",
                "call void @llvm.lifetime.start.p0(i64 8, ptr %q)",
            ),
            ("fast", "%r = fadd fast double %x, %y"),
            ("nnan", "%r = fadd nnan double %x, %y"),
            ("ninf", "%r = fsub ninf double %x, %y"),
            ("nsz", "%r = fmul nsz double %x, %y"),
            ("arcp", "%r = fdiv arcp double %x, %y"),
            ("contract", "%r = fmul contract double %x, %y"),
            ("afn", "%r = fadd afn double %x, %y"),
            ("reassoc", "%r = fadd reassoc double %x, %y"),
            (
                "@llvm.fmuladd.f64",
                "%r = call double @llvm.fmuladd.f64(double %x, double %y, double %x)",
            ),
            (
                "@llvm.fma.f64",
                "%r = call double @llvm.fma.f64(double %x, double %y, double %x)",
            ),
            ("global", "@h = internal global i64 0"),
        ];
        for (word, line) in cases {
            assert!(names(&function(line), word), "{word} passed in {line:?}");
        }
        let attributes = "attributes #0 = { mustprogress willreturn }";
        assert!(names(attributes, "mustprogress") && names(attributes, "willreturn"));
    }

    #[test]
    fn m1_words_pass_only_where_the_table_allows() {
        let load = "  %p = getelementptr inbounds i64, ptr %q, i64 %a\n  %v = load i64, ptr %p";
        assert_eq!(lint(&function(load)), Vec::<String>::new());
        let m1 = [
            "nsw", "nuw", "noalias", "fast", "nnan", "ninf", "nsz", "arcp", "contract", "afn",
            "reassoc",
        ];
        for word in m1 {
            assert!(!allowed(word), "{word}");
        }
        let misuses = [
            "  %p = getelementptr inbounds i64, ptr %q, i64 %a\n  %r = call i32 @f(ptr %p)",
            "  %p = getelementptr inbounds i64, ptr %q, i64 %a\n  store ptr %p, ptr %q",
            "  %p = getelementptr inbounds i64, ptr %q, i64 %a\n  store ptr %p, ptr %p",
            "  %p = getelementptr inbounds i64, ptr %q, i64 %a\n  %s = getelementptr i8, ptr %p, i64 1",
            "  %p = getelementptr inbounds i64, ptr %q, i64 %a\n  %s = phi ptr [ %p, %x ]",
            "  %v = load i64, ptr getelementptr inbounds (i64, ptr @g, i64 1)",
            "  getelementptr inbounds i64, ptr %q, i64 %a",
        ];
        for ir in misuses {
            assert!(!lint(&function(ir)).is_empty(), "{ir}");
        }
    }

    #[test]
    fn division_and_shifts_pass_only_on_the_operand_their_check_made() {
        let checked = [
            "  %d = select i1 %c, i64 1, i64 %b\n  %r = sdiv i64 %a, %d\n  %s = srem i64 %a, %d",
            "  %k = and i64 %b, 63\n  %r = shl i64 %a, %k\n  %s = ashr i64 %a, %k",
        ];
        for ir in checked {
            assert_eq!(lint(&function(ir)), Vec::<String>::new(), "{ir}");
        }
        let unchecked = [
            ("sdiv", "%r = sdiv i64 %a, %b"),
            ("srem", "%r = srem i64 %a, 7"),
            ("shl", "%r = shl i64 %a, %b"),
            ("ashr", "%k = and i64 %b, 31\n  %r = ashr i64 %a, %k"),
            (
                "sdiv",
                "%d = select i1 %c, i64 2, i64 %b\n  %r = sdiv i64 %a, %d",
            ),
            (
                "shl",
                "%d = select i1 %c, i64 1, i64 %b\n  %r = shl i64 %a, %d",
            ),
            (
                "sdiv",
                "%v = load i64, ptr getelementptr (i8, ptr @g, i64 sdiv (i64 1, i64 0))",
            ),
        ];
        for (word, ir) in unchecked {
            assert!(names(&function(ir), word), "{word} passed in {ir:?}");
        }
    }

    #[test]
    fn to_int_passes_only_on_the_operand_its_check_made() {
        let checked = "  %s = select i1 %c, double 0.0, double %x\n  %r = fptosi double %s to i64";
        assert_eq!(lint(&function(checked)), Vec::<String>::new());
        for ir in [
            "%r = fptosi double %x to i64",
            "%s = select i1 %c, double 1.0, double %x\n  %r = fptosi double %s to i64",
            "%s = select i1 %c, double 0.0, double %x\n  %r = fptosi double %x to i64",
        ] {
            assert!(names(&function(ir), "fptosi"), "passed: {ir:?}");
        }
    }

    #[test]
    fn only_runtime_entries_and_intrinsics_are_declared() {
        assert!(lint("declare double @llvm.floor.f64(double)").is_empty());
        assert!(lint("declare void @paka_rt1_trap(ptr, i64) cold nounwind").is_empty());
        for line in [
            "declare double @sqrt(double)",
            "declare double @floor(double) nounwind",
            "declare double @pow(double, double)",
            "declare void @paka_rt1_trap(ptr, i64)",
            "declare i32 @paka_rt1_list_grow(ptr, i64) nounwind",
        ] {
            assert!(
                lint(line).iter().any(|f| f.contains("declaration")),
                "{line}"
            );
        }
    }

    #[test]
    fn no_store_targets_a_global() {
        for line in [
            "store i64 1, ptr @g",
            "store i64 1, ptr @g, align 8",
            "store i64 1, ptr getelementptr (i8, ptr @g, i64 1)",
        ] {
            let found = lint(&function(line));
            assert!(
                found.iter().any(|f| f.contains("global")),
                "{line}: {found:?}"
            );
        }
        assert!(lint(&function("store ptr @g, ptr %q")).is_empty());
    }

    #[test]
    fn names_strings_comments_and_labels_are_not_keywords() {
        let line = r#"%fast = call i32 @nsw_f(ptr @"tail x", ptr %q) ; fast"#;
        assert_eq!(keywords(line), ["call", "i32", "ptr", "ptr"]);
        assert_eq!(keywords("inbounds_x:"), Vec::<&str>::new());
        assert_eq!(
            keywords("!0 = !{!\"nsw\", double 1.0e+00, i64 0x1F}"),
            ["double", "i64"]
        );
    }
}
