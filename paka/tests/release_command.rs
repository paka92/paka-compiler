//! `paka release` (M7-12, docs/decisions/M7-03.md sections 1.3, 1.5 and 2): one read, the gate
//! for that one program, one build, its human-owned examples against the binary, and an artefact
//! whose `RELEASE` holds every hash. Each refusal exits 1 and leaves nothing at `-o`. Keys are
//! made without a passphrase in each test's scratch directory.

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch, build_runtime};
use paka::approvals::{Record, Store, records};
use paka::hash::hex;
use paka::sha256::sha256;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

/// A visual counter: `bumps` is a function example, `inc_then_dec` a UI one, and `agent_guess`
/// is left out of the store.
const COUNTER: &str = r##"type Counter { count: Int = 0 }

enum Msg { Inc, Dec }

fn update(s: inout Counter, m: Msg) {
  match m {
    Inc => s.count += 1
    Dec => s.count = max(0, s.count - 1)
  }
}

fn bump(n: Int) -> Int { n + 4242 }

view counter_view(s: Counter) -> View<Msg> {
  Col(gap: 8) {
    Text("{s.count}")
    Row(gap: 8) {
      Button("-", on: Dec)
      Button("+", on: Inc)
    }
  }
}

app counter {
  state:  Counter
  update: update
  view:   counter_view
  grants: []
}

example bumps {
  bump(1) == 4243
}

example inc_then_dec {
  var ui = ui_start(counter)
  ui_click(&ui, "#1/#1")
  ui_click(&ui, "#1/#1")
  ui_click(&ui, "#1/#0")
  ui_text(ui, "#0") == "1"
}

example agent_guess {
  bump(0) == 4242
}
"##;

/// A headless app granted the foreign declaration `Sum`.
const SUM: &str = r##"foreign Sum {
  source: "sum.c"
  header: "sum.h"
  Add(a: Int) -> Int = "sum_add"
}

enum Msg { Start, Got(n: Int), Failed(why: String) }

fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start => [Add(1, then: Got, or: Failed)]
    Got(n) => [Print("got {n}"), Exit]
    Failed(why) => [Print(why), Exit]
  }
}

fn twice(n: Int) -> Int { n * 2 }

app sum {
  start: Start
  update: update
  grants: [Console, Sum]
}

example twice_two {
  twice(2) == 4
}
"##;

const SUM_C: &str = "#include \"sum.h\"\nint64_t sum_add(int64_t a) { return a + 1; }\n";
const SUM_H: &str = "#include <stdint.h>\nint64_t sum_add(int64_t a);\n";

const REVERT: &str = "Revert the change, or ask the owner to run paka approve.\n";

/// A scratch directory holding the owner's key and `tree`: the programs `counter` and `sum`
/// under one store, signed.
struct Tree(Scratch);

impl Tree {
    fn new(name: &str) -> Tree {
        build_runtime();
        let tree = Tree(Scratch::new(name));
        tree.keygen("owner");
        fs::create_dir_all(tree.path("tree/counter")).unwrap();
        fs::create_dir_all(tree.path("tree/sum")).unwrap();
        fs::write(tree.path("tree/counter/main.paka"), COUNTER).unwrap();
        fs::write(tree.path("tree/sum/main.paka"), SUM).unwrap();
        fs::write(tree.path("tree/sum/sum.c"), SUM_C).unwrap();
        fs::write(tree.path("tree/sum/sum.h"), SUM_H).unwrap();
        fs::copy(tree.path("owner.pub"), tree.path("tree/approvals.pub")).unwrap();
        tree.sign(|_| true);
        tree
    }

    fn path(&self, file: &str) -> PathBuf {
        self.0.path(file)
    }

    fn keygen(&self, name: &str) {
        let made = Command::new("/usr/bin/ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
            .arg(self.path(name))
            .env_clear()
            .status()
            .unwrap();
        assert!(made.success());
    }

    /// Signs, with serial 1, the records of both programs as they stand that `keep` keeps, but
    /// `agent_guess`.
    fn sign(&self, keep: impl Fn(&Record) -> bool) {
        let mut all = vec![];
        for program in ["counter", "sum"] {
            let dir = self.path("tree").join(program);
            let (source, read) = paka::source::read(dir.as_os_str());
            read.unwrap();
            all.extend(records(program, &source).unwrap());
        }
        let guess = |r: &Record| matches!(r, Record::Example { name, .. } if name == "agent_guess");
        all.retain(|r| keep(r) && !guess(r));
        let store = Store {
            serial: 1,
            records: all,
        };
        store.save(&self.path("tree"), &self.path("owner")).unwrap();
    }

    /// Replaces `from` with `to` in `file`.
    fn edit(&self, file: &str, from: &str, to: &str) {
        let path = self.path(file);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "{from:?}");
        fs::write(path, text.replace(from, to)).unwrap();
    }

    /// Puts first on the path a clang that runs `sed` on the IR before the real one links it.
    fn plant_ir(&self, sed: &str) {
        let script = format!(
            "#!/bin/sh\nfor a in \"$@\"; do case \"$a\" in *.ll) sed -i '{sed}' \"$a\";; esac; done\n\
             exec '{}' \"$@\"\n",
            clang().display()
        );
        self.fake_clang(&script);
    }

    /// Puts first on the path a clang that links the program to `real` in the scratch directory
    /// and writes `wrapper`, with `REAL` standing for that path, where the binary belongs.
    fn wrap_binary(&self, wrapper: &str) {
        let real = self.path("real");
        let wrapper = wrapper.replace("REAL", &real.display().to_string());
        fs::write(self.path("wrapper"), wrapper).unwrap();
        let script = format!(
            "#!/bin/sh\n[ \"$1\" = -O3 ] || exec '{clang}' \"$@\"\n\
             for a in \"$@\"; do [ \"$prev\" = -o ] && out=$a; prev=$a; done\n\
             '{clang}' \"$@\" || exit\nmv \"$out\" '{real}'\n\
             cp '{wrapper}' \"$out\"\nchmod +x \"$out\"\n",
            clang = clang().display(),
            real = real.display(),
            wrapper = self.path("wrapper").display(),
        );
        self.fake_clang(&script);
    }

    fn fake_clang(&self, script: &str) {
        fs::create_dir_all(self.path("bin")).unwrap();
        fs::write(self.path("bin/clang"), script).unwrap();
        fs::set_permissions(self.path("bin/clang"), fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// `paka release tree/<program> -o dist --key <key>` in the scratch directory: standard
    /// output, standard error and status.
    fn release(&self, program: &str, key: &str) -> (String, String, Option<i32>) {
        let path = env::var_os("PATH").unwrap();
        let path = [self.path("bin")]
            .into_iter()
            .chain(env::split_paths(&path));
        let path = env::join_paths(path).unwrap();
        let out = Command::new(PAKA)
            .arg("release")
            .arg(format!("tree/{program}"))
            .args(["-o", "dist", "--key", key])
            .current_dir(self.path(""))
            .env("PATH", path)
            .env("HOME", self.path("home"))
            .env("XDG_CACHE_HOME", self.path("cache"))
            .output()
            .unwrap();
        let [stdout, stderr] = [out.stdout, out.stderr].map(|b| String::from_utf8(b).unwrap());
        (stdout, stderr, out.status.code())
    }

    /// Asserts the release of `program` refuses with status 1 and standard error starting with
    /// `why`, all of it when `whole`, and leaves nothing at `dist` nor beside it.
    fn refuses(&self, program: &str, why: &str, whole: bool) {
        let (_, err, status) = self.release(program, "owner");
        assert_eq!(status, Some(1), "{err}");
        match whole {
            true => assert_eq!(err, why),
            false => assert!(err.starts_with(why), "{err}"),
        }
        assert!(!self.path("dist").exists());
        let entries = fs::read_dir(self.path("")).unwrap().flatten();
        let fresh = entries.filter(|e| e.file_name().to_string_lossy().starts_with(".dist"));
        assert_eq!(fresh.count(), 0, "a directory is left beside dist");
    }
}

/// The clang on PATH.
fn clang() -> PathBuf {
    let path = env::var_os("PATH").unwrap();
    let found = env::split_paths(&path).map(|d| d.join("clang"));
    found
        .into_iter()
        .find(|c| c.is_file())
        .expect("clang on PATH")
}

fn hash(path: impl AsRef<Path>) -> String {
    hex(&sha256(&fs::read(path).unwrap()))
}

/// The names in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// The `compiler` line's value: this paka and the pinned clang.
fn compiler() -> String {
    let clang = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/clang-version"));
    format!("paka 0.0.0  sha256:{}  clang {}", hash(PAKA), clang.trim())
}

/// The runtime every binary links.
fn runtime() -> PathBuf {
    Path::new(PAKA).with_file_name("libpaka_rt.a")
}

#[test]
fn a_release_holds_the_binary_and_the_hashes_of_all_it_read_and_made() {
    let tree = Tree::new("release_counter");
    let (out, err, status) = tree.release("counter", "owner");
    assert_eq!((err.as_str(), status), ("", Some(0)));
    let binary = hash(tree.path("dist/counter"));
    let expected = format!(
        "key: approvals.pub matches owner.pub\n\
         gate: approvals serial 1, signature good, nothing pending\n\
         build: paka build counter -o counter --examples bumps\n\
         examples in binary: 1 of 1 pass\n\
         ui examples replayed: inc_then_dec 4 of 4 actions agree\n\
         released: dist  counter sha256 {binary}\n"
    );
    assert_eq!(out, expected);
    assert_eq!(names(&tree.path("dist")), ["RELEASE", "counter"]);
    let release = format!(
        "program    counter\n\
         source     main.paka  sha256:{}\n\
         build      paka build counter -o counter --examples bumps\n\
         binary     counter  sha256:{binary}\n\
         compiler   {}\n\
         runtime    libpaka_rt.a  sha256:{}\n\
         approvals  serial 1  sha256:{}\n\
         example    bumps  pass\n\
         example    inc_then_dec  4 of 4 actions agree\n",
        hash(tree.path("tree/counter/main.paka")),
        compiler(),
        hash(runtime()),
        hash(tree.path("tree/approvals")),
    );
    assert_eq!(
        fs::read_to_string(tree.path("dist/RELEASE")).unwrap(),
        release
    );
    // Only the human-owned examples are built in.
    let guess = Command::new(tree.path("dist/counter"))
        .args(["--paka-examples", "agent_guess"])
        .output()
        .unwrap();
    assert_ne!(guess.stdout, b"agent_guess pass\n");
}

#[test]
fn a_release_builds_each_granted_host_fresh() {
    let tree = Tree::new("release_sum");
    let (out, err, status) = tree.release("sum", "owner");
    assert_eq!((err.as_str(), status), ("", Some(0)));
    let binary = hash(tree.path("dist/sum"));
    let expected = format!(
        "key: approvals.pub matches owner.pub\n\
         gate: approvals serial 1, signature good, nothing pending\n\
         build: paka build sum -o sum --examples twice_two\n\
         examples in binary: 1 of 1 pass\n\
         ui examples replayed: none\n\
         released: dist  sum sha256 {binary}\n"
    );
    assert_eq!(out, expected);
    assert_eq!(names(&tree.path("dist")), ["RELEASE", "sum", "sum.Sum"]);
    let release = format!(
        "program    sum\n\
         source     main.paka  sha256:{}\n\
         source     sum.c  sha256:{}\n\
         source     sum.h  sha256:{}\n\
         build      paka build sum -o sum --examples twice_two\n\
         binary     sum  sha256:{binary}\n\
         host       sum.Sum  sha256:{}\n\
         compiler   {}\n\
         runtime    libpaka_rt.a  sha256:{}\n\
         approvals  serial 1  sha256:{}\n\
         example    twice_two  pass\n",
        hash(tree.path("tree/sum/main.paka")),
        hash(tree.path("tree/sum/sum.c")),
        hash(tree.path("tree/sum/sum.h")),
        hash(tree.path("dist/sum.Sum")),
        compiler(),
        hash(runtime()),
        hash(tree.path("tree/approvals")),
    );
    assert_eq!(
        fs::read_to_string(tree.path("dist/RELEASE")).unwrap(),
        release
    );
    let run = Command::new(tree.path("dist/sum")).output().unwrap();
    assert_eq!(
        (&run.stdout[..], run.status.code()),
        (&b"got 2\n"[..], Some(0))
    );
    assert!(
        !tree.path("cache/paka/hosts").exists(),
        "no host from the cache"
    );
}

#[test]
fn an_existing_output_is_refused_and_kept() {
    let tree = Tree::new("release_exists");
    fs::create_dir(tree.path("dist")).unwrap();
    fs::write(tree.path("dist/keep"), "kept").unwrap();
    let (_, err, status) = tree.release("counter", "owner");
    assert_eq!(
        (err.as_str(), status),
        ("release: dist already exists\n", Some(1))
    );
    assert_eq!(names(&tree.path("dist")), ["keep"]);
}

#[test]
fn a_key_other_than_the_stores_is_refused() {
    let tree = Tree::new("release_key");
    tree.keygen("other");
    let (_, err, status) = tree.release("counter", "other");
    let why = "key: approvals.pub differs from other.pub\n";
    assert_eq!((err.as_str(), status), (why, Some(1)));
    assert!(!tree.path("dist").exists());
}

#[test]
fn a_gate_refusal_refuses_the_release() {
    let tree = Tree::new("release_gate");
    tree.edit(
        "tree/counter/main.paka",
        "bump(1) == 4243",
        "bump(1) >= 4243",
    );
    let why = format!("example counter bumps: edited\n{REVERT}");
    tree.refuses("counter", &why, true);
}

#[test]
fn a_program_without_records_is_refused() {
    let tree = Tree::new("release_no_records");
    tree.sign(|r| r.program() != Some("counter"));
    tree.refuses(
        "counter",
        "release: counter has no approval records\n",
        true,
    );
}

#[test]
fn a_program_without_a_human_owned_example_is_refused() {
    let tree = Tree::new("release_no_owned");
    tree.sign(|r| !matches!(r, Record::Example { program, .. } if program == "counter"));
    let why = "release: counter has no human-owned example\n";
    tree.refuses("counter", why, true);
}

#[test]
fn a_build_that_fails_is_refused() {
    let tree = Tree::new("release_hole");
    let with_hole = COUNTER.to_string() + "\nfn later() -> Int {\n  ??later\n}\n";
    fs::write(tree.path("tree/counter/main.paka"), with_hole).unwrap();
    let why = "tree/counter/main.paka:48:3: error[hole-in-build]: a release build has no holes; \
               finish ??later\n";
    tree.refuses("counter", why, true);
}

#[test]
fn a_host_that_fails_is_refused() {
    let tree = Tree::new("release_host");
    // The header's type is not the declared one, so the host does not build.
    let header = "#include <stdint.h>\nint32_t sum_add(int32_t a);\n";
    fs::write(tree.path("tree/sum/sum.h"), header).unwrap();
    tree.sign(|_| true);
    tree.refuses("sum", "host Sum: clang failed (exit status: 1): ", false);
}

#[test]
fn a_function_example_that_fails_in_the_binary_is_refused() {
    let tree = Tree::new("release_example");
    tree.plant_ir("s/i64 4242/i64 4241/g");
    tree.refuses("counter", "example bumps: fail\n", true);
}

#[test]
fn a_ui_example_that_differs_in_the_binary_is_refused() {
    let tree = Tree::new("release_ui");
    // A column drawn as a row (paka-diff's plant 19).
    tree.plant_ir("s/@paka_rt1_view(i64 3,/@paka_rt1_view(i64 2,/g");
    let why = "example inc_then_dec: release compared on 0 of 4 actions: action 0: \
               line \"Col gap=8\", release \"Row gap=8\"\n";
    tree.refuses("counter", why, true);
}

#[test]
fn a_ui_example_compared_on_fewer_actions_is_refused() {
    let tree = Tree::new("release_ui_short");
    // Each result is six lines: the replay stops after the launch and two actions.
    tree.wrap_binary(
        "#!/bin/sh\nif [ \"$1\" = --paka-replay ]; then\n  'REAL' \"$@\" | head -n 18\n  \
         exit 0\nfi\nexec 'REAL' \"$@\"\n",
    );
    let why = "example inc_then_dec: release compared on 3 of 4 actions: action 3: \
               release output ends, status Some(0) and \"\"\n";
    tree.refuses("counter", why, true);
}

#[test]
fn a_binary_changed_after_it_was_hashed_is_refused() {
    let tree = Tree::new("release_planted");
    // The binary grows by a byte at each run.
    tree.wrap_binary("#!/bin/sh\nprintf x >> \"$0\"\nexec 'REAL' \"$@\"\n");
    let why = "artefact: counter changed between the runs and the final path\n";
    tree.refuses("counter", why, true);
}

#[test]
fn a_host_changed_after_it_was_hashed_is_refused() {
    let tree = Tree::new("release_host_planted");
    // The binary's example run grows the host beside it by a byte.
    tree.wrap_binary("#!/bin/sh\nprintf x >> \"$0.Sum\"\nexec 'REAL' \"$@\"\n");
    let why = "artefact: sum.Sum changed between the runs and the final path\n";
    tree.refuses("sum", why, true);
}

#[test]
fn a_file_the_build_left_unlisted_is_refused() {
    let tree = Tree::new("release_extra");
    let script = format!(
        "#!/bin/sh\n[ \"$1\" = -O3 ] || exec '{clang}' \"$@\"\n\
         for a in \"$@\"; do [ \"$prev\" = -o ] && out=$a; prev=$a; done\n\
         '{clang}' \"$@\" || exit\necho 'not listed' > \"$(dirname \"$out\")/extra\"\n",
        clang = clang().display(),
    );
    tree.fake_clang(&script);
    tree.refuses(
        "counter",
        "artefact: extra is not a file RELEASE lists\n",
        true,
    );
}

#[test]
fn a_link_that_fails_is_refused() {
    let tree = Tree::new("release_link");
    let script = format!(
        "#!/bin/sh\n[ \"$1\" = -O3 ] && exit 1\nexec '{}' \"$@\"\n",
        clang().display()
    );
    tree.fake_clang(&script);
    let why = "tree/counter: error[clang-failed]: clang failed (exit status: 1)\n";
    tree.refuses("counter", why, true);
}

#[test]
fn a_missing_approvals_pub_is_refused() {
    let tree = Tree::new("release_no_pub");
    fs::remove_file(tree.path("tree/approvals.pub")).unwrap();
    let why = "key: approvals.pub: No such file or directory (os error 2)\n";
    tree.refuses("counter", why, true);
}

#[test]
fn an_output_made_during_the_run_is_kept() {
    let tree = Tree::new("release_raced");
    let script = format!(
        "#!/bin/sh\n'{}' \"$@\" || exit\n[ \"$1\" = -O3 ] && mkdir dist\nexit 0\n",
        clang().display()
    );
    tree.fake_clang(&script);
    let (_, err, status) = tree.release("counter", "owner");
    assert_eq!(
        (err.as_str(), status),
        ("release: dist already exists\n", Some(1))
    );
    assert!(names(&tree.path("dist")).is_empty());
}

#[test]
fn a_key_with_another_comment_matches() {
    let tree = Tree::new("release_comment");
    let key = fs::read_to_string(tree.path("owner.pub")).unwrap();
    let commented = format!("{}  the owner\n", key.trim_end());
    fs::write(tree.path("tree/approvals.pub"), commented).unwrap();
    let (out, err, status) = tree.release("counter", "owner");
    assert_eq!((err.as_str(), status), ("", Some(0)));
    assert!(
        out.starts_with("key: approvals.pub matches owner.pub\n"),
        "{out}"
    );
}

#[test]
fn a_staging_directory_of_a_dead_run_is_removed() {
    let tree = Tree::new("release_stale");
    // No pid reaches 99999999; this test's own pid is alive.
    let dead = tree.path(".dist.release-99999999");
    let alive = tree.path(&format!(".dist.release-{}", std::process::id()));
    for dir in [&dead, &alive] {
        fs::create_dir(dir).unwrap();
        fs::write(dir.join("counter"), "left").unwrap();
    }
    let (_, err, status) = tree.release("counter", "owner");
    assert_eq!((err.as_str(), status), ("", Some(0)));
    assert!(!dead.exists());
    assert_eq!(names(&alive), ["counter"]);
}

#[test]
fn a_refusal_shows_only_the_end_of_a_long_output() {
    let tree = Tree::new("release_long");
    tree.wrap_binary(
        "#!/bin/sh\nif [ \"$1\" = --paka-examples ]; then\n  \
         head -c 3000000 /dev/zero | tr '\\0' x >&2\nfi\nexec 'REAL' \"$@\"\n",
    );
    let why = format!(
        "example bumps: status 0, standard output \"bumps pass\\n\", standard error \"…{}\"\n",
        "x".repeat(200)
    );
    tree.refuses("counter", &why, true);
}
