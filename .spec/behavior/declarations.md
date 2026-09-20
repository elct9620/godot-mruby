# Declarations

What a node script's class says about itself as its body runs, and how Godot is answered from it. A Ruby class has no shape Godot can read from the source, so the class declares what it makes public, and the realm publishes it once the file has run.

## Includes

- `godot/test/declarations/**/*.rb`
- `tasks/support/exports.rb`

## `RD-001` A node has the signal its class declared

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares a signal |
| When | a node of that class is asked whether it has the signal |
| Then | it answers that it has it |

## `RD-002` A declared signal carries the parameters it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares a signal with one parameter |
| When | a node of that class is asked for its signals |
| Then | the declared signal's arguments are named as the declaration named them |

## `RD-003` A declared signal reaches what connected to it

| Step | Statement |
| --- | --- |
| Given | a node of a class declaring a signal, connected to a Ruby method |
| When | the node emits the signal with a value |
| Then | the connected method is called with that value |

## `RD-004` A node has a method its class defined as it ran

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines a method as its body runs, rather than with `def` |
| When | a node of that class is asked whether it has the method |
| Then | it answers that it has it |

## `RD-005` A method a class defined as it ran is among its script's methods

| Step | Statement |
| --- | --- |
| Given | a node script whose class defined a method as its body ran, rather than with `def` |
| When | the script is asked for the methods its class has |
| Then | the method is among them |

## `RD-006` A callback a class defined as it ran is called on the node

| Step | Statement |
| --- | --- |
| Given | a node of a class that defined an engine callback as its body ran |
| When | the engine notifies the node |
| Then | the callback is called with the notification |

## `RD-007` A signal declared again with other parameters is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares one signal with two sets of parameters |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-008` A signal declared again as it stands is no new signal

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares one signal twice with the same parameters |
| When | a node of that class is asked for its signals |
| Then | the signal is among them once |

## `RD-009` A node has the signal the class its class extends declared

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class, which declares a signal |
| When | a node of that class is asked whether it has the signal |
| Then | it answers that it has it |

## `RD-010` A node has a method the class its class extends defined as it ran

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class, which defined a method as its body ran |
| When | a node of that class is asked whether it has the method |
| Then | it answers that it has it |

## `RD-011` A class's export is among its script's properties

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value |
| When | the script is asked for the properties its class has |
| Then | the export is among them |

## `RD-012` An export takes its type from the value it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a float value |
| When | the script is asked for that property's default value |
| Then | it answers that float |

## `RD-013` A script's properties include the ones the class it extends exported

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class, which exports a value |
| When | the script is asked for the properties its class has |
| Then | the class it extends exported is among them |

## `RD-014` An export declared with no value is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value of `nil`, which names no type |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-015` An export a Ruby superclass declared is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a name the class it extends exported |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-016` An export the engine class already has is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a name its engine class has a property of |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-017` An export Ruby's `Object` already answers is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a name every Ruby object answers, such as `hash` |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-018` An export declared again as it stands is no new property

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports one name twice with the same value |
| When | the script is asked for the properties its class has |
| Then | the property is among them once |

## `RD-019` An export declared again with another value is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports one name with two values |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-020` A signal a Ruby superclass declared is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class declares a signal the class it extends declared |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-021` A node answers an exported property by the accessors its class gets

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value |
| When | a node of that class is asked for that property |
| Then | it answers the value it was exported with |

## `RD-022` A class's own accessor stands in place of the one an export would define

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value and defines a reader of that name |
| When | a node of that class is asked for that property |
| Then | the class's own reader answers |

## `RD-023` An exported value is written before the object is initialized

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value and reads it in `initialize` |
| When | a node of that class is made |
| Then | `initialize` read the value it was exported with |

## `RD-024` Each node has its own copy of a container it was exported

| Step | Statement |
| --- | --- |
| Given | two nodes of a class exporting an array |
| When | one of them changes the array it was given |
| Then | the other's array is as it was exported |

## `RD-025` A node lists the properties its class exported

| Step | Statement |
| --- | --- |
| Given | a node of a class exporting a value |
| When | the node is asked for its properties |
| Then | the export is among them |

## `RD-026` Godot writes an exported property through the accessor

| Step | Statement |
| --- | --- |
| Given | a node of a class exporting a value |
| When | Godot sets that property |
| Then | the node's class reads the value it was set to |

## `RD-027` A property the engine class has stays the engine's

| Step | Statement |
| --- | --- |
| Given | a node whose object wrote an instance variable named as an engine property |
| When | Godot sets that property |
| Then | the engine's property is what answers the value |

## `RD-028` Godot reads an instance variable the class did not export

| Step | Statement |
| --- | --- |
| Given | a node whose object wrote an instance variable its class did not export |
| When | Godot reads a property of that name |
| Then | it answers what the variable holds |

## `RD-029` Godot writes an instance variable the class did not export

| Step | Statement |
| --- | --- |
| Given | a node whose object wrote an instance variable its class did not export |
| When | Godot sets a property of that name |
| Then | the class reads the value it was set to |

## `RD-030` A property answers what a scene wrote before the node has an object

| Step | Statement |
| --- | --- |
| Given | a scene setting an exported property of a node whose Ruby object nothing has built |
| When | Godot reads that property |
| Then | it answers what the scene wrote |

## `RD-031` A scene's value reaches the node's Ruby object once it is built

| Step | Statement |
| --- | --- |
| Given | a scene setting an exported property of a node whose Ruby object nothing has built |
| When | something builds the node's Ruby object |
| Then | the class reads what the scene wrote |

## `RD-032` A property answers the class's default while the node has no object

| Step | Statement |
| --- | --- |
| Given | a node whose class has run and whose Ruby object nothing has built |
| When | Godot reads an exported property the scene set nothing on |
| Then | it answers the value the class exported |

## `RD-033` A copy of a node carries what its properties hold

| Step | Statement |
| --- | --- |
| Given | a node of a class exporting a value, set to another value |
| When | the node is duplicated |
| Then | the copy answers the value the node was set to |

## `RD-034` A property only the node's own class has stays the engine's

| Step | Statement |
| --- | --- |
| Given | a node whose object wrote an instance variable named as a property only the node's class has, not the class its script extends |
| When | Godot sets that property |
| Then | the engine's property is what answers the value |

## `RD-035` A variable named as a property of the node's own class is untouched

| Step | Statement |
| --- | --- |
| Given | a node whose object wrote an instance variable named as a property only the node's class has |
| When | Godot sets that property |
| Then | the variable the object wrote is unchanged |

## `RD-036` A name a Ruby superclass declared is refused in GDScript's words

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a name the class it extends exported |
| When | the file runs |
| Then | the refusal reads `The member "<name>" already exists in parent class <class>.` |

## `RD-037` A name the engine class has is refused in GDScript's words

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a name its engine class has a property of |
| When | the file runs |
| Then | the refusal reads `Member "<name>" redefined (original in native class '<class>')` |

## `RD-038` An export naming no type is refused in GDScript's words

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value of `nil`, which names no type |
| When | the file runs |
| Then | the refusal reads `Cannot use "export" because the type of the initialized value can't be inferred.` |

## `RD-039` A range hint carries the bounds it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a float with a range |
| When | the script is asked for the properties its class has |
| Then | that property carries the range hint, read with `0,360` |

## `RD-040` A range hint carries the step it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports an integer with a range and a step |
| When | the script is asked for the properties its class has |
| Then | that property's range is read with `1,10,2` |

## `RD-041` An enum hint names the values it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a name with the values it may take |
| When | the script is asked for the properties its class has |
| Then | that property carries the enum hint, read with `nearest,strongest` |

## `RD-042` A flags hint names the flags it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports an integer with the flags it holds |
| When | the script is asked for the properties its class has |
| Then | that property carries the flags hint, read with `water,fire` |

## `RD-043` A file hint carries the filter it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a path to a file of one kind |
| When | the script is asked for the properties its class has |
| Then | that property carries the file hint, read with `*.ogg` |

## `RD-044` A directory hint is declared without a filter

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a path to a directory |
| When | the script is asked for the properties its class has |
| Then | that property carries the directory hint, read with nothing |

## `RD-045` A multiline hint is declared without a filter

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a string written over several lines |
| When | the script is asked for the properties its class has |
| Then | that property carries the multiline hint, read with nothing |

## `RD-046` A placeholder hint carries the text it was declared with

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a string with the text an empty field shows |
| When | the script is asked for the properties its class has |
| Then | that property carries the placeholder hint, read with `Name` |

## `RD-047` A hint the exported type cannot take is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a string with a range |
| When | the file runs |
| Then | the refusal reads `"range:" requires a variable of type "int" or "float", but type "String" was given instead.` |

## `RD-048` A range leaving out its end is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value with a range written `1...9` |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-049` A step without a range is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value with a step and no range |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-050` A second hint on one property is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports one value with two hints |
| When | the file runs |
| Then | the declaration raises where it is written |

## `RD-051` A node's properties carry the hints its class declared

| Step | Statement |
| --- | --- |
| Given | a node of a class exporting a float with a range |
| When | the node is asked for its properties |
| Then | that property carries the range hint, read with `0,360` |

## `RD-052` An export naming a node class is read as a node of it

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value of `nil` naming an engine node class |
| When | the script is asked for the properties its class has |
| Then | that property is an object carrying the node hint, read with `Marker2D` |

## `RD-053` An export naming a resource class is read as a resource of it

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value of `nil` naming an engine resource class |
| When | the script is asked for the properties its class has |
| Then | that property carries the resource hint, read with `PackedScene` |

## `RD-054` An export naming a Ruby class is read by the name it was announced under

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value of `nil` naming another file's node class |
| When | the script is asked for the properties its class has |
| Then | that property carries the node hint, read with the name the editor lists that class under |

## `RD-055` An export naming a type takes no value of another type

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a number naming a node class |
| When | the file runs |
| Then | the refusal reads `Cannot assign a value of type int to variable "spot" with specified type Marker2D.` |

## `RD-056` An export naming neither a node nor a resource class is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value naming an engine class of neither kind |
| When | the file runs |
| Then | the refusal reads `Export type can only be built-in, a resource, a node, or an enum.` |

## `RD-057` An export naming a class no announcement lists is refused

| Step | Statement |
| --- | --- |
| Given | a node script whose class exports a value naming a library file's class |
| When | the file runs |
| Then | the refusal reads `The class "<class>" was not found in the global scope.` |
