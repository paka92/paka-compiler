//! The shape of a `foreign` declaration (docs/decisions/M7-02.md section 2): one header and a
//! library or source to build, plain file names, the four plain types across and a C name behind
//! each effect.

use super::{Res, Resolver};
use crate::ast::{ForeignLine, Name, Param, Type};

const ARGUMENTS: [&str; 4] = ["Int", "Float", "Bool", "String"];

impl<'a> Resolver<'a> {
    pub(super) fn foreign(&self, name: &Name, lines: &[ForeignLine]) -> Res {
        let (mut headers, mut built) = (0, false);
        for line in lines {
            match line {
                ForeignLine::File(key, file) => {
                    self.file_name(file)?;
                    if key.text != "header" {
                        built = true;
                    } else if headers == 1 {
                        return Err(self.bad(
                            key,
                            "one header: per declaration; include the others from it",
                        ));
                    } else {
                        headers = 1;
                    }
                }
                ForeignLine::Function {
                    params,
                    result,
                    symbol,
                    ..
                } => {
                    self.params(params)?;
                    if let Some(t) = result {
                        self.crossing(
                            t,
                            &ARGUMENTS[..3],
                            "a result is an Int, Float or Bool, or none",
                        )?;
                    }
                    let mut chars = symbol.text.chars();
                    let ident = |c: char| c == '_' || c.is_ascii_alphanumeric();
                    if !chars
                        .next()
                        .is_some_and(|c| ident(c) && !c.is_ascii_digit())
                        || !chars.all(ident)
                    {
                        return Err(self.bad(symbol, "the C name is a C identifier"));
                    }
                }
            }
        }
        if headers == 0 || !built {
            return Err(self.bad(
                name,
                "a foreign declaration names one header: and a library: or source:",
            ));
        }
        Ok(())
    }

    /// A name beside the program: no path, no leading dash, nothing escaped.
    fn file_name(&self, file: &Name) -> Res {
        let text = file.text;
        let plain = !matches!(text, "" | "." | "..")
            && !text.starts_with('-')
            && !text.contains(['/', '\\', '{']);
        match plain {
            true => Ok(()),
            false => Err(self.bad(
                file,
                "a file name is plain, beside the program: no path, no leading dash",
            )),
        }
    }

    fn params(&self, params: &[Param]) -> Res {
        for (i, p) in params.iter().enumerate() {
            if p.inout {
                return Err(self.bad(&p.name, "a foreign argument is copied; drop inout"));
            }
            let taken = params[..i].iter().any(|q| q.name.text == p.name.text);
            if taken || matches!(p.name.text, "then" | "or") {
                let message = match taken {
                    true => format!("{} is already a parameter here", p.name.text),
                    false => format!("{} labels the answer; rename the parameter", p.name.text),
                };
                let at = (p.name.at, p.name.text.len());
                return Err(self.fail(at.0, at.1, "duplicate-name", message));
            }
            self.crossing(
                &p.ty,
                &ARGUMENTS,
                "an argument is an Int, Float, Bool or String",
            )?;
        }
        Ok(())
    }

    /// `t` is one of the plain types `allowed`.
    fn crossing(&self, t: &Type, allowed: &[&str], rule: &str) -> Res {
        match t {
            Type::Name { name, args } if args.is_empty() && allowed.contains(&name.text) => Ok(()),
            Type::Name { name, .. } => Err(self.bad(name, rule)),
            Type::Fn { at, .. } => Err(self.fail(*at, 2, "bad-foreign", rule.into())),
        }
    }

    fn bad(&self, at: &Name, message: &str) -> crate::diagnostic::Diagnostic {
        self.fail(at.at, at.text.len().max(1), "bad-foreign", message.into())
    }
}
