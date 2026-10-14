//! The commit check: a counted batch runs only from the commit tagged `run` (M3-18), with a
//! clean working tree. Asks `git`; practice batches skip it.

use crate::Error;
use std::path::Path;
use std::process::Command;

/// The tag the owner puts on the frozen commit.
const TAG: &str = "run";

fn git(repo: &Path, args: &[&str]) -> Result<String, Error> {
    let out = Command::new("git").current_dir(repo).args(args).output()?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        return Err(Error::Commit(format!("git {}: {why}", args.join(" "))));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Fails unless HEAD of `repo` is the commit tagged `run` and nothing is modified or untracked.
pub fn check(repo: &Path) -> Result<(), Error> {
    let head = git(repo, &["rev-parse", "HEAD"])?;
    let tagged = git(repo, &["rev-parse", &format!("refs/tags/{TAG}^{{commit}}")])?;
    if head != tagged {
        return Err(Error::Commit(format!(
            "HEAD is not the commit tagged {TAG}"
        )));
    }
    if !git(repo, &["status", "--porcelain"])?.is_empty() {
        return Err(Error::Commit("the working tree is not clean".into()));
    }
    Ok(())
}
