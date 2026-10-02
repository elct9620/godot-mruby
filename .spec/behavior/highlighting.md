# Syntax highlighting

How the script editor colours a Ruby file: the language names Ruby's keywords and string delimiters for the editor's standard highlighter.

## Includes

- `rust/src/language.rs`

## `RU-001` A control-flow keyword is a reserved word

| Step | Statement |
| --- | --- |
| Given | a word the language calls a control-flow keyword |
| When | the reserved words are read |
| Then | the word is among them |
