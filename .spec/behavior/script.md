# Ruby scripts

How a `.rb` file attached to a node comes to run, and where what it prints goes. A node's Ruby object is built the first time Godot calls a method its class defines, so a node is made without running Ruby.

## Includes

- `godot/test/e2e/**/*_test.rb`
- `godot/test/unit/script/**/*_test.rb`
- `tasks/support/script/announcement.rb`
- `tasks/support/script/callbacks.rb`
- `tasks/support/script/editor.rb`
- `tasks/support/script/header.rb`
- `tasks/support/script.rb`

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
| Given | a node script whose source Godot was given in place of what the file holds |
| When | a method of a node with that script is called |
| Then | the method the given source defines answers |

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
| Given | a node script whose class defines `_ready` adding children to its node |
| When | its node enters the scene |
| Then | the node has the children `_ready` added |

## `RS-012` `_process` is given the frame's delta as a Float

| Step | Statement |
| --- | --- |
| Given | a node in the tree whose script's class defines `_process` keeping its argument |
| When | a process frame passes |
| Then | the argument kept is a `Float` |

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
| Given | a node script whose class defines `_notification`, emitting a signal when it is given `NOTIFICATION_CHILD_ORDER_CHANGED`'s number with none of its node's children of one class left |
| When | the scene runs until the last of those children is freed |
| Then | the signal is emitted |

## `RS-018` A node script reports that its class is a tool before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class body calls `tool` |
| When | Godot asks the script whether it is a tool |
| Then | the script answers that it is |

## `RS-019` A node script reports that its class is abstract before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class body calls `abstract` |
| When | Godot asks the script whether it is abstract |
| Then | the script answers that it is |

## `RS-020` An abstract node script cannot be a node's script

| Step | Statement |
| --- | --- |
| Given | a node script whose class body calls `abstract` |
| Given | a node of the engine class it extends, with that script |
| When | the scene runs |
| Then | the log carries an error that the script is abstract |

## `RS-021` A node script reports the engine class it extends through other files before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class, which extends an engine node class |
| When | Godot asks the script for its instance base type |
| Then | the script answers that engine class |

## `RS-022` A node script reports the script of the class it extends

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class |
| When | Godot asks the script for its base script |
| Then | the script answers the script of that other file |

## `RS-023` A node's object is given a callback its class inherits from another file

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines no method and extends another file's class defining `_ready` |
| When | its node enters the scene |
| Then | the inherited `_ready` runs on the node's object |

## `RS-024` A file whose superclass no file names cannot be a node's script

| Step | Statement |
| --- | --- |
| Given | a file whose class extends a name no file spells |
| Given | a node with that script |
| When | the scene runs |
| Then | the log carries an error naming the superclass no file names |

## `RS-025` The editor lists a node script by its announcement

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another node script's class and names an icon relative to its file |
| When | the editor scans the project |
| Then | the project's class list carries the class with the other class as base and the icon's path from the project root |

## `RS-026` The editor warns once of node scripts sharing a name

| Step | Statement |
| --- | --- |
| Given | two node scripts in different directories whose classes share a name |
| When | the editor scans the project, as often as it does |
| Then | the log carries one warning naming both files |

## `RS-027` `p` answers what it is given

| Step | Statement |
| --- | --- |
| Given | no argument, one argument, or several |
| When | `p` is called with them |
| Then | it answers `nil`, the argument itself, or an array of the arguments |

## `RS-028` A method's answer reaches Godot as the value Ruby gave

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines one method answering a Float and another answering an Integer |
| When | Godot calls each method on the node |
| Then | the caller gets the first answer as a `float` and the second as an `int`, each with its value |

## `RS-029` The editor passes over a node script it cannot read

| Step | Statement |
| --- | --- |
| Given | a project holding a node script nobody may read |
| When | the editor scans the project |
| Then | the log says nothing about that file |

## `RS-030` A node script's Ruby object is its node

| Step | Statement |
| --- | --- |
| Given | a node whose script's `_physics_process` moves it by setting its own engine property |
| When | physics frames pass |
| Then | the node's position has changed |

## `RS-031` A node script's class makes a node carrying its script

| Step | Statement |
| --- | --- |
| Given | the class a node script defines, whose `initialize` takes arguments |
| When | Ruby calls `new` on the class with arguments |
| Then | Ruby gets an object of the class, initialized with those arguments, that answers the engine methods of the node class it extends |
| Then | Godot's calls on that node reach the same object |

## `RS-032` A node that fails to initialize is freed

| Step | Statement |
| --- | --- |
| Given | the class a node script defines, whose `initialize` raises |
| When | Ruby calls `new` on the class |
| Then | the exception reaches the caller |
| Then | the node it made has been freed |

## `RS-033` A class no node script defines makes no node

| Step | Statement |
| --- | --- |
| Given | a class extending an engine node class that no file of the game defines |
| When | Ruby calls `new` on the class |
| Then | it raises `NotImplementedError` |

## `RS-034` A call back into the same node reaches its Ruby object

| Step | Statement |
| --- | --- |
| Given | a node script's method that calls one of its node's engine methods, which calls back into the node's script |
| When | Godot calls the method |
| Then | the call back reaches the same Ruby object |

## `RS-035` A call arriving while a node initializes reaches the object initializing

| Step | Statement |
| --- | --- |
| Given | a node script's class whose `initialize` calls one of its node's engine methods, which calls back into the node's script |
| When | Ruby makes a node of the class |
| Then | the call back reaches the object `initialize` runs on |

## `RS-036` A call arriving before a file defines its class does nothing

| Step | Statement |
| --- | --- |
| Given | a file whose statements, before its class statement, make a node carrying the file's own script and call back into it |
| When | the file runs, and Godot later calls the node's script again |
| Then | the first call back does nothing and reports nothing, and the later call reaches the node's Ruby object |

## `RS-037` A node's script taken away while its Ruby runs lets the call finish

| Step | Statement |
| --- | --- |
| Given | a node script's method that takes its own node's script away and then answers |
| When | Godot calls the method |
| Then | the method runs to its end, Godot gets its answer, and the node has no script, as with GDScript |

## `RS-038` A node script reports the signals its class declares before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares a signal |
| When | Godot asks a node with that script whether it has that signal |
| Then | the node answers that it has |

## `RS-039` A scene's connection to a declared signal is made before its file runs

| Step | Statement |
| --- | --- |
| Given | a scene connecting a signal a node script's class declares |
| When | the scene runs |
| Then | the node answers that the signal is connected |

## `RS-040` A node script answers an exported property's value before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a property, on a node a scene wrote nothing to |
| When | Godot reads that property |
| Then | the node answers the value the export was declared with |

## `RS-041` A node script lists an exported property before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a property |
| When | Godot asks a node with that script for its properties |
| Then | the list names that property |

## `RS-042` A node script answers nothing for an export whose value is not written out

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a property with a value a call answers |
| When | Godot reads that property before the file runs |
| Then | the node answers nothing |

## `RS-043` A node lists the methods its class defines

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines a method |
| When | Godot asks a node with that script for its methods |
| Then | the list names that method |

## `RS-044` A node lists the methods its class inherits from another file

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines no method and inherits one from another file |
| When | Godot asks a node with that script for its methods |
| Then | the list names the inherited method |

## `RS-045` A node in the editor is given a placeholder for its script

| Step | Statement |
| --- | --- |
| Given | a scene whose node's script is a node script declaring a signal |
| When | the editor opens that scene |
| Then | the node's connection to that signal is made |

## `RS-046` `_property_can_revert` tells Godot a property can be reverted

| Step | Statement |
| --- | --- |
| Given | a node whose class defines `_property_can_revert`, answering true for a property's name |
| When | Godot asks the node whether that property can be reverted |
| Then | the node answers true |

## `RS-047` `_property_get_revert` gives Godot the value a property reverts to

| Step | Statement |
| --- | --- |
| Given | a node whose class defines `_property_get_revert`, answering a value for a property's name |
| When | Godot asks the node for the value that property reverts to |
| Then | the node answers that value |

## `RS-048` A node in the editor lists a property its script's changed source exports

| Step | Statement |
| --- | --- |
| Given | a scene open in the editor whose node's script is a node script |
| Given | the script's source Godot holds, changed to export another property |
| When | the script is reloaded |
| Then | the node lists that property |

## `RS-049` Godot saving a Ruby script writes its source to the file

| Step | Statement |
| --- | --- |
| Given | a Ruby script whose source Godot holds, changed from its file's |
| When | Godot saves the script to a `.rb` file |
| Then | the file holds the source Godot holds |
