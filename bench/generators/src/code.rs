//! Machine code in executable memory, the slot table, and the stand-in runtime functions.

use paka_rt::ops::Float;

/// The status a definition returns for each trap; 0 is success.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trap {
    Overflow = 1,
    DivideByZero = 2,
    BadConversion = 4,
}

/// The rows the runtime functions stand in for, in slot order after the definitions.
pub const RUNTIME: [&str; 6] = [
    "floor(x: Float) -> Float",
    "ceil(x: Float) -> Float",
    "trunc(x: Float) -> Float",
    "round(x: Float) -> Float",
    "min(a: Float, b: Float) -> Float",
    "max(a: Float, b: Float) -> Float",
];

fn float(x: f64) -> Float {
    Float::from_bits(x.to_bits())
}

fn back(x: Float) -> f64 {
    f64::from_bits(x.bits())
}

extern "C" fn floor(x: f64) -> f64 {
    back(float(x).floor())
}

extern "C" fn ceil(x: f64) -> f64 {
    back(float(x).ceil())
}

extern "C" fn trunc(x: f64) -> f64 {
    back(float(x).trunc())
}

extern "C" fn round(x: f64) -> f64 {
    back(float(x).round())
}

extern "C" fn min(a: f64, b: f64) -> f64 {
    back(float(a).min(float(b)))
}

extern "C" fn max(a: f64, b: f64) -> f64 {
    back(float(a).max(float(b)))
}

/// The slot table for `defs` definitions: their code addresses, then the runtime's.
pub fn slots(defs: usize) -> Vec<u64> {
    let unary: [extern "C" fn(f64) -> f64; 4] = [floor, ceil, trunc, round];
    let binary: [extern "C" fn(f64, f64) -> f64; 2] = [min, max];
    let runtime = unary
        .map(|f| f as usize as u64)
        .into_iter()
        .chain(binary.map(|f| f as usize as u64));
    vec![0; defs].into_iter().chain(runtime).collect()
}

unsafe extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut u8, len: usize) -> i32;
}

const READ: i32 = 1;
const WRITE: i32 = 2;
const EXEC: i32 = 4;
const PRIVATE_ANONYMOUS: i32 = 0x02 | 0x20;
const PAGE: usize = 4096;

/// One definition's machine code, mapped read and execute.
pub struct Code {
    at: *mut u8,
    len: usize,
}

impl Code {
    pub fn install(bytes: &[u8]) -> Code {
        let len = bytes.len().div_ceil(PAGE) * PAGE;
        // SAFETY: a fresh private mapping; the copy stays inside it, and it turns executable only
        // after the write.
        unsafe {
            let at = mmap(
                std::ptr::null_mut(),
                len,
                READ | WRITE,
                PRIVATE_ANONYMOUS,
                -1,
                0,
            );
            assert!(at as isize != -1, "mmap failed");
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), at, bytes.len());
            assert_eq!(mprotect(at, len, READ | EXEC), 0, "mprotect failed");
            Code { at, len }
        }
    }

    pub fn address(&self) -> u64 {
        self.at as u64
    }
}

impl Drop for Code {
    fn drop(&mut self) {
        // SAFETY: the mapping is this value's own, and no slot points at it any more.
        unsafe { munmap(self.at, self.len) };
    }
}

type Entry = unsafe extern "C" fn(*const u64, *mut i64, i64, i64, i64) -> u32;

/// Calls the definition in slot `at` on three arguments.
pub fn call(slots: &[u64], at: usize, args: [i64; 3]) -> Result<i64, Trap> {
    let mut out = 0;
    // SAFETY: every slot holds installed code built for this signature by `lower::function`, or a
    // runtime function above.
    let status = unsafe {
        let f: Entry = std::mem::transmute(slots[at] as usize);
        f(slots.as_ptr(), &mut out, args[0], args[1], args[2])
    };
    let traps = [Trap::Overflow, Trap::DivideByZero, Trap::BadConversion];
    match traps.into_iter().find(|&t| t as u32 == status) {
        _ if status == 0 => Ok(out),
        Some(trap) => Err(trap),
        None => panic!("unknown status {status}"),
    }
}
