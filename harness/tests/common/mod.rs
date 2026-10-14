//! Helpers shared by the runner and sandbox tests.

use harness::runner::{Limits, Runner};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A new empty directory under the temporary directory, named after the test.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("harness-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A runner with the paka binary Cargo built and a 5-second case limit.
pub fn runner() -> Runner {
    // Cargo builds the paka binary beside the test binary's `deps` folder.
    let exe = std::env::current_exe().unwrap();
    let paka = exe.parent().unwrap().parent().unwrap().join("paka");
    assert!(
        paka.exists(),
        "build the paka binary first: {}",
        paka.display()
    );
    let limits = Limits {
        build: Duration::from_secs(120),
        case: Duration::from_secs(5),
    };
    Runner {
        limits,
        ..Runner::new(paka).unwrap()
    }
}
