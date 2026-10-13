//! The reference interpreter: performs the effects `update` returns (spec/m1.md section 9).

use crate::lower::{Core, Effect, Grant};
use std::io::{self, ErrorKind, Write};

/// Runs the program; returns its status.
pub fn run(core: &Core, out: &mut impl Write, err: &mut impl Write) -> u8 {
    match stop(core, out) {
        None => 0,
        Some(reason) => {
            let _ = err.write_all(format!("{}: {reason}\n", core.app).as_bytes());
            3
        }
    }
}

/// Performs the effects in order; the reason the program stopped, or `None` after `Exit`.
fn stop(core: &Core, out: &mut impl Write) -> Option<String> {
    for effect in &core.update {
        match effect {
            Effect::Exit => return None,
            Effect::Print(_) if !core.grants.contains(&Grant::Console) => {
                return Some("effect Print refused: grant Console is missing".into());
            }
            Effect::Print(text) => {
                if let Err(e) = print(out, text) {
                    return Some(format!("effect Print failed: {}", reason(&e)));
                }
            }
        }
    }
    Some("ended without Exit".into())
}

/// The text and its line feed in one buffer, then a flush.
fn print(out: &mut impl Write, text: &str) -> io::Result<()> {
    out.write_all(format!("{text}\n").as_bytes())?;
    out.flush()
}

fn reason(e: &io::Error) -> &'static str {
    match e.kind() {
        ErrorKind::StorageFull => "no space left on device",
        ErrorKind::BrokenPipe => "broken pipe",
        _ => "write error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::{Effect, Grant};
    use std::io::{self, ErrorKind};

    fn print(text: &str) -> Effect {
        Effect::Print(text.into())
    }

    fn core(grants: Vec<Grant>, update: Vec<Effect>) -> Core {
        Core {
            app: "light".into(),
            variants: vec!["Start".into()],
            start: 0,
            grants,
            update_name: "update".into(),
            update,
        }
    }

    /// Runs on in-memory streams; returns (stdout, stderr, status).
    fn go(core: &Core) -> (String, String, u8) {
        let (mut out, mut err) = (vec![], vec![]);
        let status = run(core, &mut out, &mut err);
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
            status,
        )
    }

    fn console(update: Vec<Effect>) -> Core {
        core(vec![Grant::Console], update)
    }

    #[test]
    fn print_then_exit() {
        let c = console(vec![print("let there be light"), Effect::Exit]);
        assert_eq!(go(&c), ("let there be light\n".into(), "".into(), 0));
    }

    #[test]
    fn empty_print_writes_one_line_feed() {
        let c = console(vec![print(""), Effect::Exit]);
        assert_eq!(go(&c), ("\n".into(), "".into(), 0));
    }

    #[test]
    fn prints_in_order() {
        let c = console(vec![print("a"), print("b"), Effect::Exit]);
        assert_eq!(go(&c), ("a\nb\n".into(), "".into(), 0));
    }

    #[test]
    fn exit_ends_the_list_untested() {
        let c = core(vec![], vec![Effect::Exit, print("x")]);
        assert_eq!(go(&c), ("".into(), "".into(), 0));
    }

    #[test]
    fn empty_list_stops() {
        let c = console(vec![]);
        assert_eq!(go(&c), ("".into(), "light: ended without Exit\n".into(), 3));
    }

    #[test]
    fn list_without_exit_keeps_its_output_and_stops() {
        let c = console(vec![print("a")]);
        assert_eq!(
            go(&c),
            ("a\n".into(), "light: ended without Exit\n".into(), 3)
        );
    }

    #[test]
    fn missing_grant_refuses_and_drops_the_rest() {
        let c = core(vec![], vec![print("x"), print("y"), Effect::Exit]);
        let want = "light: effect Print refused: grant Console is missing\n";
        assert_eq!(go(&c), ("".into(), want.into(), 3));
    }

    #[test]
    fn missing_grant_is_tested_at_the_effects_turn() {
        let c = core(vec![], vec![Effect::Exit, print("x")]);
        assert_eq!(go(&c).2, 0);
    }

    /// Accepts `room` bytes one call at a time, then fails with `kind`; logs its calls.
    struct Faulty {
        room: usize,
        kind: ErrorKind,
        log: Vec<String>,
    }

    impl Write for Faulty {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.room == 0 {
                return Err(self.kind.into());
            }
            self.room -= 1;
            self.log.push(String::from_utf8_lossy(&buf[..1]).into());
            Ok(1)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.log.push("flush".into());
            Ok(())
        }
    }

    fn faulty(room: usize, kind: ErrorKind, update: Vec<Effect>) -> (Faulty, String, u8) {
        let mut out = Faulty {
            room,
            kind,
            log: vec![],
        };
        let mut err = vec![];
        let status = run(&console(update), &mut out, &mut err);
        (out, String::from_utf8(err).unwrap(), status)
    }

    #[test]
    fn short_writes_are_continued_and_each_print_is_flushed() {
        let (out, err, status) = faulty(
            9,
            ErrorKind::Other,
            vec![print("ab"), print("c"), Effect::Exit],
        );
        assert_eq!(out.log, ["a", "b", "\n", "flush", "c", "\n", "flush"]);
        assert_eq!((err.as_str(), status), ("", 0));
    }

    #[test]
    fn failed_write_reasons() {
        for (kind, reason) in [
            (ErrorKind::StorageFull, "no space left on device"),
            (ErrorKind::BrokenPipe, "broken pipe"),
            (ErrorKind::PermissionDenied, "write error"),
        ] {
            let (_, err, status) = faulty(0, kind, vec![print("x"), Effect::Exit]);
            assert_eq!(err, format!("light: effect Print failed: {reason}\n"));
            assert_eq!(status, 3);
        }
    }

    #[test]
    fn failed_write_drops_the_rest_and_keeps_bytes_already_written() {
        let (out, err, status) = faulty(
            2,
            ErrorKind::BrokenPipe,
            vec![print("a"), print("b"), Effect::Exit],
        );
        assert_eq!(out.log, ["a", "\n", "flush"]);
        assert_eq!(err, "light: effect Print failed: broken pipe\n");
        assert_eq!(status, 3);
    }

    #[test]
    fn write_that_accepts_nothing_is_a_failure() {
        struct Zero;
        impl Write for Zero {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Ok(0)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut err = vec![];
        let status = run(
            &console(vec![print("x"), Effect::Exit]),
            &mut Zero,
            &mut err,
        );
        assert_eq!(err, b"light: effect Print failed: write error\n");
        assert_eq!(status, 3);
    }

    #[test]
    fn failed_standard_error_changes_no_status() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let status = run(&console(vec![]), &mut vec![], &mut Broken);
        assert_eq!(status, 3);
    }
}
