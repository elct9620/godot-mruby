# Validation

How the editor learns what is wrong with a Ruby file as it is typed: the source is compiled in a state of its own and never run, and what the compiler says comes back as data rather than through the log, with what the project's other files make of the file's name.

### What it does not see

- A codegen error comes back only as `codegen error` at the file's first line; mruby 4.0.0 writes its line to the process's standard error.
- Only the first syntax error comes back.
- An unknown method, a mismatched type or a refused declaration shows only when the file runs.
- A file naming a constant the realm already has is warned of only as the realm opens: only the realm knows what it has.

## Includes

- `rust/src/compiler.rs`
- `rust/src/validation.rs`

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

## `RK-004` A file naming what another file names is warned of

| Step | Statement |
| --- | --- |
| Given | two files whose paths name the same constant |
| When | one of them is checked |
| Then | a warning at its first line names the other file |

## `RK-005` A node script sharing its name is warned of

| Step | Statement |
| --- | --- |
| Given | two node scripts in different directories whose classes share a name |
| When | one of them is checked |
| Then | a warning at its first line names the other file |

## `RK-006` A file is checked as it is typed

| Step | Statement |
| --- | --- |
| Given | a file saved as a library file and typed as a node script whose class shares another node script's name |
| When | the typed source is checked |
| Then | a warning names the other file |

## `RK-007` A syntax error's column counts characters

| Step | Statement |
| --- | --- |
| Given | source whose line does not parse after characters wider than a byte |
| When | the source is checked |
| Then | the error's column counts each of those characters once |
