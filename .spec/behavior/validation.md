# Validation

How the editor learns what is wrong with a Ruby file as it is typed: the source is compiled in a state of its own and never run, and what the compiler says comes back as data rather than through the log.

### What it does not see

- A codegen error comes back only as `codegen error` at the file's first line; mruby 4.0.0 writes its line to the process's standard error.
- Only the first syntax error comes back.
- An unknown method, a mismatched type or a refused declaration shows only when the file runs.

## Includes

- `rust/src/compiler.rs`

## `RK-001` A syntax error comes back at its line and column

| Step | Statement |
| --- | --- |
| Given | source whose second line does not parse |
| When | the source is checked |
| Then | the error names the second line and the column of the token it stopped at |

## `RK-002` A compiler warning comes back at its line

| Step | Statement |
| --- | --- |
| Given | source whose `begin` block has a useless `else` and ends on its fifth line |
| When | the source is checked |
| Then | the warning names the fifth line |

## `RK-003` Checked source never runs

| Step | Statement |
| --- | --- |
| Given | source whose top level raises |
| When | the source is checked |
| Then | no error comes back |
