//! The ABI of `docs/abi/m1.md`: its three shapes and their decoding, the exported entry and the
//! SIGPIPE call. This is the crate's only unsafe code.
//!
//! One invariant carries the safe decoders: a `Str`, `Effect` or `List` holds either a value its
//! decoder refuses or a pointer to memory that is readable for the stated size, never written
//! and never freed (ABI section 2, "Memory"). The fields are private, so such a value is made
//! only by a constructor here that takes a `'static` reference, by `Str::from_raw` under its
//! contract, or by compiled code under the contract of `paka_rt1_run`.

use crate::{Decoded, Refused, run};
use std::{ptr, slice};

/// The longest string: 2^63 - 1 bytes.
const MAX_BYTES: u64 = i64::MAX.unsigned_abs();
/// The longest list: the most effects that fit in `MAX_BYTES`.
const MAX_EFFECTS: u64 = 384_307_168_202_282_325;

const PRINT: u64 = 0;
const EXIT: u64 = 1;

const SIGPIPE: i32 = 13;
const SIG_IGN: usize = 1;

const _: () = assert!(size_of::<usize>() == 8);
const _: () = assert!(size_of::<Str>() == 16 && align_of::<Str>() == 8);
const _: () = assert!(size_of::<Effect>() == 24 && align_of::<Effect>() == 8);
const _: () = assert!(size_of::<List>() == 16 && align_of::<List>() == 8);
const _: () = assert!(MAX_EFFECTS == MAX_BYTES / 24);

/// The function named by `update:` (ABI section 3).
pub type Update = extern "C" fn(msg: u64, out: &mut List) -> i32;

/// A string: pointer to the bytes, number of bytes.
#[repr(C)]
pub struct Str {
    ptr: *const u8,
    len: u64,
}

/// An effect: tag, pointer, length.
#[repr(C)]
pub struct Effect {
    tag: u64,
    ptr: *const u8,
    len: u64,
}

/// An effect list: pointer to the first effect, number of effects.
#[repr(C)]
pub struct List {
    ptr: *const Effect,
    count: u64,
}

impl Str {
    /// # Safety
    ///
    /// Unless `bytes` refuses the pair (see there), `ptr` must point at `len` bytes that stay
    /// readable, unwritten and unfreed for the rest of the process.
    pub const unsafe fn from_raw(ptr: *const u8, len: u64) -> Str {
        Str { ptr, len }
    }

    /// The string's bytes, or `None` when the string is refused: a null pointer with a length
    /// other than 0, a length above 2^63 - 1, or bytes that would run past the last address.
    pub fn bytes(&self) -> Option<&'static [u8]> {
        if self.len == 0 {
            return Some(&[]);
        }
        if self.ptr.is_null() || self.len > MAX_BYTES {
            return None;
        }
        let len = usize::try_from(self.len).ok()?;
        self.ptr.addr().checked_add(len)?;
        // SAFETY: `from_raw_parts` asks for a non-null pointer, aligned for `u8` (any pointer
        // is), at most `isize::MAX` bytes that do not wrap around the address space, and `len`
        // readable, initialized bytes that nothing writes or frees during `'static`. The first
        // four are checked above. The last cannot be checked: it is the module invariant, owed
        // by whoever made this value.
        Some(unsafe { slice::from_raw_parts(self.ptr, len) })
    }
}

impl Effect {
    pub const EXIT: Effect = Effect {
        tag: EXIT,
        ptr: ptr::null(),
        len: 0,
    };

    pub const fn print(text: &'static [u8]) -> Effect {
        let ptr = if text.is_empty() {
            ptr::null()
        } else {
            text.as_ptr()
        };
        Effect {
            tag: PRINT,
            ptr,
            len: text.len() as u64,
        }
    }

    pub fn decode(&self) -> Result<Decoded, Refused> {
        match self.tag {
            PRINT => self.text().map(Decoded::Print).ok_or(Refused::Effect),
            EXIT if self.ptr.is_null() && self.len == 0 => Ok(Decoded::Exit),
            EXIT => Err(Refused::Effect),
            _ => Err(Refused::EffectTag),
        }
    }

    fn text(&self) -> Option<&'static [u8]> {
        Str {
            ptr: self.ptr,
            len: self.len,
        }
        .bytes()
    }
}

impl List {
    /// What the result slot holds before `update` is called; `effects` refuses it.
    pub const UNWRITTEN: List = List {
        ptr: ptr::null(),
        count: 1,
    };

    pub const fn new(effects: &'static [Effect]) -> List {
        let ptr = if effects.is_empty() {
            ptr::null()
        } else {
            effects.as_ptr()
        };
        List {
            ptr,
            count: effects.len() as u64,
        }
    }

    /// The list's effects, not yet decoded. Nothing is read through the pointer here. Refused:
    /// a pointer off an 8-byte boundary, a null pointer with a count other than 0, a count above
    /// `MAX_EFFECTS`, or effects that would run past the last address.
    pub fn effects(&self) -> Result<&'static [Effect], Refused> {
        self.slice().ok_or(Refused::EffectList)
    }

    fn slice(&self) -> Option<&'static [Effect]> {
        if !self.ptr.is_aligned() {
            return None;
        }
        if self.count == 0 {
            return Some(&[]);
        }
        if self.ptr.is_null() || self.count > MAX_EFFECTS {
            return None;
        }
        let count = usize::try_from(self.count).ok()?;
        let bytes = count.checked_mul(size_of::<Effect>())?;
        self.ptr.addr().checked_add(bytes)?;
        // SAFETY: `from_raw_parts` asks for a non-null pointer aligned for `Effect`, at most
        // `isize::MAX` bytes that do not wrap around the address space, and `count` readable,
        // initialized effects that nothing writes or frees during `'static`. The first four are
        // checked above, and any three initialized words are a valid `Effect`. That the memory
        // is there cannot be checked: it is the module invariant, owed by whoever made this
        // value.
        Some(unsafe { slice::from_raw_parts(self.ptr, count) })
    }
}

// SAFETY: this is the C library's `signal`, declared as ABI section 6 writes it. On x86-64
// Linux `int` is `i32` and a handler is one pointer-sized word.
unsafe extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Sets SIGPIPE to ignored, so a write to a closed pipe fails with EPIPE.
pub fn ignore_sigpipe() {
    // SAFETY: `signal` asks for a signal whose action may be changed and a handler that is
    // `SIG_DFL`, `SIG_IGN` or a function. 13 is SIGPIPE and 1 is `SIG_IGN` on Linux, so no code
    // is installed. The call touches no memory of ours and may be made from any thread.
    unsafe { signal(SIGPIPE, SIG_IGN) };
}

/// The entry `main` calls (ABI section 4).
///
/// # Safety
///
/// `name` and `name_len` are a string as ABI section 2 defines it. `update` is null or a function
/// that keeps ABI section 3: it writes one list through `out`, keeps no pointer and does not
/// unwind. Every list it stores, and every effect in it, is as section 2 defines it: each pointer
/// is null or points at constants that stay readable for the stated size while the process lives.
// SAFETY: no other symbol of a Paka executable has this name: compiled functions are prefixed
// `p_` (ABI section 11).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn paka_rt1_run(
    name: *const u8,
    name_len: u64,
    update: Option<Update>,
    start: u64,
    grants: u64,
) -> i32 {
    ignore_sigpipe();
    // SAFETY: the caller promises that `name` and `name_len` are an ABI string, which is what
    // `from_raw` asks for.
    let name = unsafe { Str::from_raw(name, name_len) };
    run(name, update, start, grants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::INTERNAL_ERROR;
    use std::mem::transmute;

    /// One 8-byte ABI word. It is a pointer type so that a word holding an address keeps the
    /// right to read through it; a word holding a number is made by `n`.
    type Word = *const u8;

    const SIG_DFL: usize = 0;
    const TEXT: &[u8] = b"let there be light";
    const ODD: u64 = 0x1001;
    const WILD: u64 = 0xdead_beef_0000;
    const LAST_WORD: u64 = u64::MAX - 7;

    fn n(value: u64) -> Word {
        ptr::without_provenance(value as usize)
    }

    /// Words that live for the whole process, as the constants of compiled code do.
    fn constant<const N: usize>(words: [Word; N]) -> Word {
        Box::leak(Box::new(words)).as_ptr().cast()
    }

    fn string(words: [Word; 2]) -> Str {
        // SAFETY: two words are the size of a `Str`, and every word is a valid pointer field
        // and a valid `u64` field.
        unsafe { transmute(words) }
    }

    fn effect(words: [Word; 3]) -> Effect {
        // SAFETY: three words are the size of an `Effect`, and every word is valid in each of
        // its fields.
        unsafe { transmute(words) }
    }

    fn list(words: [Word; 2]) -> List {
        // SAFETY: two words are the size of a `List`, and every word is valid in each of its
        // fields.
        unsafe { transmute(words) }
    }

    fn words_of_effect(effect: Effect) -> [u64; 3] {
        // SAFETY: an `Effect` is three initialized words with no padding.
        unsafe { transmute(effect) }
    }

    fn words_of_list(list: List) -> [u64; 2] {
        // SAFETY: a `List` is two initialized words with no padding.
        unsafe { transmute(list) }
    }

    fn address<T>(pointer: *const T) -> u64 {
        pointer.addr() as u64
    }

    /// The `@fx` of `docs/abi/light.ll`.
    fn light_fx() -> Word {
        constant([n(0), TEXT.as_ptr(), n(18), n(1), n(0), n(0)])
    }

    // Strings.

    #[test]
    fn a_string_is_its_bytes() {
        assert_eq!(string([TEXT.as_ptr(), n(18)]).bytes(), Some(TEXT));
        assert_eq!(string([TEXT.as_ptr(), n(3)]).bytes(), Some(&b"let"[..]));
    }

    #[test]
    fn a_string_is_not_told_to_be_utf8() {
        const BYTES: &[u8] = &[0xFF, 0x00, 0xC0];
        assert_eq!(string([BYTES.as_ptr(), n(3)]).bytes(), Some(BYTES));
    }

    #[test]
    fn an_empty_string_never_uses_its_pointer() {
        assert_eq!(string([n(0), n(0)]).bytes(), Some(&[][..]));
        assert_eq!(string([n(WILD), n(0)]).bytes(), Some(&[][..]));
        assert_eq!(string([n(u64::MAX), n(0)]).bytes(), Some(&[][..]));
    }

    #[test]
    fn a_null_string_with_a_length_is_refused() {
        assert_eq!(string([n(0), n(1)]).bytes(), None);
        assert_eq!(string([n(0), n(MAX_BYTES)]).bytes(), None);
        assert_eq!(string([n(0), n(u64::MAX)]).bytes(), None);
    }

    #[test]
    fn a_string_longer_than_the_limit_is_refused() {
        assert_eq!(string([TEXT.as_ptr(), n(MAX_BYTES + 1)]).bytes(), None);
        assert_eq!(string([TEXT.as_ptr(), n(u64::MAX)]).bytes(), None);
    }

    #[test]
    fn a_string_that_runs_past_the_last_address_is_refused() {
        assert_eq!(string([n(u64::MAX), n(1)]).bytes(), None);
        assert_eq!(string([n(u64::MAX - 3), n(8)]).bytes(), None);
        assert_eq!(string([n(MAX_BYTES + 2), n(MAX_BYTES)]).bytes(), None);
    }

    // Effects.

    #[test]
    fn tag_0_is_print() {
        let print = effect([n(0), TEXT.as_ptr(), n(18)]);
        assert_eq!(print.decode(), Ok(Decoded::Print(TEXT)));
    }

    #[test]
    fn a_print_of_nothing_never_uses_its_pointer() {
        assert_eq!(effect([n(0), n(0), n(0)]).decode(), Ok(Decoded::Print(b"")));
        assert_eq!(
            effect([n(0), n(WILD), n(0)]).decode(),
            Ok(Decoded::Print(b""))
        );
    }

    #[test]
    fn tag_1_is_exit() {
        assert_eq!(effect([n(1), n(0), n(0)]).decode(), Ok(Decoded::Exit));
    }

    #[test]
    fn a_tag_above_1_is_refused() {
        for tag in [2, 3, 256, 1 << 32, 1 << 63, u64::MAX] {
            let unknown = effect([n(tag), n(0), n(0)]);
            assert_eq!(unknown.decode(), Err(Refused::EffectTag), "tag {tag}");
        }
        let with_text = effect([n(2), TEXT.as_ptr(), n(18)]);
        assert_eq!(with_text.decode(), Err(Refused::EffectTag));
        let with_bad_text = effect([n(2), n(0), n(18)]);
        assert_eq!(with_bad_text.decode(), Err(Refused::EffectTag));
    }

    #[test]
    fn an_exit_with_a_used_field_is_refused() {
        let cases = [
            [n(1), TEXT.as_ptr(), n(0)],
            [n(1), n(0), n(1)],
            [n(1), TEXT.as_ptr(), n(18)],
            [n(1), n(WILD), n(u64::MAX)],
        ];
        for words in cases {
            assert_eq!(effect(words).decode(), Err(Refused::Effect));
        }
    }

    #[test]
    fn a_print_with_a_refused_text_is_refused() {
        let cases = [
            [n(0), n(0), n(18)],
            [n(0), TEXT.as_ptr(), n(MAX_BYTES + 1)],
            [n(0), TEXT.as_ptr(), n(u64::MAX)],
            [n(0), n(u64::MAX), n(1)],
        ];
        for words in cases {
            assert_eq!(effect(words).decode(), Err(Refused::Effect));
        }
    }

    // Lists.

    #[test]
    fn the_list_of_light_ll_decodes() {
        let effects = list([light_fx(), n(2)]).effects().unwrap();
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].decode(), Ok(Decoded::Print(TEXT)));
        assert_eq!(effects[1].decode(), Ok(Decoded::Exit));
    }

    #[test]
    fn a_list_is_no_longer_than_its_count() {
        let effects = list([light_fx(), n(1)]).effects().unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].decode(), Ok(Decoded::Print(TEXT)));
    }

    #[test]
    fn an_empty_list_never_uses_its_pointer() {
        assert_eq!(list([n(0), n(0)]).effects().unwrap().len(), 0);
        assert_eq!(list([n(WILD), n(0)]).effects().unwrap().len(), 0);
        assert_eq!(list([n(LAST_WORD), n(0)]).effects().unwrap().len(), 0);
    }

    #[test]
    fn a_list_pointer_off_an_8_byte_boundary_is_refused() {
        let fx = light_fx();
        for offset in 1..8 {
            let header = list([fx.wrapping_add(offset), n(2)]);
            assert_eq!(
                header.effects().err(),
                Some(Refused::EffectList),
                "offset {offset}"
            );
        }
    }

    #[test]
    fn an_empty_list_off_an_8_byte_boundary_is_refused() {
        for pointer in [1, 4, ODD, u64::MAX] {
            let header = list([n(pointer), n(0)]);
            assert_eq!(
                header.effects().err(),
                Some(Refused::EffectList),
                "pointer {pointer}"
            );
        }
    }

    #[test]
    fn a_null_list_with_a_count_is_refused() {
        for count in [1, 2, MAX_EFFECTS, u64::MAX] {
            let header = list([n(0), n(count)]);
            assert_eq!(
                header.effects().err(),
                Some(Refused::EffectList),
                "count {count}"
            );
        }
        assert_eq!(List::UNWRITTEN.effects().err(), Some(Refused::EffectList));
    }

    #[test]
    fn a_list_longer_than_the_limit_is_refused() {
        // The third count times 24 wraps to 8, the last to 0.
        let wraps_to_8 = (1 << 61) + 1;
        for count in [MAX_EFFECTS + 1, 1 << 60, wraps_to_8, 1 << 61, u64::MAX] {
            let header = list([light_fx(), n(count)]);
            assert_eq!(
                header.effects().err(),
                Some(Refused::EffectList),
                "count {count}"
            );
        }
    }

    #[test]
    fn a_list_that_runs_past_the_last_address_is_refused() {
        let cases = [
            [n(LAST_WORD), n(1)],
            [n(LAST_WORD - 16), n(1)],
            [n((1 << 63) + 8), n(MAX_EFFECTS)],
        ];
        for words in cases {
            assert_eq!(list(words).effects().err(), Some(Refused::EffectList));
        }
    }

    #[test]
    fn the_header_is_decoded_without_decoding_an_effect() {
        let fx = constant([
            n(1),
            n(0),
            n(0),
            n(99),
            n(WILD),
            n(u64::MAX),
            n(0),
            n(0),
            n(7),
        ]);
        let effects = list([fx, n(3)]).effects().unwrap();
        assert_eq!(effects[0].decode(), Ok(Decoded::Exit));
        assert_eq!(effects[1].decode(), Err(Refused::EffectTag));
        assert_eq!(effects[2].decode(), Err(Refused::Effect));
    }

    // What the safe constructors write.

    #[test]
    fn the_constructors_write_the_abi_words() {
        assert_eq!(
            words_of_effect(Effect::print(TEXT)),
            [0, address(TEXT.as_ptr()), 18]
        );
        assert_eq!(words_of_effect(Effect::print(b"")), [0, 0, 0]);
        assert_eq!(words_of_effect(Effect::EXIT), [1, 0, 0]);
        assert_eq!(words_of_list(List::UNWRITTEN), [0, 1]);
        assert_eq!(words_of_list(List::new(&[])), [0, 0]);
        let fx: &'static [Effect] = Box::leak(Box::new([Effect::print(TEXT), Effect::EXIT]));
        assert_eq!(words_of_list(List::new(fx)), [address(fx.as_ptr()), 2]);
    }

    // The entry's arguments.

    #[test]
    fn a_null_update_word_is_none() {
        // SAFETY: `Option` of a function pointer is one word, and the null word is its `None`.
        let update: Option<Update> = unsafe { transmute(n(0)) };
        assert!(update.is_none());
    }

    fn set_sigpipe(handler: usize) -> usize {
        // SAFETY: 13 is a signal whose action may be changed, and both handlers used here,
        // `SIG_DFL` and `SIG_IGN`, install no code.
        unsafe { signal(SIGPIPE, handler) }
    }

    // One test, because the signal's action belongs to the whole process.
    #[test]
    fn the_wrapper_and_the_entry_set_sigpipe_to_ignored() {
        set_sigpipe(SIG_DFL);
        ignore_sigpipe();
        assert_eq!(set_sigpipe(SIG_DFL), SIG_IGN);
        // SAFETY: the name is 5 constant bytes and there is no update function.
        let status = unsafe { paka_rt1_run(b"light".as_ptr(), 5, None, 0, 1) };
        assert_eq!(set_sigpipe(SIG_IGN), SIG_IGN);
        assert_eq!(status, INTERNAL_ERROR);
    }
}
