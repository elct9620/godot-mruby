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
