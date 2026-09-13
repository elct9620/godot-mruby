# Glossary

The words this project keeps, and the ones it turns down in their place.

## Godot mruby

### Includes

- `rust/src/**/*.rs`
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
