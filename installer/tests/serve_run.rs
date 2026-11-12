//! The run call and an edit's run (M4-39, docs/decisions/M4-03.md section 3.4): the transcripts in
//! `tests/serve-run`, served in this process on the dev runner built beside it. A transcript
//! `x.txt` is served on a copy of `x/before`, or of the wc sample without it, from that folder.

use paka::serve::serve;
use paka::session::{self, Runner, Session};
use paka::source::read;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const WC: &str = "tests/cases/run-interp/sample_wc_count";

#[test]
fn run_transcripts_get_their_replies() {
    let mut paths: Vec<_> = (fs::read_dir(Path::new(ROOT).join("tests/serve-run")).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    paths.sort();
    for path in &paths {
        let text = fs::read_to_string(path).unwrap();
        let sent: String = (text.lines())
            .filter_map(|l| l.strip_prefix("> "))
            .flat_map(|l| [l, "\n"])
            .collect();
        let want: Vec<_> = text.lines().filter_map(|l| l.strip_prefix("< ")).collect();
        let stem = path.with_extension("");
        let before = Some(stem.join("before")).filter(|b| b.exists());
        let dir = scratch(stem.file_name().unwrap().to_str().unwrap());
        copy(&before.unwrap_or(Path::new(ROOT).join(WC)), &dir);
        // Runs read their files from the server's working directory; this is the one test here.
        env::set_current_dir(&dir).unwrap();
        let file = "main.paka".as_ref();
        let (source, _) = read(file);
        let workspace = session::workspace(&source.text).unwrap();
        let lock = fs::File::open(file).unwrap();
        let runner = Runner::new(env!("CARGO_BIN_EXE_paka-runner").into());
        let mut session = Session::new(file, &source, workspace, lock, runner);
        let mut got = vec![];
        serve(sent.as_bytes(), &mut got, |call| session.handle(call)).unwrap();
        let got = String::from_utf8(got).unwrap();
        assert_eq!(got.lines().collect::<Vec<_>>(), want, "{}", path.display());
    }
    assert!(paths.len() >= 2);
}

fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("paka-serve-run-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        fs::copy(&path, to.join(path.file_name().unwrap())).unwrap();
    }
}
