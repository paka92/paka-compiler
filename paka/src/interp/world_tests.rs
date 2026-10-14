//! The effects of M2-15 against a world in memory: each effect, each failure the world can give,
//! and each effect refused without its grant. Real files are tested in `paka/tests/effects.rs`.

use super::effects::{Reason, World};
use super::run_in;
use std::collections::BTreeMap;
use std::ffi::OsString;

/// A file in the memory world: its text, or the reason every use of it fails with.
enum File {
    Text(String),
    Fails(Reason),
}

/// Files by path, everything printed, and a reason that makes every `Print` fail.
#[derive(Default)]
struct Memory {
    files: BTreeMap<String, File>,
    out: String,
    print_fails: Option<Reason>,
}

impl World for Memory {
    fn print(&mut self, text: &str) -> Result<(), Reason> {
        match self.print_fails {
            Some(why) => Err(why),
            None => {
                self.out += &format!("{text}\n");
                Ok(())
            }
        }
    }

    fn read(&mut self, path: &str) -> Result<String, Reason> {
        match self.files.get(path) {
            Some(File::Text(text)) => Ok(text.clone()),
            Some(File::Fails(why)) => Err(why),
            None => Err("not found"),
        }
    }

    fn write(&mut self, path: &str, text: &str) -> Result<(), Reason> {
        if let Some(File::Fails(why)) = self.files.get(path) {
            return Err(why);
        }
        self.files.insert(path.into(), File::Text(text.into()));
        Ok(())
    }
}

impl Memory {
    fn with<const N: usize>(files: [(&str, File); N]) -> Memory {
        let files = files.into_iter().map(|(path, file)| (path.into(), file));
        Memory {
            files: files.collect(),
            ..Memory::default()
        }
    }

    fn text(&self, path: &str) -> Option<&str> {
        match self.files.get(path) {
            Some(File::Text(text)) => Some(text),
            _ => None,
        }
    }
}

/// Reads the first argument, writes it to the second and says so; a failure is printed with its
/// reason and ends with status 20.
fn copy(grants: &str) -> String {
    format!(
        "
type Job {{ dest: String = \"\" }}

enum Msg {{ Start(src: String, dest: String), Loaded(text: String), Wrote(name: String), Failed(why: String) }}

fn update(s: inout Job, m: Msg) -> List<Effect<Msg>> {{
  match m {{
    Start(src, dest) => {{
      s.dest = dest
      [ReadFile(src, then: Loaded, or: Failed)]
    }}
    Loaded(text) => [WriteFile(s.dest, text, then: Wrote(s.dest), or: Failed)]
    Wrote(name) => [Print(\"wrote {{name}}\"), Exit]
    Failed(why) => [Print(\"failed: {{why}}\"), ExitWith(20)]
  }}
}}

app copy {{
  state: Job
  start: Start
  update: update
  grants: [{grants}]
}}
"
    )
}

const ALL: &str = "Console, ReadFiles, WriteFiles";

/// The standard error and the status of a run of `source` in `world`.
fn play(source: &str, args: &[&str], world: &mut Memory) -> (String, u8) {
    let core = crate::check("main.paka", source.as_bytes())
        .unwrap_or_else(|d| panic!("{d}"))
        .0;
    let args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let mut err = vec![];
    let status = run_in(&core, &args, world, &mut err);
    (String::from_utf8(err).unwrap(), status)
}

#[test]
fn a_file_is_read_and_another_written_with_the_message_that_was_asked_for() {
    let mut world = Memory::with([("in.txt", File::Text("hello".into()))]);
    let done = play(&copy(ALL), &["in.txt", "out.txt"], &mut world);
    assert_eq!(done, (String::new(), 0));
    assert_eq!(world.out, "wrote out.txt\n");
    assert_eq!(world.text("out.txt"), Some("hello"));
    assert_eq!(world.text("in.txt"), Some("hello"));
}

#[test]
fn every_reason_a_read_can_fail_with_reaches_its_or_function() {
    for why in [
        "not found",
        "no permission",
        "not a file",
        "not text",
        "failed",
    ] {
        let mut world = Memory::with([("in.txt", File::Fails(why))]);
        let done = play(&copy(ALL), &["in.txt", "out.txt"], &mut world);
        assert_eq!(
            (world.out.as_str(), done.1),
            (&format!("failed: {why}\n")[..], 20)
        );
        assert_eq!(world.text("out.txt"), None);
    }
}

#[test]
fn every_reason_a_write_can_fail_with_reaches_its_or_function() {
    for why in [
        "not found",
        "no permission",
        "not a file",
        "no space",
        "failed",
    ] {
        let mut world = Memory::with([
            ("in.txt", File::Text("x".into())),
            ("out.txt", File::Fails(why)),
        ]);
        let done = play(&copy(ALL), &["in.txt", "out.txt"], &mut world);
        assert_eq!(
            (world.out.as_str(), done.1),
            (&format!("failed: {why}\n")[..], 20)
        );
    }
}

#[test]
fn a_path_no_file_can_have_fails_without_asking_the_world() {
    let world = || {
        Memory::with([
            ("", File::Text("x".into())),
            ("a\0b", File::Text("x".into())),
        ])
    };
    for (src, dest) in [("", "out"), ("a\0b", "out"), ("in", ""), ("in", "a\0b")] {
        let mut world = world();
        world.files.insert("in".into(), File::Text("x".into()));
        play(&copy(ALL), &[src, dest], &mut world);
        assert_eq!(world.out, "failed: failed\n", "{src:?} {dest:?}");
        assert_eq!(world.text("out"), None);
    }
}

#[test]
fn an_empty_file_is_an_empty_string_and_is_written_as_one() {
    let mut world = Memory::with([("in.txt", File::Text(String::new()))]);
    play(&copy(ALL), &["in.txt", "out.txt"], &mut world);
    assert_eq!(
        (world.out.as_str(), world.text("out.txt")),
        ("wrote out.txt\n", Some(""))
    );
}

/// Each effect without its grant stops there, after the effects that came before it.
#[test]
fn an_effect_without_its_grant_is_refused_and_leaves_the_world_alone() {
    let rows = [
        ("", "ReadFile refused: grant ReadFiles"),
        ("ReadFiles", "WriteFile refused: grant WriteFiles"),
        ("ReadFiles, WriteFiles", "Print refused: grant Console"),
        ("Console, WriteFiles", "ReadFile refused: grant ReadFiles"),
        ("Console, ReadFiles", "WriteFile refused: grant WriteFiles"),
    ];
    for (grants, line) in rows {
        let mut world = Memory::with([("in.txt", File::Text("x".into()))]);
        let done = play(&copy(grants), &["in.txt", "out.txt"], &mut world);
        assert_eq!(
            done,
            (format!("copy: effect {line} is missing\n"), 3),
            "[{grants}]"
        );
        assert_eq!(world.out, "");
        let written = grants.contains("WriteFiles") && grants.contains("ReadFiles");
        assert_eq!(world.text("out.txt").is_some(), written, "[{grants}]");
    }
}

#[test]
fn a_refused_effect_keeps_what_the_earlier_ones_did() {
    let source = "
enum Msg { Start, Failed(why: String) }

fn update(m: Msg) -> List<Effect<Msg>> {
  [Print(\"a\"), WriteFile(\"x\", \"y\", then: Start, or: Failed), Print(\"b\")]
}

app light {
  start: Start
  update: update
  grants: [Console]
}
";
    let mut world = Memory::default();
    let done = play(source, &[], &mut world);
    let line = "light: effect WriteFile refused: grant WriteFiles is missing\n";
    assert_eq!(done, (line.into(), 3));
    assert_eq!((world.out.as_str(), world.text("x")), ("a\n", None));
}

#[test]
fn a_failed_print_stops_with_the_worlds_reason() {
    for why in ["broken pipe", "no space left on device", "write error"] {
        let mut world = Memory::with([("in.txt", File::Text("x".into()))]);
        world.print_fails = Some(why);
        let done = play(&copy(ALL), &["in.txt", "out.txt"], &mut world);
        assert_eq!(done, (format!("copy: effect Print failed: {why}\n"), 3));
        assert_eq!(world.text("out.txt"), Some("x"), "the write came first");
    }
}

#[test]
fn a_file_answer_waits_behind_the_effects_listed_after_it() {
    let source = "
enum Msg { Start, Loaded(text: String), Missing(why: String) }

fn update(m: Msg) -> List<Effect<Msg>> {
  match m {
    Start => [ReadFile(\"in.txt\", then: Loaded, or: Missing), Print(\"asked\")]
    Loaded(text) => [Print(text), Exit]
    Missing(why) => [Print(why), ExitWith(20)]
  }
}

app reader {
  start: Start
  update: update
  grants: [Console, ReadFiles]
}
";
    let mut world = Memory::with([("in.txt", File::Text("body".into()))]);
    assert_eq!(play(source, &[], &mut world), (String::new(), 0));
    assert_eq!(world.out, "asked\nbody\n");
}

#[test]
fn the_start_message_is_filled_from_the_arguments_in_order() {
    let mut world = Memory::with([("a", File::Text("1".into()))]);
    play(&copy(ALL), &["a", "b"], &mut world);
    assert_eq!((world.text("a"), world.text("b")), (Some("1"), Some("1")));
    let (line, status) = play(&copy(ALL), &["a"], &mut world);
    assert_eq!(
        (line.as_str(), status),
        ("copy: expected 2 arguments (src, dest), got 1\n", 2)
    );
}
