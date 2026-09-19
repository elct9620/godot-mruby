# Reporting

How what mruby has to say about a Ruby file reaches Godot: through the log, at the file and line it names, rather than on the process's standard error.

### What stays outside the log

- A codegen error reaches Godot only as `codegen error`; mruby 4.0.0 writes its line to the process's standard error.
- An mruby bug or an exception with nowhere to go ends the process with a message on standard error.
- A report carries the line, not the column: the log has no place for one.
- A file's compiler warnings are reported after the file has run, so they follow what it printed.

## Includes

- `tasks/support/godot.rb`

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
