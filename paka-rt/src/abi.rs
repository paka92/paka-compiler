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
const EFFECT_BYTES: u64 = 24;

const PRINT: u64 = 0;
const EXIT: u64 = 1;

const SIGPIPE: i32 = 13;
const SIGXFSZ: i32 = 25;
const SIG_IGN: usize = 1;

const _: () = assert!(size_of::<usize>() == 8);
const _: () = assert!(size_of::<Str>() == 16 && align_of::<Str>() == 8);
const _: () = assert!(size_of::<Effect>() as u64 == EFFECT_BYTES && align_of::<Effect>() == 8);
const _: () = assert!(size_of::<List>() == 16 && align_of::<List>() == 8);
const _: () = assert!(MAX_EFFECTS == MAX_BYTES / EFFECT_BYTES);

/// `count` as a slice length, when `count` items of `size` bytes at `addr` may become one slice:
/// `addr` is not 0, the items take at most 2^63 - 1 bytes, and `addr` plus those bytes is at
/// most 2^64 - 1. So a range whose last byte is at the last address is refused too.
fn span(addr: usize, count: u64, size: u64) -> Option<usize> {
    let bytes = count
        .checked_mul(size)
        .filter(|&bytes| bytes <= MAX_BYTES)?;
    addr.checked_add(usize::try_from(bytes).ok()?)?;
    if addr == 0 {
        return None;
    }
    usize::try_from(count).ok()
}

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

    /// The string's bytes, or `None` when the string is refused. With a length other than 0 that
    /// is: a null pointer, a length above 2^63 - 1, or pointer + length above 2^64 - 1.
    pub fn bytes(&self) -> Option<&'static [u8]> {
        if self.len == 0 {
            return Some(&[]);
        }
        let len = span(self.ptr.addr(), self.len, 1)?;
        // SAFETY: `from_raw_parts` asks for a non-null pointer, aligned for `u8` (any pointer
        // is), at most `isize::MAX` bytes that do not wrap around the address space, and `len`
        // readable, initialized bytes that nothing writes or frees during `'static`. `span`
        // checked the first four. The last cannot be checked: it is the module invariant, owed
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
    /// a pointer off an 8-byte boundary, whatever the count; and with a count other than 0, a
    /// null pointer, a count above `MAX_EFFECTS`, or pointer + 24 x count above 2^64 - 1.
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
        let count = span(self.ptr.addr(), self.count, EFFECT_BYTES)?;
        // SAFETY: `from_raw_parts` asks for a non-null pointer aligned for `Effect`, at most
        // `isize::MAX` bytes that do not wrap around the address space, and `count` readable,
        // initialized effects that nothing writes or frees during `'static`. The alignment
        // is checked above and `span` checked the other three, and any three initialized words
        // are a valid `Effect`. That the memory
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

/// Sets SIGPIPE and SIGXFSZ to ignored, so a write to a closed pipe or past a file-size limit
/// fails with EPIPE or EFBIG.
pub fn ignore_signals() {
    for signum in [SIGPIPE, SIGXFSZ] {
        // SAFETY: `signal` asks for a signal whose action may be changed and a handler that is
        // `SIG_DFL`, `SIG_IGN` or a function. 13 is SIGPIPE, 25 is SIGXFSZ and 1 is `SIG_IGN` on
        // Linux, so no code is installed. The call touches no memory of ours and may be made
        // from any thread.
        unsafe { signal(signum, SIG_IGN) };
    }
}

/// The entry `main` calls (ABI section 4).
///
/// # Safety
///
/// `name` and `name_len` are a string as ABI section 2 defines it.
///
/// `update` is null or a function that keeps ABI section 3 for every message word and however
/// often it is called: `Update` is a safe type, so safe code may call it with anything. It writes
/// at most the 16 bytes of `out`, reads nothing through it, keeps no pointer and does not unwind.
///
/// Every list `update` stores is as section 2 defines it: all `count` effects are readable, and
/// each effect's pointer is null or points at its `len` bytes, unwritten and unfreed for the
/// life of the process.
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
    ignore_signals();
    // SAFETY: the caller promises that `name` and `name_len` are an ABI string, which is what
    // `from_raw` asks for.
    let name = unsafe { Str::from_raw(name, name_len) };
    run(name, update, start, grants)
}

/// Test only: an effect holding these words. They must make a value the decoder refuses, or one
/// that reads nothing through its pointer.
#[cfg(test)]
pub(crate) const fn effect_of_words(words: [u64; 3]) -> Effect {
    Effect {
        tag: words[0],
        ptr: ptr::without_provenance(words[1] as usize),
        len: words[2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::INTERNAL_ERROR;
    use std::io::{self, ErrorKind, Write};
    use std::mem::transmute;

    /// One 8-byte ABI word. It is a pointer type so that a word holding an address keeps the
    /// right to read through it; a word holding a number is made by `n`.
    type Word = *const u8;

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

    fn number(word: Word) -> u64 {
        word.addr() as u64
    }

    // The words given to these three must make a value its decoder refuses, or point at memory
    // that lives for the whole process.

    fn string([ptr, len]: [Word; 2]) -> Str {
        let len = number(len);
        Str { ptr, len }
    }

    fn effect([tag, ptr, len]: [Word; 3]) -> Effect {
        let (tag, len) = (number(tag), number(len));
        Effect { tag, ptr, len }
    }

    fn list([ptr, count]: [Word; 2]) -> List {
        let (ptr, count) = (ptr.cast(), number(count));
        List { ptr, count }
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

    fn set_action(signum: i32, handler: usize) -> usize {
        // SAFETY: 13 and 25 are signals whose action may be changed, and the two handlers the
        // test passes, 0 (`SIG_DFL`) and 1 (`SIG_IGN`), install no code.
        unsafe { signal(signum, handler) }
    }

    fn set_sigpipe(handler: usize) -> usize {
        set_action(13, handler)
    }

    /// The error of a write to a pipe whose read end is closed. With SIGPIPE at its default the
    /// write kills the test process instead.
    fn write_to_closed_pipe() -> ErrorKind {
        let (reader, mut writer) = io::pipe().unwrap();
        drop(reader);
        writer.write(b"x").unwrap_err().kind()
    }

    // One test, because the signal's action belongs to the whole process. 0 is `SIG_DFL` and 1
    // is `SIG_IGN`, written out so the test does not lean on the module's own constant.
    #[test]
    fn the_wrapper_and_the_entry_set_sigpipe_to_ignored() {
        set_sigpipe(0);
        ignore_signals();
        assert_eq!(write_to_closed_pipe(), ErrorKind::BrokenPipe);
        assert_eq!(set_sigpipe(0), 1);
        // SAFETY: the name is 5 constant bytes and there is no update function.
        let status = unsafe { paka_rt1_run(b"light".as_ptr(), 5, None, 0, 1) };
        assert_eq!(write_to_closed_pipe(), ErrorKind::BrokenPipe);
        assert_eq!(set_sigpipe(1), 1);
        assert_eq!(status, INTERNAL_ERROR);
    }

    // Own test and own signal: the file-size signal's action is also process-wide, but no other
    // test touches it. 25 is SIGXFSZ, written out as above.
    #[test]
    fn the_wrapper_sets_sigxfsz_to_ignored() {
        set_action(25, 0);
        ignore_signals();
        assert_eq!(set_action(25, 0), 1);
    }

    // The limits, at their edges. `span` forms no slice, so no memory has to be there.

    const LAST: usize = usize::MAX;

    #[test]
    fn a_null_address_has_no_span() {
        assert_eq!(span(0, 1, 1), None);
        assert_eq!(span(0, 1, EFFECT_BYTES), None);
        assert_eq!(span(1, 1, 1), Some(1));
    }

    #[test]
    fn a_span_is_at_most_the_longest_string() {
        assert_eq!(span(8, MAX_BYTES, 1), Some(MAX_BYTES as usize));
        assert_eq!(span(8, MAX_BYTES + 1, 1), None);
    }

    #[test]
    fn a_span_is_at_most_the_longest_list() {
        assert_eq!(
            span(8, MAX_EFFECTS, EFFECT_BYTES),
            Some(MAX_EFFECTS as usize)
        );
        assert_eq!(span(8, MAX_EFFECTS + 1, EFFECT_BYTES), None);
    }

    #[test]
    fn a_span_ends_at_most_at_the_last_address() {
        assert_eq!(span(LAST - 5, 5, 1), Some(5));
        assert_eq!(span(LAST - 4, 5, 1), None);
        assert_eq!(span(LAST - 24, 1, EFFECT_BYTES), Some(1));
        assert_eq!(span(LAST - 23, 1, EFFECT_BYTES), None);
    }
}
