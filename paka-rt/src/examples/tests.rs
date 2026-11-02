use super::*;
use crate::abi::heap::paka_rt1_alloc;
use crate::abi::record_trap;

const NAMES: &str = "holds fails traps stops leaks";

/// Example `id` of `NAMES`, as compiled code would run it.
extern "C" fn example(id: u64, out: &mut u64) -> i32 {
    match id {
        0 => *out = 1,
        1 => *out = 0,
        2 => {
            record_trap(Some(b"divide_by_zero at main.paka:5:37"));
            return 1;
        }
        3 => return 1,
        _ => {
            paka_rt1_alloc(8);
            *out = 1;
        }
    }
    0
}

fn ran(args: &[&str]) -> Option<(i32, String)> {
    let args: Vec<OsString> = args.iter().map(OsString::from).collect();
    outcome("app", NAMES, Some(example), &args)
}

fn said(status: i32, line: &str) -> Option<(i32, String)> {
    Some((status, line.into()))
}

#[test]
fn without_the_switch_the_app_runs() {
    assert_eq!(ran(&[]), None);
    assert_eq!(ran(&["holds"]), None);
    assert_eq!(ran(&["--paka-replay", "holds"]), None);
    assert_eq!(ran(&["x", "--paka-examples", "holds"]), None);
}

#[test]
fn an_example_passes_or_fails_by_its_result() {
    assert_eq!(ran(&["--paka-examples", "holds"]), said(0, "holds pass\n"));
    assert_eq!(ran(&["--paka-examples", "fails"]), said(0, "fails fail\n"));
}

#[test]
fn a_trap_gives_its_line_and_status_3() {
    let trap = "app: trap divide_by_zero at main.paka:5:37\n";
    assert_eq!(ran(&["--paka-examples", "traps"]), said(3, trap));
}

#[test]
fn a_leak_or_a_status_without_a_trap_is_an_internal_error() {
    let leak = "app: internal error: memory ledger not clean\n";
    assert_eq!(ran(&["--paka-examples", "leaks"]), said(4, leak));
    let status = "app: internal error: example returned a non-zero status\n";
    assert_eq!(ran(&["--paka-examples", "stops"]), said(4, status));
    let args = [OsString::from("--paka-examples"), "holds".into()];
    let null = "app: internal error: examples call is null\n";
    assert_eq!(outcome("app", NAMES, None, &args), said(4, null));
}

#[test]
fn an_unknown_name_or_another_shape_is_a_usage_line() {
    for name in ["nope", "hold", "holds fails", ""] {
        let line = format!("app: --paka-examples: no function example named {name}\n");
        assert_eq!(ran(&["--paka-examples", name]), said(2, &line), "{name:?}");
    }
    let shape = "app: --paka-examples takes one example name\n";
    assert_eq!(ran(&["--paka-examples"]), said(2, shape));
    assert_eq!(ran(&["--paka-examples", "holds", "fails"]), said(2, shape));
}
