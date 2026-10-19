//! Loads machine code into memory that is never freed (M4-23, docs/decisions/M4-05.md section 1).
//! Pages are writable while the code is copied in, then readable and executable only. They are
//! never unmapped, so no two installs share an address. Linux on x86-64 only, where the
//! instruction cache needs no flush after a copy.

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
compile_error!("the installer runs on Linux x86-64 only");

use std::ffi::{c_int, c_long, c_void};
use std::io;

const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const PROT_EXEC: c_int = 4;
const MAP_PRIVATE: c_int = 2;
const MAP_ANONYMOUS: c_int = 0x20;
const SC_PAGESIZE: c_int = 30;
/// Where each function starts, a multiple of this.
const ALIGN: usize = 16;

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
    fn sysconf(name: c_int) -> c_long;
}

/// Installed machine code: its first byte's address and its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Code {
    pub address: usize,
    pub len: usize,
}

/// Copies `functions` into fresh pages, one after another, and makes the pages readable and
/// executable only. An empty function is refused.
pub fn install(functions: &[&[u8]]) -> io::Result<Vec<Code>> {
    if functions.iter().any(|f| f.is_empty()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "an empty function",
        ));
    }
    let starts: Vec<usize> = (functions.iter())
        .scan(0, |end, f| {
            let start = *end;
            *end = (start + f.len()).next_multiple_of(ALIGN);
            Some(start)
        })
        .collect();
    let total = starts
        .last()
        .zip(functions.last())
        .map_or(0, |(s, f)| s + f.len());
    if total == 0 {
        return Ok(vec![]);
    }
    let len = total.next_multiple_of(page_size());
    // SAFETY: a fresh anonymous private mapping at an address the kernel picks touches no memory
    // in use.
    let base = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if base as isize == -1 {
        return Err(io::Error::last_os_error());
    }
    for (f, &start) in functions.iter().zip(&starts) {
        // SAFETY: `start + f.len() <= total <= len`, so the bytes stay inside the mapping, which is
        // writable and shared with nothing.
        unsafe { std::ptr::copy_nonoverlapping(f.as_ptr(), base.cast::<u8>().add(start), f.len()) };
    }
    // SAFETY: `base` and `len` are exactly the mapping made above.
    if unsafe { mprotect(base, len, PROT_READ | PROT_EXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let code = |(f, start): (&&[u8], &usize)| Code {
        address: base as usize + start,
        len: f.len(),
    };
    Ok(functions.iter().zip(&starts).map(code).collect())
}

fn page_size() -> usize {
    // SAFETY: sysconf reads a constant of the system and touches no memory.
    let size = unsafe { sysconf(SC_PAGESIZE) };
    usize::try_from(size).expect("a positive page size")
}
