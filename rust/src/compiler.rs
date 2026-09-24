//! Ruby source turned into what runs in an `mrb_state`, whoever holds it: a
//! realm running a file or its own Ruby, or a gem installing what it ships.
//! Source only checked is compiled in a state of its own.

use std::ffi::CStr;

use beni::{Ccontext, Error, Mrb};

/// What checking source found: the error that stopped it compiling, if any,
/// and every warning.
pub struct Diagnostics {
    pub error: Option<CompileError>,
    pub warnings: Vec<Warning>,
}

/// Why source does not compile, at the 1-based line and column it names.
pub struct CompileError {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

/// What the compiler warns about in the source it compiled, at the line it
/// names.
pub struct Warning {
    pub line: u32,
    pub message: String,
}

/// Compiles and runs `source` under `name`: mruby stamps the name on
/// everything compiled from it, so warnings, errors and backtraces name it.
/// Each warning goes to `warned` once the source has run, and the caller
/// decides where it is reported.
pub fn run(
    mrb: &Mrb,
    name: &CStr,
    source: &str,
    mut warned: impl FnMut(Warning),
) -> Result<(), Error> {
    let Some(context) = Ccontext::new(mrb, name) else {
        let class = mrb.exc_get(c"RuntimeError")?;
        return Err(Error::new(
            mrb,
            class,
            "mruby could not make a compile context",
        ));
    };
    let outcome = context.load_nstring(source.as_bytes());
    warnings_of(&context).into_iter().for_each(&mut warned);
    outcome.map(|_| ())
}

/// What the compiler says of `source` under `name`, compiled and never run.
/// A new state compiles it, so this belongs to no realm, never waits for one
/// and prints nothing.
pub fn diagnostics(name: &CStr, source: &str) -> Diagnostics {
    let error = |message: String| CompileError {
        line: 1,
        column: 1,
        message,
    };
    let mrb = match Mrb::open() {
        Ok(mrb) => mrb,
        Err(opened) => {
            return Diagnostics {
                error: Some(error(format!("mruby could not open a state: {opened}"))),
                warnings: Vec::new(),
            };
        }
    };
    let Some(context) = Ccontext::new(&mrb, name) else {
        return Diagnostics {
            error: Some(error("mruby could not make a compile context".to_owned())),
            warnings: Vec::new(),
        };
    };
    let error = match context.compile(source.as_bytes()) {
        Ok(_) => None,
        // mruby records where its lexer stood after the offending token,
        // which counted from 1 is that token's last character; an error at
        // the end of the file, or one it recorded no position for, stands
        // at 0, and Godot counts from 1.
        Err(Error::Syntax(parsed)) => Some(CompileError {
            line: u32::from(parsed.line()).max(1),
            column: u32::try_from(parsed.column()).unwrap_or(0).max(1),
            message: parsed.message().to_owned(),
        }),
        Err(failed) => Some(error(failed.message(&mrb))),
    };
    Diagnostics {
        error,
        warnings: warnings_of(&context),
    }
}

fn warnings_of(context: &Ccontext) -> Vec<Warning> {
    context
        .warnings()
        .into_iter()
        .map(|warning| Warning {
            line: warning.line().into(),
            message: warning.message().to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostics_of(source: &str) -> Diagnostics {
        diagnostics(c"res://checked.rb", source)
    }

    // @behavior RK-001
    #[test]
    fn a_syntax_error_comes_back_at_its_line_and_column() {
        let checked = diagnostics_of("x = 1\nx = )\n");

        let error = checked.error.map(|error| (error.line, error.column));
        assert_eq!(error, Some((2, 5)));
    }

    // @behavior RK-002
    #[test]
    fn a_compiler_warning_comes_back_at_its_line() {
        let checked = diagnostics_of("begin\n  :body\nelse\n  :useless\nend\n");

        let lines: Vec<u32> = checked
            .warnings
            .iter()
            .map(|warning| warning.line)
            .collect();
        assert_eq!(lines, vec![5]);
    }

    // @behavior RK-003
    #[test]
    fn checked_source_never_runs() {
        let checked = diagnostics_of("raise 'ran'\n");

        assert!(checked.error.is_none());
    }
}
