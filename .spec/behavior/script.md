# Ruby scripts

How a `.rb` file attached to a node comes to run, and where what it prints goes. A node's Ruby object is built the first time Godot calls a method its class defines, so a node is made without running Ruby.

## Includes

- `tasks/support/godot.rb`
- `tasks/support/node_scripts.rb`

## `RS-001` A script attached to a node runs

| Step | Statement |
| --- | --- |
| Given | a node whose script is a node script calling `puts` and defining a callback |
| When | the scene runs |
| Then | the text given to `puts` appears in the log |

## `RS-002` A file runs once however many nodes share it

| Step | Statement |
| --- | --- |
| Given | two nodes whose script is the same node script calling `puts` and defining a callback |
| When | the scene runs |
| Then | the text given to `puts` appears in the log once |

## `RS-003` `print` writes to the log

| Step | Statement |
| --- | --- |
| Given | a node whose script is a node script calling `print` and defining a callback |
| When | the scene runs |
| Then | the text given to `print` appears in the log |

## `RS-004` `p` writes what it is given, inspected, to the log

| Step | Statement |
| --- | --- |
| Given | a node whose script is a node script calling `p` with a symbol and defining a callback |
| When | the scene runs |
| Then | the symbol's `inspect` form appears in the log |

## `RS-005` A script runs the source Godot holds for it

| Step | Statement |
| --- | --- |
| Given | a node script defining a callback, whose source Godot was given in place of what the file holds |
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

## `RS-011` `_ready` is called when a node is ready

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines `_ready` |
| When | its node enters the scene |
| Then | what `_ready` prints appears in the log |

## `RS-012` `_process` is given the frame's delta as a Float

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines `_process` printing its argument's class |
| When | the scene runs a frame |
| Then | the log carries `Float` as that class |

## `RS-013` A node's object is built once for its node

| Step | Statement |
| --- | --- |
| Given | two nodes with the same node script, whose `initialize` prints and whose class defines callbacks |
| When | the scene runs for several frames |
| Then | what `initialize` prints appears in the log twice |

## `RS-014` An object that cannot be built is reported once

| Step | Statement |
| --- | --- |
| Given | a node script whose `initialize` raises and whose class defines `_process` |
| When | the scene runs for several frames |
| Then | the log carries the exception once |

## `RS-015` A node whose object cannot be built calls no callback

| Step | Statement |
| --- | --- |
| Given | a node script whose `initialize` raises and whose `_process` prints |
| When | the scene runs for several frames |
| Then | what `_process` prints never appears in the log |

## `RS-016` A node script whose class defines no callback never runs its file

| Step | Statement |
| --- | --- |
| Given | a node script whose file prints and whose class defines no method |
| Given | its node, given a property value by the scene |
| When | the scene runs for several frames |
| Then | what the file prints never appears in the log |

## `RS-017` `_notification` is given each notification's number

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines `_notification`, printing when it is given `NOTIFICATION_READY`'s number |
| When | its node enters the scene |
| Then | what `_notification` prints appears in the log |
