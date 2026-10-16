# Reloading

How a changed source takes effect while the game runs: what Godot reads of a script follows the source it now holds, and a reloaded file runs again in the same realm, so the objects already made keep their state and take the new methods.

A changed source naming another superclass makes its class afresh, as Zeitwerk loads a class it removed: the classes inheriting it are made afresh too, and each node of their scripts gets a new object holding the values its exports had.

### What it does not see

- An object another object held from before its class was made afresh stays an object of the old class, as a stale reference does after Zeitwerk reloads.

## Includes

- `godot/test/unit/reload/**/*_test.rb`

## `RF-001` A node script reports the engine class the file it extends now reaches

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class, asked for its instance base type once |
| Given | the other file's source Godot holds, changed to extend another engine node class |
| When | Godot asks the script for its instance base type |
| Then | the script answers the engine class the changed source extends |

## `RF-002` A reloaded node script's object answers a method only its changed source defines

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to define another method |
| When | the script is reloaded and a frame passes |
| Then | the node's object answers that method |

## `RF-003` A reloaded node script's object keeps its state

| Step | Statement |
| --- | --- |
| Given | a node whose object has set an instance variable |
| Given | the script's source Godot holds, changed |
| When | the script is reloaded and a frame passes |
| Then | the node's object still has the value it set |

## `RF-004` A method the changed source no longer defines is gone once the script reloads

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to leave out one of its methods |
| When | the script is reloaded and a frame passes |
| Then | the class no longer defines that method, as a class loaded afresh would not |

## `RF-005` A changed source that raises leaves the class as it was

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to raise before it defines anything |
| When | the script is reloaded and a frame passes |
| Then | the node's object answers as it did before |

## `RF-006` A node script whose file raised makes objects again once its changed source runs

| Step | Statement |
| --- | --- |
| Given | a node script whose file raised as a node's object was built |
| Given | the script's source Godot holds, changed so that it runs |
| When | the script is reloaded, a frame passes, and a new node with the script is called |
| Then | the new node's object answers |

## `RF-007` A node whose object failed stays without one after its file reloads

| Step | Statement |
| --- | --- |
| Given | a node whose object could not be built because its file raised |
| Given | the script's source Godot holds, changed so that it runs |
| When | the script is reloaded, a frame passes, and the node is called |
| Then | the call answers null |

## `RF-008` A property the changed source no longer exports is not given to a new object

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, exporting a property |
| Given | the script's source Godot holds, changed to leave out that export |
| When | the script is reloaded, a frame passes, and a new node with the script is called |
| Then | the new node's object has no instance variable of that name |

## `RF-009` A changed source that raises keeps the constants it assigned again

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, its class holding a constant |
| Given | the script's source Godot holds, changed to assign that constant again and then raise |
| When | the script is reloaded and a frame passes |
| Then | the class still holds the constant |

## `RF-010` A property the changed source exports anew is given to an object made before

| Step | Statement |
| --- | --- |
| Given | a node whose object was built |
| Given | the script's source Godot holds, changed to export another property |
| When | the script is reloaded and a frame passes |
| Then | the node answers that property with the value it is exported with |

## `RF-011` An object keeps the value it holds for a property the changed source exports anew

| Step | Statement |
| --- | --- |
| Given | a node whose object has set an instance variable |
| Given | the script's source Godot holds, changed to export a property of that name |
| When | the script is reloaded and a frame passes |
| Then | the node answers that property with the value its object set |

## `RF-012` A reloaded node script answers the default its changed source exports

| Step | Statement |
| --- | --- |
| Given | a node script whose file has run, exporting a property |
| Given | the script's source Godot holds, changed to export that property with another value |
| When | the script is reloaded, a frame passes, and Godot asks the script for the property's default value |
| Then | the script answers the value the changed source exports |

## `RF-013` A method another file defined in the class stays when the script reloads

| Attribute | Value |
| --- | --- |
| relates | `RF-004` |

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run, and a method another file defined in its class |
| When | the script is reloaded with its source changed and a frame passes |
| Then | the class still defines that method |

## `RF-014` A node script whose changed source names another superclass takes it

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to extend another engine class the node's own class reaches |
| When | the script is reloaded and a frame passes |
| Then | the node's object is of a class extending the new superclass |

## `RF-015` A node whose class took another superclass keeps its exported values

| Step | Statement |
| --- | --- |
| Given | a node whose script's object holds a value for a property it exports |
| Given | the script's source Godot holds, changed to extend another engine class and export the property still |
| When | the script is reloaded and a frame passes |
| Then | the node answers the value it held |

## `RF-016` A class inheriting a class that took another superclass is made afresh on it

| Step | Statement |
| --- | --- |
| Given | a node whose script's class inherits another file's class, both files run |
| Given | the other file's source Godot holds, changed to extend another engine class |
| When | that script is reloaded and a frame passes |
| Then | the node's object is of a class whose superclass is the other file's new class |

## `RF-017` A changed source naming another superclass that raises leaves the class as it was

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to extend another engine class and raise inside it |
| When | the script is reloaded and a frame passes |
| Then | the class extends the superclass it had, and the node's object answers as it did before |

## `RF-018` A node whose class took a superclass its own class does not reach loses its script

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to extend an engine class the node's own class does not reach |
| When | the script is reloaded and a frame passes |
| Then | the node has no script, as a GDScript extending another native type is refused |

## `RF-019` A node that lost its script to a superclass gets it back with its values

| Step | Statement |
| --- | --- |
| Given | a node left without its script by a superclass its own class does not reach, its object having held a value for an exported property |
| When | the script's source extends a class the node's own class reaches again, the script is reloaded and a frame passes |
| Then | the node has the script and answers the value it held, as GDScript keeps a refused node's reload state |
