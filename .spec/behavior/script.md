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
