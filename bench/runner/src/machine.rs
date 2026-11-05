//! The machine facts the results file records (M6-02 section 3) and the load gate.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;

/// The one logical CPU every timed run is pinned to: the last one.
pub fn pinned_cpu() -> usize {
    thread::available_parallelism().map_or(0, |n| n.get() - 1)
}

/// The one-minute load average.
pub fn load() -> Result<f64, String> {
    let text = fs::read_to_string("/proc/loadavg").map_err(|e| format!("/proc/loadavg: {e}"))?;
    first_load(&text).ok_or_else(|| format!("/proc/loadavg reads {text:?}"))
}

fn first_load(text: &str) -> Option<f64> {
    text.split_whitespace().next()?.parse().ok()
}

/// `name value` lines: CPU model, kernel, governor, boost, pinned CPU and source commit.
pub fn facts(programs: &Path, cpu: usize) -> String {
    let model = read("/proc/cpuinfo")
        .lines()
        .find_map(|l| {
            l.strip_prefix("model name")
                .map(|r| r.trim_start_matches([' ', '\t', ':']))
        })
        .unwrap_or("unknown")
        .to_string();
    let governor = read(&format!(
        "/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor"
    ));
    format!(
        "cpu {model}\nkernel {}\ngovernor {}\nboost {}\npinned-cpu {cpu}\ncommit {}\n",
        or_unknown(read("/proc/sys/kernel/osrelease")),
        or_unknown(governor),
        boost(),
        commit(programs),
    )
}

fn boost() -> &'static str {
    let on = match read("/sys/devices/system/cpu/cpufreq/boost").trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => match read("/sys/devices/system/cpu/intel_pstate/no_turbo").trim() {
            "0" => Some(true),
            "1" => Some(false),
            _ => None,
        },
    };
    on.map_or("unknown", |on| if on { "on" } else { "off" })
}

/// The commit the sources were read from, marked `-dirty` when the tree has changes.
fn commit(programs: &Path) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(programs)
        .args([
            "describe",
            "--always",
            "--dirty",
            "--abbrev=40",
            "--exclude=*",
        ])
        .output();
    let text = out
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
    or_unknown(text.unwrap_or_default())
}

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn or_unknown(text: String) -> String {
    let text = text.trim();
    if text.is_empty() { "unknown" } else { text }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_is_the_first_field() {
        assert_eq!(first_load("0.64 3.82 9.05 3/2990 1867236\n"), Some(0.64));
        assert_eq!(first_load(""), None);
    }

    #[test]
    fn every_fact_is_named() {
        let text = facts(Path::new("."), pinned_cpu());
        let names: Vec<_> = text.lines().map(|l| l.split(' ').next().unwrap()).collect();
        assert_eq!(
            names,
            ["cpu", "kernel", "governor", "boost", "pinned-cpu", "commit"]
        );
    }
}
