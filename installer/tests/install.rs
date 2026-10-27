//! Installed code runs, its pages are never writable and executable at once, and no address is
//! handed out twice.

use paka_installer::{Code, install};
use std::fs;

/// `mov eax, 42; ret`
const FORTY_TWO: &[u8] = &[0xb8, 0x2a, 0x00, 0x00, 0x00, 0xc3];
/// `lea eax, [rdi + 1]; ret`
const PLUS_ONE: &[u8] = &[0x8d, 0x47, 0x01, 0xc3];

fn call0(code: Code) -> i32 {
    // SAFETY: the code is FORTY_TWO, a whole function of this type that touches no memory.
    let f: extern "sysv64" fn() -> i32 = unsafe { std::mem::transmute(code.address) };
    f()
}

fn call1(code: Code, x: i32) -> i32 {
    // SAFETY: the code is PLUS_ONE, a whole function of this type that touches no memory.
    let f: extern "sysv64" fn(i32) -> i32 = unsafe { std::mem::transmute(code.address) };
    f(x)
}

/// The permissions of the mapping that holds `address`, as /proc/self/maps spells them.
fn permissions(address: usize) -> String {
    let maps = fs::read_to_string("/proc/self/maps").unwrap();
    let line = maps.lines().find(|l| {
        let (range, _) = l.split_once(' ').unwrap();
        let (start, end) = range.split_once('-').unwrap();
        let start = usize::from_str_radix(start, 16).unwrap();
        let end = usize::from_str_radix(end, 16).unwrap();
        (start..end).contains(&address)
    });
    line.unwrap().split(' ').nth(1).unwrap().into()
}

#[test]
fn installed_functions_run() {
    let code = install(&[FORTY_TWO, PLUS_ONE]).unwrap();
    assert_eq!(call0(code[0]), 42);
    assert_eq!(call1(code[1], 41), 42);
    assert_eq!(code[1].address % 16, 0);
    assert_eq!(code.iter().map(|c| c.len).collect::<Vec<_>>(), [6, 4]);
}

#[test]
fn pages_are_readable_and_executable_only() {
    for code in install(&[FORTY_TWO, PLUS_ONE]).unwrap() {
        assert_eq!(permissions(code.address), "r-xp");
    }
}

#[test]
fn addresses_never_repeat() {
    let mut ranges: Vec<(usize, usize)> = (0..200)
        .flat_map(|_| install(&[FORTY_TWO, PLUS_ONE]).unwrap())
        .map(|c| (c.address, c.address + c.len))
        .collect();
    ranges.sort();
    assert!(ranges.windows(2).all(|w| w[0].1 <= w[1].0));
    assert_eq!(ranges.len(), 400);
}

#[test]
fn an_empty_function_is_refused() {
    let error = install(&[FORTY_TWO, &[]]).unwrap_err();
    assert_eq!(error.to_string(), "an empty function");
    assert_eq!(install(&[]).unwrap(), []);
}
