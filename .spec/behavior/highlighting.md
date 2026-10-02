# Syntax highlighting

How the script editor colours a Ruby file: a highlighter of the extension's own colours each line from the tokens Prism reads in the whole file, in the colours the editor's theme gives each kind of word, and the language names Ruby's keywords and string delimiters for the editor's standard highlighter.

### What it does not see

- A line above an edit keeps its colours until it is edited or the file is opened again, as in every Godot highlighter: the editor asks again only from the edited line down.

## Includes

- `rust/src/language.rs`
- `rust/src/highlighter.rs`

## `RU-001` A control-flow keyword is a reserved word

| Step | Statement |
| --- | --- |
| Given | a word the language calls a control-flow keyword |
| When | the reserved words are read |
| Then | the word is among them |

## `RU-002` A reserved word is coloured as a keyword

| Step | Statement |
| --- | --- |
| Given | a line holding one of the language's reserved words |
| When | the line is coloured |
| Then | the word takes a keyword's colour |

## `RU-003` A control-flow keyword is coloured apart from the other keywords

| Step | Statement |
| --- | --- |
| Given | a method whose body returns early with a modifier `if` |
| When | the lines are coloured |
| Then | `return` and `if` take the control-flow colour, and `def` the keyword colour |

## `RU-004` A heredoc's body is coloured as a string

| Step | Statement |
| --- | --- |
| Given | a heredoc whose body spans two lines and is followed by code |
| When | the lines are coloured |
| Then | both lines of the body are strings, and the code after its terminator is not |

## `RU-005` An embedded document is coloured as a comment

| Step | Statement |
| --- | --- |
| Given | source with a `=begin` and `=end` block between two lines of code |
| When | the lines are coloured |
| Then | every line of the block is a comment, and the code around it is not |

## `RU-006` Code interpolated into a string is coloured as code

| Step | Statement |
| --- | --- |
| Given | a double-quoted string interpolating a call to a method that takes a number |
| When | the line is coloured |
| Then | the text outside the interpolation is a string, and the number inside it is a number |

## `RU-007` A word list is coloured as a string

| Step | Statement |
| --- | --- |
| Given | a `%w[]` list of words |
| When | the line is coloured |
| Then | the words take a string's colour |

## `RU-008` A symbol is coloured apart from a string

| Step | Statement |
| --- | --- |
| Given | a hash written with a `name:` label and a `:name` symbol value |
| When | the line is coloured |
| Then | both take a symbol's colour |

## `RU-009` A method's name is coloured where it is defined

| Step | Statement |
| --- | --- |
| Given | a method defined on `self` with a name ending in `?` |
| When | the line is coloured |
| Then | the whole name, its `?` included, takes a function's colour |

## `RU-010` An engine class is coloured apart from the game's constants

| Step | Statement |
| --- | --- |
| Given | a class that extends `Godot::Node2D` |
| When | the line is coloured |
| Then | `Node2D` takes an engine class's colour, and the class's own name and `Godot` a constant's |

## `RU-011` A column counts characters

| Step | Statement |
| --- | --- |
| Given | a line whose string of multibyte characters is followed by a comment |
| When | the line is coloured |
| Then | the comment starts at the column counted in characters |

## `RU-012` Source that does not parse is still coloured

| Step | Statement |
| --- | --- |
| Given | a method whose body has a syntax error before a keyword |
| When | the lines are coloured |
| Then | the keyword after the error still takes a keyword's colour |
