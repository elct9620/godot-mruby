# Compiler

How the extension turns Ruby source into what runs in an `mrb_state`, whoever holds the state: a realm running a game file or its own Ruby, and a gem installing the Ruby it ships. It sits outside both, so neither reaches into the other to run Ruby. Source only checked is compiled in a state of its own, which belongs to no realm and never waits for one.

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

## `diagnostics`

What the compiler says of Ruby source under a name, compiled in a new state and never run, so it answers on any thread without waiting for a realm or printing.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn diagnostics(name: &CStr, source: &str) -> Diagnostics {}
```

## `Diagnostics`

What checking source found: the error that stopped it compiling, if any, and every warning.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Diagnostics;
```

## `CompileError`

Why source does not compile, at the 1-based line and column it names; one with no position names the first line and column.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct CompileError;
```

## `Warning`

What the compiler warns about in the source it compiled, at the line it names.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Warning;
```
