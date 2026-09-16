# Compiler

How the extension turns Ruby source into what runs in an `mrb_state`, whoever holds the state: a realm running a game file or its own Ruby, and a gem installing the Ruby it ships. It sits outside both, so neither reaches into the other to run Ruby.

## Includes

- `rust/src/compiler.rs`

## `run`

Compiles and runs Ruby source under a name, which mruby stamps on everything compiled from it, so warnings, errors and backtraces name it; each warning the compiler gives is handed to the caller, who decides where it is reported.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn run(mrb: &Mrb, name: &CStr, source: &str, warned: impl FnMut(Warning)) -> Result<(), Error> {}
```

## `Warning`

What the compiler warns about in the source it ran, at the line it names.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Warning;
```
