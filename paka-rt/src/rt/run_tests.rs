//! The run loop, driven by fake update functions and captured sinks. Nothing here is unsafe.

use super::{Stop, buffer, run_with, stop};
use crate::abi::{Effect, List, Update, effect_of_words};
use std::io::{self, ErrorKind, Write};
use std::{fs, path::Path};

/// A fake `update`: returns 0 and stores the effects when called with `$start`, else status 99.
macro_rules! fake {
    ($name:ident, $start:expr, [$($effect:expr),*]) => {
        extern "C" fn $name(msg: u64, out: &mut List) -> i32 {
            const FX: &[Effect] = &[$($effect),*];
            if msg != $start {
                return 99;
            }
            *out = List::new(FX);
            0
        }
    };
}

const WIDE: u64 = 0xfedc_ba98_7654_3210;

const LIGHT: Effect = Effect::print(b"let there be light");

fake!(light, 0, [LIGHT, Effect::EXIT]);
fake!(light_at_1, 1, [LIGHT, Effect::EXIT]);
fake!(nothing, 0, []);
fake!(empty_print, 0, [Effect::print(b""), Effect::EXIT]);
fake!(
    exit_first,
    0,
    [Effect::EXIT, Effect::print(b"never printed")]
);
fake!(no_exit, 0, [LIGHT]);
fake!(
    print_twice,
    0,
    [LIGHT, Effect::print(b"and there was light"), Effect::EXIT]
);
fake!(
    symbols,
    0,
    [
        Effect::print(b"// ??rest 42 then: 'x' -> [<a>, (b)] = #"),
        Effect::EXIT
    ]
);
fake!(exit_at_7, 7, [Effect::EXIT]);
fake!(exit_at_wide, WIDE, [Effect::EXIT]);
fake!(unknown_tag, 0, [LIGHT, effect_of_words([2, 0, 0])]);
fake!(bad_print, 0, [effect_of_words([0, 0, 18])]);
fake!(bad_exit, 0, [effect_of_words([1, 8, 0])]);
fake!(exit_then_bad, 0, [Effect::EXIT, effect_of_words([2, 0, 0])]);

extern "C" fn status_1(_msg: u64, out: &mut List) -> i32 {
    *out = List::new(&[Effect::EXIT]);
    1
}

extern "C" fn status_1_stores_nothing(_msg: u64, _out: &mut List) -> i32 {
    1
}

extern "C" fn stores_nothing(_msg: u64, _out: &mut List) -> i32 {
    0
}

type Run = (String, String, i32);

fn go_with(app: Option<&'static [u8]>, update: Option<Update>, start: u64, grants: u64) -> Run {
    go_with_args(app, update, start, grants, 0)
}

fn go_with_args(
    app: Option<&'static [u8]>,
    update: Option<Update>,
    start: u64,
    grants: u64,
    args: usize,
) -> Run {
    let (mut out, mut err) = (vec![], vec![]);
    let status = run_with(&mut out, &mut err, app, update, start, grants, args);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        status,
    )
}

/// The app `light`, started with message 0.
fn go(update: Update, grants: u64) -> Run {
    go_with(Some(b"light"), Some(update), 0, grants)
}

fn internal(reason: &str) -> String {
    format!("light: internal error: {reason}\n")
}

// The cases under tests/cases/run.

struct Case {
    dir: &'static str,
    app: &'static [u8],
    start: u64,
    grants: u64,
    update: Update,
}

const fn case(dir: &'static str, update: Update, grants: u64) -> Case {
    Case {
        dir,
        app: b"light",
        start: 0,
        grants,
        update,
    }
}

// Transcribed by hand from each case's main.paka: the app name, the position of `start:` in its
// enum, the `grants:` list (Console is bit 0) and the list `update` returns.
const CASES: [Case; 17] = [
    case("app_first", light, 1),
    case("empty_list", nothing, 0),
    case("empty_string", empty_print, 1),
    case("exit_first", exit_first, 0),
    case("fewest_spaces", light, 1),
    Case {
        app: b"apps",
        ..case("keyword_like_names", light, 1)
    },
    case("light", light, 1),
    case("light_extra_argument", light, 1),
    case("light_no_console", light, 0),
    case("light_two_arguments", light, 1),
    Case {
        app: b"light_2",
        ..case("names_with_digits", light, 1)
    },
    case("no_exit", no_exit, 1),
    case("no_final_newline", light, 1),
    case("print_twice", print_twice, 1),
    case("string_symbols", symbols, 1),
    Case {
        start: 1,
        update: light_at_1,
        ..case("two_variants", light, 1)
    },
    case("unused_definitions", light, 1),
];

/// The cases the milestone 1 back end cannot build (a state, a start message with fields,
/// `ExitWith`); the interpreter alone runs them.
const BEYOND: [&str; 10] = [
    "exit_with_computed_four",
    "exit_with_computed_high",
    "exit_with_computed_negative",
    "exit_with_five",
    "exit_with_max",
    "exit_with_skips_rest",
    "exit_with_zero",
    "start_two_fields",
    "start_with_argument",
    "state_in_handler",
];

/// The arguments a case is run with: one per line of its `args` file.
fn argument_count(dir: &Path) -> usize {
    fs::read_to_string(dir.join("args")).map_or(0, |text| text.lines().count())
}

#[test]
fn every_run_case_gets_its_expected_output_and_status() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/cases/run");
    let mut found: Vec<String> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();
    found.sort();
    assert!(!found.is_empty(), "no cases found");
    let mut faked: Vec<&str> = CASES.iter().map(|case| case.dir).collect();
    faked.extend(BEYOND);
    faked.sort();
    assert_eq!(found, faked, "the cases on disk and the fakes differ");
    for case in &CASES {
        let dir = root.join(case.dir);
        let stdout = fs::read_to_string(dir.join("stdout")).unwrap();
        let stderr = fs::read_to_string(dir.join("stderr")).unwrap_or_default();
        let status = fs::read_to_string(dir.join("status")).unwrap();
        let want = (stdout, stderr, status.trim().parse().unwrap());
        let args = argument_count(&dir);
        let got = go_with_args(
            Some(case.app),
            Some(case.update),
            case.start,
            case.grants,
            args,
        );
        assert_eq!(got, want, "case {}", case.dir);
    }
}

// Start, grants and effects.

#[test]
fn the_start_message_is_passed_unchanged() {
    for start in [7, 0, u64::MAX] {
        let got = go_with(Some(b"light"), Some(exit_at_7), start, 0);
        let status = if start == 7 { 0 } else { 4 };
        assert_eq!(got.2, status, "start {start}");
    }
}

#[test]
fn the_whole_start_message_is_passed() {
    assert_eq!(go_with(Some(b"light"), Some(exit_at_wide), WIDE, 0).2, 0);
    assert_eq!(
        go_with(Some(b"light"), Some(exit_at_wide), WIDE >> 32, 0).2,
        4
    );
}

#[test]
fn a_print_without_console_is_refused_and_writes_nothing() {
    let want = "light: effect Print refused: grant Console is missing\n";
    assert_eq!(go(print_twice, 0), ("".into(), want.into(), 3));
}

#[test]
fn exit_ends_the_list_without_decoding_the_rest() {
    assert_eq!(go(exit_then_bad, 0), ("".into(), "".into(), 0));
    assert_eq!(go(exit_then_bad, 1), ("".into(), "".into(), 0));
}

#[test]
fn a_print_is_one_write_of_text_and_line_feed_and_nothing_is_flushed() {
    let (out, _, status) = with_out(Sink::default(), print_twice);
    assert_eq!(status, 0);
    assert_eq!(out.calls, ["let there be light\n", "and there was light\n"]);
    assert_eq!(out.flushes, 0);
}

// Arguments: the start message of every program this ABI carries has no fields.

#[test]
fn any_argument_is_a_usage_error_before_update_is_called() {
    for (args, line) in [(1, "1"), (2, "2"), (40, "40")] {
        let got = go_with_args(Some(b"light"), Some(status_1), 0, 1, args);
        let want = format!("light: expected 0 arguments, got {line}\n");
        assert_eq!(got, ("".into(), want, 2));
    }
}

#[test]
fn the_usage_line_is_written_without_allocating_in_pieces() {
    let mut err = Sink::default();
    let mut out = Sink::default();
    let status = run_with(&mut out, &mut err, Some(b"light"), Some(light), 0, 1, 3);
    assert_eq!(err.calls.concat(), "light: expected 0 arguments, got 3\n");
    assert_eq!((out.calls.len(), status), (0, 2));
}

#[test]
fn refused_inputs_come_before_the_arguments_are_counted() {
    let got = go_with_args(Some(b"light"), None, 0, 1, 1);
    assert_eq!(got, ("".into(), internal("update is null"), 4));
    let got = go_with_args(Some(b"light"), Some(light), 0, 2, 1);
    assert_eq!(got, ("".into(), internal("bad grants"), 4));
}

// Internal errors: status 4 and the reasons of ABI section 7.

#[test]
fn a_refused_app_name_is_a_question_mark() {
    let got = go_with(None, Some(light), 0, 1);
    assert_eq!(
        got,
        ("".into(), "?: internal error: bad app name\n".into(), 4)
    );
}

#[test]
fn grants_other_than_bit_0_are_refused_before_anything_runs() {
    for grants in [2, 3, 1 << 63, u64::MAX] {
        assert_eq!(go(light, grants), ("".into(), internal("bad grants"), 4));
    }
}

#[test]
fn bad_grants_are_refused_before_a_null_update() {
    let got = go_with(Some(b"light"), None, 0, 2);
    assert_eq!(got, ("".into(), internal("bad grants"), 4));
}

#[test]
fn bad_grants_are_refused_before_update_is_called() {
    for update in [status_1, status_1_stores_nothing, stores_nothing] {
        assert_eq!(go(update, 2), ("".into(), internal("bad grants"), 4));
    }
}

#[test]
fn the_status_is_tested_before_the_list_header_is_read() {
    let reason = internal("update returned a non-zero status");
    assert_eq!(go(status_1_stores_nothing, 1), ("".into(), reason, 4));
}

#[test]
fn a_null_update_is_refused() {
    let got = go_with(Some(b"light"), None, 0, 1);
    assert_eq!(got, ("".into(), internal("update is null"), 4));
}

#[test]
fn a_non_zero_update_status_is_an_internal_error_and_its_list_is_not_read() {
    let reason = internal("update returned a non-zero status");
    assert_eq!(go(status_1, 1), ("".into(), reason, 4));
}

#[test]
fn an_update_that_stores_nothing_gives_a_bad_list() {
    assert_eq!(
        go(stores_nothing, 1),
        ("".into(), internal("bad effect list"), 4)
    );
}

#[test]
fn an_unknown_tag_stops_after_the_effects_before_it() {
    let want = (
        "let there be light\n".into(),
        internal("unknown effect tag"),
        4,
    );
    assert_eq!(go(unknown_tag, 1), want);
}

#[test]
fn an_undecodable_effect_is_bad() {
    for update in [bad_print, bad_exit] {
        assert_eq!(go(update, 1), ("".into(), internal("bad effect"), 4));
    }
}

#[test]
fn an_effect_is_decoded_before_its_grant_is_tested() {
    assert_eq!(go(bad_print, 0), ("".into(), internal("bad effect"), 4));
}

#[test]
fn a_buffer_that_cannot_be_reserved_is_out_of_memory() {
    assert_eq!(buffer(usize::MAX), Err(Stop::Internal("out of memory")));
    let mut err = vec![];
    let status = stop(&mut err, b"light", Stop::Internal("out of memory"));
    assert_eq!(err, internal("out of memory").as_bytes());
    assert_eq!(status, 4);
}

struct Panics;

impl Write for Panics {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        panic!("planted in a test sink");
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_panic_is_caught_and_names_the_app() {
    let mut err = vec![];
    let status = run_with(&mut Panics, &mut err, Some(b"light"), Some(light), 0, 1, 0);
    assert_eq!(err, internal("runtime panicked").as_bytes());
    assert_eq!(status, 4);
}

// Failing sinks.

/// Accepts at most `chunk` bytes per call. After `room` accepted calls it fails with `fail`.
/// `calls` holds the bytes of each accepted call, `flushes` counts the flushes asked for.
struct Sink {
    chunk: usize,
    room: usize,
    fail: Option<ErrorKind>,
    calls: Vec<String>,
    flushes: usize,
}

impl Default for Sink {
    fn default() -> Sink {
        Sink {
            chunk: usize::MAX,
            room: usize::MAX,
            fail: None,
            calls: vec![],
            flushes: 0,
        }
    }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(kind) = self.fail.filter(|_| self.calls.len() >= self.room) {
            return Err(kind.into());
        }
        let taken = buf.len().min(self.chunk);
        let text = String::from_utf8_lossy(&buf[..taken]).into_owned();
        self.calls.push(text);
        Ok(taken)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

fn failing(kind: ErrorKind, room: usize) -> Sink {
    Sink {
        fail: Some(kind),
        room,
        ..Sink::default()
    }
}

fn with_out(mut out: Sink, update: Update) -> (Sink, String, i32) {
    let mut err = vec![];
    let status = run_with(&mut out, &mut err, Some(b"light"), Some(update), 0, 1, 0);
    (out, String::from_utf8(err).unwrap(), status)
}

#[test]
fn a_failed_write_gives_the_specs_reason_and_status_3() {
    for (kind, reason) in [
        (ErrorKind::StorageFull, "no space left on device"),
        (ErrorKind::QuotaExceeded, "no space left on device"),
        (ErrorKind::BrokenPipe, "broken pipe"),
        (ErrorKind::PermissionDenied, "write error"),
    ] {
        let (out, err, status) = with_out(failing(kind, 0), light);
        assert_eq!(err, format!("light: effect Print failed: {reason}\n"));
        assert_eq!((out.calls.len(), status), (0, 3));
    }
}

#[test]
fn a_failed_write_drops_the_rest_and_keeps_what_was_written() {
    let (out, err, status) = with_out(failing(ErrorKind::BrokenPipe, 1), print_twice);
    assert_eq!(out.calls, ["let there be light\n"]);
    assert_eq!(err, "light: effect Print failed: broken pipe\n");
    assert_eq!(status, 3);
}

#[test]
fn short_writes_are_continued() {
    let sink = Sink {
        chunk: 4,
        ..Sink::default()
    };
    let (out, err, status) = with_out(sink, light);
    assert_eq!(out.calls.concat(), "let there be light\n");
    assert_eq!(out.calls.len(), 5);
    assert_eq!((err.as_str(), status), ("", 0));
}

#[test]
fn a_write_that_accepts_nothing_is_a_failure() {
    let sink = Sink {
        chunk: 0,
        ..Sink::default()
    };
    let (_, err, status) = with_out(sink, light);
    assert_eq!(err, "light: effect Print failed: write error\n");
    assert_eq!(status, 3);
}

/// Interrupts its first write when told to, and fails every flush.
struct Flaky {
    interrupt_once: bool,
    bytes: Vec<u8>,
}

impl Write for Flaky {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if std::mem::take(&mut self.interrupt_once) {
            return Err(ErrorKind::Interrupted.into());
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(ErrorKind::BrokenPipe.into())
    }
}

#[test]
fn an_interrupted_write_is_retried() {
    let mut out = Flaky {
        interrupt_once: true,
        bytes: vec![],
    };
    let status = run_with(&mut out, &mut vec![], Some(b"light"), Some(light), 0, 1, 0);
    assert_eq!((out.bytes, status), (b"let there be light\n".to_vec(), 0));
}

#[test]
fn a_flush_is_never_asked_for() {
    let mut out = Flaky {
        interrupt_once: false,
        bytes: vec![],
    };
    let mut err = vec![];
    let status = run_with(&mut out, &mut err, Some(b"light"), Some(light), 0, 1, 0);
    assert_eq!(
        (out.bytes, err, status),
        (b"let there be light\n".to_vec(), vec![], 0)
    );
}

#[test]
fn a_standard_error_line_is_written_in_pieces() {
    let mut err = Sink::default();
    let status = run_with(
        &mut Sink::default(),
        &mut err,
        Some(b"light"),
        Some(nothing),
        0,
        1,
        0,
    );
    assert_eq!(err.calls, ["light", ": ", "ended without Exit", "\n"]);
    assert_eq!(status, 3);
}

#[test]
fn a_failed_standard_error_changes_no_status() {
    let broken = || failing(ErrorKind::BrokenPipe, 0);
    let runs: [(Option<Update>, u64, i32); 3] =
        [(Some(nothing), 1, 3), (Some(light), 0, 3), (None, 1, 4)];
    for (update, grants, want) in runs {
        let status = run_with(
            &mut vec![],
            &mut broken(),
            Some(b"light"),
            update,
            0,
            grants,
            0,
        );
        assert_eq!(status, want);
    }
}
