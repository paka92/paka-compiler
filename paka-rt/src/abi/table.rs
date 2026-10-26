//! What compiled code and the runtime agree on (M6-10): the words of a String and a List, the
//! heap's alignment, and the runtime entries as compiled code declares them. The compiler's
//! layout table reads these, so the two sides cannot drift apart.

use super::dev::words::{ListWords, TextWords};
use std::mem::offset_of;

/// Where a header's words sit, in bytes from its start.
pub struct Header {
    pub size: usize,
    pub ptr: usize,
    /// A String's bytes, a List's items.
    pub len: usize,
    pub cap: usize,
}

pub const STRING: Header = Header {
    size: size_of::<TextWords>(),
    ptr: offset_of!(TextWords, ptr),
    len: offset_of!(TextWords, bytes),
    cap: offset_of!(TextWords, cap),
};

/// A String's count of code points.
pub const CHARS: usize = offset_of!(TextWords, chars);

pub const LIST: Header = Header {
    size: size_of::<ListWords>(),
    ptr: offset_of!(ListWords, ptr),
    len: offset_of!(ListWords, len),
    cap: offset_of!(ListWords, cap),
};

/// Every heap block's address is a multiple of this.
pub const ALIGN: usize = 8;

/// The cap of a String or List that owns no block: its bytes sit in a constant or in another
/// value's block. Drop frees nothing; it is copied before its first write.
pub const CONSTANT: u64 = 0;

/// The runtime entries, each line as compiled code writes it.
pub const RUN: &str = "declare i32 @paka_rt1_run(ptr, i64, ptr, i64, i64)";
pub const TRAP: &str = "declare void @paka_rt1_trap(ptr, i64) cold nounwind";
/// Compiled code only loads it (M6-16).
pub const STACK_LIMIT: &str = "@paka_rt1_stack_limit = external global i64";
pub const ENTRIES: &[&str] = &[RUN, TRAP, STACK_LIMIT];

#[cfg(test)]
mod tests {
    use super::super::dev::records::{Part, Records, Spec, rt_copy, rt_drop};
    use super::super::dev::{Context, Heap};
    use super::super::{Update, paka_rt1_run, paka_rt1_stack_limit, paka_rt1_trap};
    use super::*;
    use std::slice;
    use std::sync::atomic::AtomicU64;

    /// The IR spelling of a Rust type at the C boundary.
    trait Ir {
        const IR: &str;
    }
    impl Ir for () {
        const IR: &str = "void";
    }
    impl Ir for u64 {
        const IR: &str = "i64";
    }
    impl Ir for i32 {
        const IR: &str = "i32";
    }
    impl<T> Ir for *const T {
        const IR: &str = "ptr";
    }
    impl Ir for Option<Update> {
        const IR: &str = "ptr";
    }

    fn declared<A: Ir, B: Ir, R: Ir>(name: &str, _: unsafe extern "C" fn(A, B) -> R) -> String {
        format!("declare {} @{name}({}, {})", R::IR, A::IR, B::IR)
    }

    #[test]
    fn entries_are_declared_as_the_runtime_defines_them() {
        assert!(TRAP.starts_with(&declared("paka_rt1_trap", paka_rt1_trap)));
        let _: &AtomicU64 = &paka_rt1_stack_limit;
        let limit = format!("@paka_rt1_stack_limit = external global {}", u64::IR);
        assert_eq!((STACK_LIMIT, size_of::<AtomicU64>()), (limit.as_str(), 8));
        let _: unsafe extern "C" fn(*const u8, u64, Option<Update>, u64, u64) -> i32 = paka_rt1_run;
        let args = [
            <*const u8>::IR,
            u64::IR,
            <Option<Update>>::IR,
            u64::IR,
            u64::IR,
        ];
        assert_eq!(
            RUN,
            format!("declare {} @paka_rt1_run({})", i32::IR, args.join(", "))
        );
    }

    #[test]
    fn every_block_is_aligned() {
        let mut heap = Heap::new(1 << 20);
        for bytes in 0..=65 {
            let block = heap.alloc(bytes).unwrap();
            assert_eq!(block.addr() % ALIGN, 0, "{bytes} bytes");
            heap.release(block, bytes);
        }
        assert!(heap.faults().is_empty());
    }

    /// The record of a value that is a String (type 0) or a List of Int (type 1).
    fn records() -> Records {
        let whole = |size: usize, part| {
            let alternatives = vec![vec![(0, part)]];
            Some(Spec {
                size: size as u64,
                tagged: false,
                alternatives,
            })
        };
        Records::new(&[
            whole(STRING.size, Part::String),
            whole(LIST.size, Part::List { item: 2, size: 8 }),
            None,
        ])
    }

    /// Drops a word copy of `words` of type `t`, then a copy the runtime made: the first frees
    /// nothing, the second frees its own new block. The copy's bytes are `bytes`.
    fn constant_is_left_and_copied<const N: usize>(t: usize, words: [u64; N], bytes: &[u8]) {
        let (records, mut ctx) = (records(), Context::new(1 << 20));
        let [ptr, cap] = [0, N - 1];
        assert_eq!(words[cap], CONSTANT);
        let mut copy = [0u64; N];
        let addr = |w: &[u64; N]| w.as_ptr().expose_provenance() as u64;
        // SAFETY: both hold a value of type `t`, apart; the constant's bytes live for the test.
        unsafe {
            rt_drop(&mut ctx, records.address(t), addr(&words));
            assert!(ctx.heap.leaks().is_empty() && ctx.heap.faults().is_empty());
            let to = copy.as_mut_ptr().expose_provenance() as u64;
            assert_eq!(
                rt_copy(&mut ctx, 0, records.address(t), addr(&words), to),
                0
            );
            assert_ne!(copy[cap], CONSTANT);
            assert_ne!(copy[ptr], words[ptr]);
            let made = std::ptr::with_exposed_provenance::<u8>(copy[ptr] as usize);
            assert_eq!(slice::from_raw_parts(made, bytes.len()), bytes);
            rt_drop(&mut ctx, records.address(t), addr(&copy));
        }
        assert!(ctx.heap.leaks().is_empty() && ctx.heap.faults().is_empty());
    }

    #[test]
    fn a_constant_string_or_list_is_never_freed_and_copies_to_its_own_block() {
        static TEXT: &[u8] = b"constant";
        static ITEMS: [u64; 3] = [7, 8, 9];
        let text = TEXT.as_ptr().expose_provenance() as u64;
        constant_is_left_and_copied(0, [text, 8, 8, CONSTANT], TEXT);
        let items = ITEMS.as_ptr().expose_provenance() as u64;
        // SAFETY: `ITEMS` is 24 bytes.
        let bytes = unsafe { slice::from_raw_parts(ITEMS.as_ptr().cast(), 24) };
        constant_is_left_and_copied(1, [items, 3, CONSTANT], bytes);
    }
}
