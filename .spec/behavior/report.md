# Reporting

How what mruby has to say about a Ruby file reaches Godot: through the log, at the file and line it names, rather than on the process's standard error.

### What stays outside the log

- A codegen error reaches Godot only as `codegen error`; mruby 4.0.0 writes its line to the process's standard error.
- An mruby bug or an exception with nowhere to go ends the process with a message on standard error.
- A report carries the line, not the column: the log has no place for one.
- A file's compiler warnings are reported after the file has run, so they follow what it printed.

## Includes

- `rust/src/bridge/member_writes.rs`
- `rust/src/realm.rs`
- `tasks/support/report.rb`

## `RR-001` A compiler warning appears as a warning at its Ruby line

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file the compiler warns about |
| When | the scene runs |
| Then | the log carries the warning at the file's `res://` path and the line the compiler names |

## `RR-002` A file that does not parse appears as a script error at its Ruby line

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file that does not parse |
| When | the scene runs |
| Then | the log carries the compiler's error as a script error at the file's `res://` path and the line the compiler names |

## `RR-003` An exception appears as a script error at the Ruby line that raised it

| Step | Statement |
| --- | --- |
| Given | a node script whose callback calls a method that raises |
| When | the scene runs |
| Then | the log carries the exception's message as a script error at the method, file and line that raised it |

## `RR-004` An exception carries the backtrace Ruby called through

| Step | Statement |
| --- | --- |
| Given | a node script whose callback calls a method that raises |
| When | the scene runs |
| Then | the log follows the exception with its Ruby backtrace, most recent call first, each frame at its method, file and line |

## `RR-005` An exception a file's top level raises carries that top level

| Step | Statement |
| --- | --- |
| Given | a Ruby file whose top level raises |
| When | the file runs |
| Then | the exception's backtrace holds the file's top level at the line that raised |

## `RR-006` An exception's backtrace ends with the callback Godot called

| Step | Statement |
| --- | --- |
| Given | a node script whose callback calls a method that raises |
| When | the scene runs |
| Then | the backtrace the log carries ends with the callback, with no frame of what called into Ruby |

## `RR-007` An error the extension's Ruby raises is placed where the project called it

| Step | Statement |
| --- | --- |
| Given | a Ruby file calling a method of the extension's own Ruby that raises, such as a refused declaration |
| When | the file runs |
| Then | the exception is placed at the file's line, and its backtrace holds no frame of the extension's own Ruby |

## `RR-008` A line after a member write keeps its number

| Attribute | Value |
| --- | --- |
| relates | `RV-043` |

| Step | Statement |
| --- | --- |
| Given | a file assigning members of members, written across lines |
| When | the file is compiled, its member writes rewritten |
| Then | every line keeps its number, so a report names the line the file writes |

## `RR-009` An error Ruby pushes carries the backtrace Ruby called through

| Step | Statement |
| --- | --- |
| Given | a node script whose callback calls a method that calls `Godot.push_error` |
| When | the scene runs |
| Then | the log follows the error with its Ruby backtrace, most recent call first, each frame at its method, file and line |

## `RR-010` A warning Ruby pushes carries the backtrace Ruby called through

| Step | Statement |
| --- | --- |
| Given | a node script whose callback calls a method that calls `Godot.push_warning` |
| When | the scene runs |
| Then | the log follows the warning with its Ruby backtrace, most recent call first, each frame at its method, file and line |

## `RR-011` An engine error inside a Ruby call carries the backtrace Ruby called through

| Attribute | Value |
| --- | --- |
| relates | `RR-009` |

| Step | Statement |
| --- | --- |
| Given | a node script whose callback calls a method that asks the engine for a node it does not have |
| When | the scene runs in the editor |
| Then | the log follows the engine's error with its Ruby backtrace, most recent call first, each frame at its method, file and line |
