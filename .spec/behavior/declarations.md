# Declarations

What a node script's class says about itself as its body runs, and how Godot is answered from it. A Ruby class has no shape Godot can read from the source, so the class declares what it makes public, and the realm publishes it once the file has run.

## Includes

- `godot/test/declarations/**/*.rb`

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
