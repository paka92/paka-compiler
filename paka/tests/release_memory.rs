//! A heap that refuses a block ends a release run in a `memory` trap, never in an abort (M6-11).

#[allow(dead_code)]
mod common;

use common::{PAKA, Scratch, build_runtime};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

const SOURCE: &str = "enum Msg { Start }

fn update(msg: Msg) -> List<Effect<Msg>> {
  let n = 3
  [Print(\"{n}\"), Exit]
}

app oom {
  start: Start
  update: update
  grants: [Console]
}
";

/// The clang on PATH.
fn clang() -> PathBuf {
    let path = env::var_os("PATH").unwrap();
    let found = env::split_paths(&path).map(|d| d.join("clang"));
    found
        .into_iter()
        .find(|c| c.is_file())
        .expect("clang on PATH")
}

#[test]
fn a_refused_allocation_traps_memory() {
    build_runtime();
    let scratch = Scratch::new("release-memory");
    // A clang that turns every block the program asks for into a refusal.
    let wrapper = format!(
        "#!/bin/sh\nfor a in \"$@\"; do case \"$a\" in *.ll)\n  \
         sed -i 's/call ptr @paka_rt1_alloc(/call ptr @refused(/' \"$a\"\n  \
         printf 'define internal ptr @refused(i64 %%n) {{\\n  ret ptr null\\n}}\\n' >> \"$a\";;\n\
         esac; done\nexec '{}' \"$@\"\n",
        clang().display()
    );
    let bin = scratch.path("bin");
    fs::create_dir(&bin).unwrap();
    let fake = bin.join("clang");
    fs::write(&fake, wrapper).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let (source, exe) = (scratch.path("main.paka"), scratch.path("main"));
    fs::write(&source, SOURCE).unwrap();
    let path = env::join_paths(
        [bin]
            .into_iter()
            .chain(env::split_paths(&env::var_os("PATH").unwrap())),
    );
    let built = Command::new(PAKA)
        .args([
            "build".as_ref(),
            source.as_os_str(),
            "-o".as_ref(),
            exe.as_os_str(),
        ])
        .env("PATH", path.unwrap())
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");
    let out = Command::new(&exe).output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(3), "{stderr}");
    assert!(
        stderr.starts_with("oom: trap memory at main.paka:"),
        "{stderr}"
    );
}
