# Reporting

How what mruby has to say about a Ruby file reaches Godot: through Godot's own log, at the file and line it names, rather than on the process's standard error.

## Includes

- `tasks/support/godot.rb`

## `RR-001` A compiler warning appears as a warning at its Ruby line

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file the compiler warns about |
| When | the scene runs |
| Then | Godot's output carries the warning as a warning at the file's `res://` path and the line the compiler names |
