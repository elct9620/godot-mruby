# Glossary

The words this project keeps, and the ones it turns down in their place.

## Godot mruby

### Includes

- `rust/src/**/*.rs`
- `rust/src/**/*.rb`
- `tasks/**/*.rb`
- `tasks/*.rake`
- `Rakefile`
- `build_config/*.rb`
- `CLAUDE.md`
- `.spec/**/*.md`

### Interpreter

One `mrb_state` together with the bookkeeping the extension keeps beside it, such as which files have run. The game's files share one interpreter, and nothing is shared between interpreters.

#### Rejected

- `VM` - mruby's virtual machine is one part of an interpreter; the interpreter is what the extension opens, enters and closes.

### Entry

A thread running Ruby in an interpreter, from the outermost call in until that call returns. A file runs at the first entry that needs it, never when it is loaded.

### Test directory

A directory the test runner runs tests from: one of a project's `mruby/test/directories`, searched recursively.

### Test file

A file under a test directory whose name matches the project's `mruby/test/pattern`. Only the test runner runs one.
