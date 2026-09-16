# Ruby scripts

How a `.rb` file attached to a node comes to run, and where what it prints goes.

## Includes

- `tasks/support/godot.rb`

## `RS-001` A script attached to a node runs

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file calling `puts` |
| When | the scene runs |
| Then | the text given to `puts` appears in the log |

## `RS-002` A file runs once however many nodes share it

| Step | Statement |
| --- | --- |
| Given | two nodes whose script is the same `.rb` file calling `puts` |
| When | the scene runs |
| Then | the text given to `puts` appears in the log once |

## `RS-003` `print` writes to the log

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file calling `print` |
| When | the scene runs |
| Then | the text given to `print` appears in the log |

## `RS-004` `p` writes what it is given, inspected, to the log

| Step | Statement |
| --- | --- |
| Given | a node whose script is a `.rb` file calling `p` with a symbol |
| When | the scene runs |
| Then | the symbol's `inspect` form appears in the log |

## `RS-005` A script runs the source Godot holds for it

| Step | Statement |
| --- | --- |
| Given | a `.rb` file whose source Godot was given in place of what the file holds |
| When | a node with that script enters the scene |
| Then | the text the given source prints appears in the log |

## `RS-006` A library file cannot be a node's script

| Step | Statement |
| --- | --- |
| Given | a node whose script is a library file |
| When | the scene runs |
| Then | the log carries an error that the file cannot be a node's script |

## `RS-007` A node script cannot be the script of a node its class does not extend

| Step | Statement |
| --- | --- |
| Given | a node script extending an engine node class |
| Given | a node whose class does not inherit that engine class, with that script |
| When | the scene runs |
| Then | the log carries an error naming the engine class and the node's class |

## `RS-008` A node script reports the engine class it extends before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script extending an engine node class |
| When | Godot asks the script for its instance base type |
| Then | the script answers that engine class |

## `RS-009` A node script reports the methods its class defines before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines a method |
| When | Godot asks a node with that script whether it has that method |
| Then | the node answers that it has |
