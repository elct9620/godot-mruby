# Reporting

How what mruby has to say about a Ruby file reaches Godot: through Godot's own log, at the file and line it names, rather than on the process's standard error.

### What stays outside Godot's log

- A codegen error reaches Godot only as `codegen error`; mruby 4.0.0 writes its line to the process's standard error.
- An mruby bug or an exception with nowhere to go ends the process with a message on standard error.
- A report carries the line, not the column: Godot's log has no place for one.
- A file's compiler warnings are reported after the file has run, so they follow what it printed.

## Includes

- `tasks/support/godot.rb`

## `RR-001` A compiler warning appears as a warning at its Ruby line

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file the compiler warns about |
| When | the scene runs |
| Then | Godot's output carries the warning as a warning at the file's `res://` path and the line the compiler names |

## `RR-002` A file that does not parse appears as a script error at its Ruby line

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file that does not parse |
| When | the scene runs |
| Then | Godot's output carries the compiler's error as a script error at the file's `res://` path and the line the compiler names |
