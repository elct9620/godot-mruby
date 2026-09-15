# Compiler

How the extension turns Ruby source into what runs in an `mrb_state`, whoever holds the state: a realm running a game file or its own Ruby, and a gem installing the Ruby it ships. It sits outside both, so neither reaches into the other to run Ruby.

## Includes

- `rust/src/compiler.rs`

## `run`

Compiles and runs Ruby source under a name, which mruby stamps on everything compiled from it, so warnings, errors and backtraces name it; what the compiler warns about is written to the log at its line.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn run(mrb: &Mrb, name: &CStr, source: &str) -> Result<(), Error> {}
```
