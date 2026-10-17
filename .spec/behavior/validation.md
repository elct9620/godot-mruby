# Validation

How the editor learns what is wrong with a Ruby file as it is typed: the source is compiled in a state of its own and never run, and what the compiler says comes back as data rather than through the log, with what the project's other files make of the file's name.

### What it does not see

- A codegen error comes back only as `codegen error` at the file's first line; mruby 4.0.0 writes its line to the process's standard error.
- Only the first syntax error comes back.
- An unknown method, a mismatched type or a refused declaration shows only when the file runs.
- A file naming a constant the realm already has is warned of only as the realm opens: only the realm knows what it has.
- A member answering a StringName is known only when called on the object itself: nothing tells the class of another receiver, such as a signal's argument.

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

## `RK-008` A node script defining a method its engine class binds is warned of

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines a method of a name its engine class binds, such as `get_name` |
| When | the editor checks the file |
| Then | a warning comes back at the method's line that the engine will not call it, as GDScript's `NATIVE_METHOD_OVERRIDE` says |

## `RK-009` A StringName member compared with a String is warned of

| Step | Statement |
| --- | --- |
| Given | a node script whose method compares, with `==` or `!=`, a String with a member of its engine class answering a StringName, called on the object itself |
| When | the file is checked |
| Then | a warning at that line says the member reaches Ruby as a Symbol, so the comparison never holds |

## `RK-010` A class with abstract methods that is not abstract is refused

| Step | Statement |
| --- | --- |
| Given | a file whose class marks a method with `abstract def` but does not call `abstract` itself |
| When | the editor checks the file |
| Then | an error comes back at the class's line that it is not abstract but contains abstract methods, in GDScript's words |

## `RK-011` A class leaving an inherited abstract method unwritten is refused

| Step | Statement |
| --- | --- |
| Given | an abstract class marking two methods abstract, and a file whose class extends it writing only one |
| When | the editor checks the file |
| Then | an error comes back at the class's line that it must implement the other, in GDScript's words |

## `RK-012` A class writing every inherited abstract method is valid

| Step | Statement |
| --- | --- |
| Given | an abstract class marking two methods abstract, and a file whose class extends it writing both |
| When | the editor checks the file |
| Then | no error comes back |
