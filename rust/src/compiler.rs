//! Ruby source turned into what runs in an `mrb_state`, whoever holds it: a
//! realm running a file or its own Ruby, or a gem installing what it ships.

use std::ffi::CStr;

use beni::{Ccontext, Error, Mrb};

use crate::log::Location;
use crate::warn;

/// Compiles and runs `source` under `name`: mruby stamps the name on
/// everything compiled from it, so warnings, errors and backtraces name it.
/// What the compiler warns about is written to Godot's log at its line.
pub fn run(mrb: &Mrb, name: &CStr, source: &str) -> Result<(), Error> {
    let Some(context) = Ccontext::new(mrb, name) else {
        let class = mrb.exc_get(c"RuntimeError")?;
        return Err(Error::new(
            mrb,
            class,
            "mruby could not make a compile context",
        ));
    };
    let outcome = context.load_nstring(source.as_bytes());
    let file = name.to_string_lossy();
    for warning in context.warnings() {
        let at = Location {
            file: file.clone().into_owned(),
            line: warning.line().into(),
        };
        warn!(at: &at, "{}", warning.message());
    }
    outcome.map(|_| ())
}
