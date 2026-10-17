# Ruby scripts

How a `.rb` file attached to a node comes to run, and where what it prints goes. A node's Ruby object is built the first time Godot calls a method its class defines, so a node is made without running Ruby.

### What differs from GDScript

- A method a class defines on `self` takes and answers values of any type, and only one its source writes as `def self.name` in the class body is answered: one defined another way, or while the file runs, is neither called through the script nor known to GDScript's analyzer.

## Includes

- `godot/test/e2e/**/*_test.rb`
- `godot/test/unit/script/**/*_test.rb`
- `tasks/support/script/announcement.rb`
- `tasks/support/script/callbacks.rb`
- `tasks/support/script/editor.rb`
- `tasks/support/script/header.rb`
- `tasks/support/script/singleton_node.rb`
- `tasks/support/script.rb`
- `rust/src/snapshot.rs`

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

## `RS-032` A node that fails to initialize is freed

| Step | Statement |
| --- | --- |
| Given | the class a node script defines, whose `initialize` raises |
| When | Ruby calls `new` on the class |
| Then | the node it made has been freed |

## `RS-033` A class no node script defines makes no node

| Step | Statement |
| --- | --- |
| Given | a class extending an engine node class that no file of the game defines, as its class or inside it |
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
| Then | the first call back does nothing, and the later call reaches the node's Ruby object |

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

## `RS-050` A node script lists an exported property's hint before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a property with a hint its source writes out |
| When | Godot asks the script for its properties |
| Then | the property carries that hint |

## `RS-051` A node script lists the headings its class writes before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class body writes `export_group` with its name spelled out |
| When | Godot asks the script for its properties |
| Then | the list carries that group before the properties written after it |

## `RS-052` A node script lists what its changed source exports before the file runs again

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run |
| Given | the script's source Godot holds, changed to export another property with a hint it writes out |
| When | Godot asks the script for its properties before the file runs again |
| Then | the list carries that property with that hint |

## `RS-053` A node script keeps what its run declared for an export its changed source does not write out

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, exporting a property with a value its source does not write out |
| Given | the script's source Godot holds, changed elsewhere |
| When | Godot asks the script for its properties before the file runs again |
| Then | the list carries that property as the run declared it |

## `RS-054` A node script drops an export its changed source no longer writes

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, exporting a property |
| Given | the script's source Godot holds, changed to export it no more |
| When | Godot asks the script for its properties before the file runs again |
| Then | the list does not carry that property |

## `RS-055` A node script keeps an export no `export` call of its source names while its source changes

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, exporting a property through a name its source does not spell out |
| Given | the script's source Godot holds, changed elsewhere |
| When | Godot asks the script for its properties before the file runs again |
| Then | the list carries that property as the run declared it |

## `RS-056` A node in the editor lists the hints and headings its script declares as its file runs

| Step | Statement |
| --- | --- |
| Given | a scene whose node's script is a node script exporting a property with a hint its source does not write out |
| When | the editor opens that scene |
| Then | the node's placeholder lists that property with that hint once the file has run in the editor |

## `RS-057` The editor runs no file under a test directory

| Step | Statement |
| --- | --- |
| Given | a scene whose node's script is under a test directory, and whose other node's script uses the constant that file names |
| When | the editor opens that scene |
| Then | the file under the test directory does not run |

## `RS-058` A node in the editor lists what its script declares as its reloaded file runs again

| Step | Statement |
| --- | --- |
| Given | a scene open in the editor whose node's script is a node script whose file has run there |
| Given | the script's source Godot holds, changed to export a property with another hint its source does not write out |
| When | the script is reloaded |
| Then | the node's placeholder lists that property with the new hint once the file has run again |

## `RS-059` A tool script's node runs its callbacks in the editor

| Step | Statement |
| --- | --- |
| Given | a scene whose node's script is a node script whose class calls `tool` and defines `_ready` and `_process` |
| When | the editor opens that scene |
| Then | the node's Ruby object is given `_ready` and then `_process` |

## `RS-060` A tool script whose source does not parse keeps a scene's values in the editor

| Step | Statement |
| --- | --- |
| Given | a scene whose node's script is a tool script with a syntax error, and which writes the node a value its class declares nothing for |
| When | the editor opens that scene |
| Then | the node answers that value |

## `RS-061` A node script answers the name the editor lists it by as its global name

| Step | Statement |
| --- | --- |
| Given | a node script the editor lists by its announcement |
| When | Godot asks the script for its global name |
| Then | the script answers the name it is announced by |

## `RS-062` A node script lists a signal its changed source declares before the file runs again

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run |
| Given | the script's source Godot holds, changed to declare another signal it writes out |
| When | Godot asks the script for its signals before the file runs again |
| Then | the list carries that signal with its parameters |

## `RS-063` A node script drops a signal its changed source no longer declares

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, declaring a signal |
| Given | the script's source Godot holds, changed to declare it no more |
| When | Godot asks the script for its signals before the file runs again |
| Then | the list does not carry that signal |

## `RS-064` A node script keeps a signal no `signal` call of its source names while its source changes

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, declaring a signal through a name its source does not spell out |
| Given | the script's source Godot holds, changed elsewhere |
| When | Godot asks the script for its signals before the file runs again |
| Then | the list carries that signal as the run declared it |

## `RS-065` A node script keeps what its run declared for a signal whose parameters its changed source does not write out

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, declaring a signal with parameters its source does not write out |
| Given | the script's source Godot holds, changed elsewhere |
| When | Godot asks the script for its signals before the file runs again |
| Then | the list carries that signal as the run declared it |

## `RS-066` A node whose object failed to build keeps the values Godot writes to it

| Step | Statement |
| --- | --- |
| Given | a node the engine made, whose script's `initialize` raises |
| When | Godot writes a property its class exports and reads it back |
| Then | the node answers the value written |

## `RS-067` A Ruby tool script extending `EditorPlugin` runs as an editor plugin

| Step | Statement |
| --- | --- |
| Given | a project enabling a plugin whose `plugin.cfg` names a node script whose class calls `tool` and extends `Godot::EditorPlugin` |
| When | the editor opens the project |
| Then | the class's object is given `_enter_tree` as the editor adds the plugin |

## `RS-068` A call from another thread waits while a node initializes

| Attribute | Value |
| --- | --- |
| unverifiable | no test can make another thread arrive between the object being built and initialized |

| Step | Statement |
| --- | --- |
| Given | a node whose first call from Godot builds its Ruby object and runs `initialize` |
| Given | another thread calling a method of the node's script before `initialize` returns |
| When | `initialize` returns |
| Then | the other thread's call reaches the object only then |

## `RS-069` Godot's calls on a node a node script's class made reach its object

| Step | Statement |
| --- | --- |
| Given | the class a node script defines, whose `initialize` takes arguments |
| When | Godot calls the node that Ruby's `new` on the class made |
| Then | the call reaches the object `new` answered |

## `RS-070` What a node's `initialize` raises reaches the caller of `new`

| Step | Statement |
| --- | --- |
| Given | the class a node script defines, whose `initialize` raises |
| When | Ruby calls `new` on the class |
| Then | the exception reaches the caller |

## `RS-071` A failure inside the extension as Godot reads a node answers nothing

| Attribute | Value |
| --- | --- |
| refines | `RD-028` |

| Step | Statement |
| --- | --- |
| Given | a node whose object answers Godot's read of an instance variable with what the extension cannot read |
| When | Godot reads that property |
| Then | Godot gets nil |

## `RS-072` A node script that ran lists what its class declared, not what its unchanged source writes

| Attribute | Value |
| --- | --- |
| relates | `RS-062` |

| Step | Statement |
| --- | --- |
| Given | a node script whose source writes a signal its class did not declare as the file ran |
| Given | the source Godot holds, unchanged since the run |
| When | Godot asks the script for its signals |
| Then | the list leaves the signal out |

## `RS-073` A node built while the game's realm failed to open reports it once

| Attribute | Value |
| --- | --- |
| unverifiable | no run can make the game's realm fail to open |
| relates | `RE-015` |

| Step | Statement |
| --- | --- |
| Given | a node whose Ruby object is not built, and the game's realm, which failed to open |
| When | Godot calls the node again and again |
| Then | the log carries the failure once for the node |

## `RS-074` A node whose class inherits from a script's class is taken where that script is the type

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class |
| Given | an array typed by the other file's script |
| When | a node with the first script is put into the array |
| Then | the array holds the node |

## `RS-075` A node whose class does not inherit from a script's class is refused where that script is the type

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class |
| Given | an array typed by the first script |
| When | a node with the other file's script is put into the array |
| Then | the array stays empty |

## `RS-076` A node script the class list does not carry still shows its icon

| Attribute | Value |
| --- | --- |
| unverifiable | Godot reads a script's icon only inside the editor, through no call a headless run makes |

| Step | Statement |
| --- | --- |
| Given | a node script whose class calls `icon` and shares its name with another node script |
| When | the editor shows a node with that script |
| Then | the node shows the icon the class calls |

## `RS-077` A node script keeps the types its run gave a signal its changed source writes alike

| Attribute | Value |
| --- | --- |
| relates | `RS-065` |

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, declaring a signal with a typed parameter |
| Given | the script's source Godot holds, changed elsewhere, writing that signal's parameters by the same names |
| When | Godot asks the script for its signals before the file runs again |
| Then | the list carries that signal with the types its run gave it |

## `RS-078` The editor's Signals dock lists a node script's signal with its parameters' types

| Attribute | Value |
| --- | --- |
| relates | `RD-084` |

| Step | Statement |
| --- | --- |
| Given | a scene with a node whose script's class declares a signal with typed parameters, open in the editor |
| When | the node is selected and the Signals dock lists its signals |
| Then | the signal's row names each parameter's type as GDScript's typed signal's row does |

## `RS-079` A node script listed as a singleton node is ready at the root before the main scene

| Step | Statement |
| --- | --- |
| Given | a project listing a node script as a singleton node, and a main scene whose node reads it in `_ready` |
| When | the scene runs |
| Then | the singleton node's `_ready` runs at `/root/<name>` before the scene's node reads it |

## `RS-080` GDScript reaches a node script listed as a singleton node by its name

| Step | Statement |
| --- | --- |
| Given | a project listing a node script defining a method as a singleton node, and a main scene whose GDScript node calls that method through the singleton node's name |
| When | the scene runs |
| Then | the GDScript node gets the method's answer |

## `RS-081` A class inside a node script's class makes a node carrying a script of its own

| Step | Statement |
| --- | --- |
| Given | a class extending an engine node class, written inside the class a node script defines |
| When | Ruby calls `new` on the inner class |
| Then | Ruby gets an object of the inner class that answers the engine methods of the node class it extends |

## `RS-082` Godot calls an inner class's method on its node

| Step | Statement |
| --- | --- |
| Given | a node an inner class made, whose class defines a method |
| When | Godot calls that method on the node |
| Then | it gets what the inner class's method answers |

## `RS-083` An inner class's node has the properties the inner class exported

| Step | Statement |
| --- | --- |
| Given | a node script's class and the class inside it, each exporting a property |
| When | Godot lists the properties of a node the inner class made |
| Then | the list has the inner class's property and not the outer class's |

## `RS-084` A node script's node has none of its inner class's properties

| Step | Statement |
| --- | --- |
| Given | a node script's class and the class inside it, each exporting a property |
| When | Godot lists the properties of a node the outer class made |
| Then | the list has the outer class's property and not the inner class's |

## `RS-085` An inner class's node has the signals the inner class declared

| Step | Statement |
| --- | --- |
| Given | a class inside a node script's class, declaring a signal |
| When | Godot asks a node the inner class made for that signal |
| Then | the node has it |

## `RS-086` An object script's class makes an object carrying its script

| Step | Statement |
| --- | --- |
| Given | the class an object script defines, extending a reference-counted engine class and defining a method |
| When | Ruby calls `new` on the class, and Godot calls that method on the object |
| Then | Godot gets what the class's method answers |

## `RS-087` An object script's reference-counted object lives while Ruby holds it

| Step | Statement |
| --- | --- |
| Given | an object an object script's class made, held by Ruby alone |
| When | the collector runs and a frame passes |
| Then | the object still answers its class's methods with the state Ruby gave it |

## `RS-088` An object script's reference-counted object is freed once neither Ruby nor the engine holds it

| Step | Statement |
| --- | --- |
| Given | an object an object script's class made, which the engine held and let go of, and then Ruby lets go of |
| When | frames pass with the collector running |
| Then | the engine object has been freed |

## `RS-089` An object script's object the engine alone holds keeps its Ruby state

| Step | Statement |
| --- | --- |
| Given | an object an object script's class made, put in an engine container and let go of by Ruby |
| When | the collector runs, a frame passes, and Ruby takes it back from the container |
| Then | Ruby gets the same object, with the state it gave it |

## `RS-090` An object script's resource keeps its exports through a save and a load

| Step | Statement |
| --- | --- |
| Given | a resource an object script's class made, with an exported property written |
| When | Godot saves it to a file and loads it back from that file |
| Then | the loaded resource's property holds what was written |

## `RS-091` An object script extending Object makes an object Ruby frees

| Step | Statement |
| --- | --- |
| Given | the class an object script defines, extending `Object` and defining a method |
| When | Ruby calls `new` on the class and then the method |
| Then | Ruby gets what the method answers, from an object it frees with `free` |

## `RS-092` A node script lists an export's extra hints and a tool button before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a range with `extra_hints:` and a tool button, whose file has not run |
| When | Godot lists the script's properties |
| Then | the range is read with its extra hints and the tool button is a Callable with its hint |

## `RS-093` A node script lists its rpcs before its file runs

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares an rpc with literal words, whose file has not run |
| When | Godot reads the script's rpc config |
| Then | the config lists that rpc as the file would declare it |

## `RS-094` A method Godot calls answers a signal as it awaits

| Step | Statement |
| --- | --- |
| Given | a node whose class's method calls `await` on a signal |
| When | Godot calls that method |
| Then | Godot gets a Signal at once, which awaiting waits on as on a GDScript coroutine's state, and the method has run up to its `await` |

## `RS-095` An awaiting method resumes once its signal is emitted

| Step | Statement |
| --- | --- |
| Given | a node whose method Godot called is awaiting a signal |
| When | the signal is emitted |
| Then | the method runs on from its `await` |

## `RS-096` `await` answers what the signal was emitted with

| Step | Statement |
| --- | --- |
| Given | a node whose method Godot called is awaiting a signal |
| When | the signal is emitted with values |
| Then | `await` answers nil for none, the value for one, and the values in an array for more, as GDScript's does |

## `RS-097` A block Godot calls back may await

| Step | Statement |
| --- | --- |
| Given | a node whose class connected a block calling `await` to a signal |
| When | the signal reaches the block, and then the one it awaits is emitted |
| Then | the block runs on from its `await`, as a GDScript lambda may await |

## `RS-098` `await` outside what Godot calls raises

| Step | Statement |
| --- | --- |
| Given | a node whose class's method calls `await` |
| When | Ruby calls that method itself, outside anything Godot called |
| Then | `await` raises `FiberError` |

## `RS-099` A method Godot calls awaits through the methods it calls

| Step | Statement |
| --- | --- |
| Given | a node whose method Godot calls writes no `await` but calls a method of its class, and one of a module, that awaits a signal |
| When | Godot calls the method, and then the signal is emitted |
| Then | the method waits where the method it called awaits, and runs on once the signal is emitted, as a GDScript coroutine awaiting another does |

## `RS-100` GDScript awaiting a Ruby method waits until it ends

| Step | Statement |
| --- | --- |
| Given | a GDScript function awaiting a node's Ruby method that awaits a signal |
| When | the signal is emitted |
| Then | the GDScript function runs on with what the Ruby method returned, as it would with a GDScript coroutine |

## `RS-101` A method that could await but does not answers its value

| Step | Statement |
| --- | --- |
| Given | a node whose method Godot calls awaits only on some calls |
| When | Godot calls it on one where it returns without awaiting |
| Then | Godot gets what it returned, as a GDScript coroutine that never awaits returns its value |

## `RS-102` `await` answers a value that is no signal at once

| Step | Statement |
| --- | --- |
| Given | a value that is neither a Signal nor a coroutine's state, such as what a method that did not wait returned |
| When | Ruby awaits it, inside or outside what Godot calls |
| Then | `await` answers the value without waiting, as GDScript's does |

## `RS-103` `Godot::Coroutine.run` goes on without waiting

| Step | Statement |
| --- | --- |
| Given | a method Godot called, running a block that awaits a signal through `Godot::Coroutine.run` |
| When | the block reaches its `await`, and later the signal is emitted |
| Then | `run` answers a Signal at once and the method goes on, the block runs on once the signal is emitted, and awaiting the Signal answers what the block returned |

## `RS-104` `await` under a method mruby runs in C raises and leaves no connection

| Attribute | Value |
| --- | --- |
| relates | `RS-098` |

| Step | Statement |
| --- | --- |
| Given | a method Godot called, awaiting a signal inside a block a method of mruby's C calls, such as `sort`'s |
| When | the method reaches the `await`, and it then awaits the signal again outside the block |
| Then | the first `await` raises `FiberError`, and the second answers what the signal is emitted with, since the first left no connection |

## `RS-105` A coroutine whose signal goes with its object is let go of

| Step | Statement |
| --- | --- |
| Given | a coroutine waiting for a signal, and the Signal its end is answered through |
| When | the object emitting the signal it waits for is freed, and the collector runs |
| Then | the object the Signal names is freed too, as nothing is left to resume the coroutine |

## `RS-106` GDScript makes an object of a Ruby class with `new`

| Step | Statement |
| --- | --- |
| Given | the class a Ruby script defines, whose `initialize` takes arguments |
| When | GDScript calls `new` on the script with arguments, as on a GDScript's |
| Then | it gets an object of the class, carrying the script, made by `initialize` with those arguments |

## `RS-107` GDScript calls a Ruby class's method on `self`

| Step | Statement |
| --- | --- |
| Given | the class a Ruby script defines, with a method it defines on `self` |
| When | GDScript calls that method on the script, as a GDScript's static function |
| Then | it gets what the method answered |

## `RS-108` A script lists its class's methods on `self` as static

| Attribute | Value |
| --- | --- |
| relates | `RS-107` |

| Step | Statement |
| --- | --- |
| Given | the class a Ruby script defines, with a method it defines on `self` |
| When | Godot lists the script's methods, and GDScript compiles a call of that method on the script |
| Then | the method is listed as static, so GDScript's analyzer takes the call as it takes a static function's |

## `RS-109` An object answers its class's method on `self`

| Step | Statement |
| --- | --- |
| Given | an object of a Ruby class defining a method on `self` |
| When | GDScript calls that method on the object, as it may call a static function on an instance |
| Then | it gets what the class's method answered |
