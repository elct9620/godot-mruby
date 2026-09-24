# Reloading

How a changed source takes effect while the game runs: what Godot reads of a script follows the source it now holds, and a reloaded file runs again in the same realm, so the objects already made keep their state and take the new methods.

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

## `RF-004` A method the changed source no longer defines stays until the game restarts

| Step | Statement |
| --- | --- |
| Given | a node whose script's file has run |
| Given | the script's source Godot holds, changed to leave out one of its methods |
| When | the script is reloaded and a frame passes |
| Then | the node's object still answers that method |

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
