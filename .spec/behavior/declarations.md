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
