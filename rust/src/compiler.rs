//! Ruby source turned into what runs in an `mrb_state`, whoever holds it: a
//! realm running a file or its own Ruby, or a gem installing what it ships.

use std::ffi::CStr;

use beni::{Ccontext, Error, Mrb};

/// What the compiler warns about in the source it ran, at the line it names.
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
    for warning in context.warnings() {
        warned(Warning {
            line: warning.line().into(),
            message: warning.message().to_owned(),
        });
    }
    outcome.map(|_| ())
}
