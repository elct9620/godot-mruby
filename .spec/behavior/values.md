# Values

How a value crosses between Ruby and the engine, either way: data is copied and objects are shared, so each side keeps what it holds and an object is the same object on both. A container either crosses whole or not at all, so neither side goes on with data the other never sent.

## Includes

- `godot/test/values/**/*.rb`
- `tasks/support/callbacks.rb`

## `RV-001` A scalar crosses as itself

| Step | Statement |
| --- | --- |
| Given | `nil`, `true`, `false`, an Integer and a Float |
| When | Ruby hands each to the engine and takes it back |
| Then | it gets the same value of the same class |

## `RV-002` A String crosses as a copy

| Step | Statement |
| --- | --- |
| Given | a String Ruby handed to the engine |
| When | Ruby changes its String and takes the engine's back |
| Then | the engine's is the String as it was handed |

## `RV-003` A Symbol crosses as a StringName

| Step | Statement |
| --- | --- |
| Given | a Symbol |
| When | Ruby hands it to the engine and takes it back |
| Then | the engine held a StringName, and Ruby gets the same Symbol |

## `RV-004` A String is taken where the engine wants a name or a path

| Step | Statement |
| --- | --- |
| Given | an engine method taking a StringName or a NodePath |
| When | Ruby calls it with a String |
| Then | the engine takes the String as the name or path it spells |

## `RV-005` An Array and a Hash cross as copies of their elements

| Step | Statement |
| --- | --- |
| Given | an Array and a Hash holding values that cross |
| When | Ruby hands each to the engine and takes it back |
| Then | it gets an equal Array and an equal Hash, not the ones it handed |

## `RV-006` A container nested too deep crosses as nothing

| Step | Statement |
| --- | --- |
| Given | an Array nested more than 100 deep, or holding itself |
| When | Ruby hands it to the engine |
| Then | it raises `Godot::CallError` and the engine is given nothing |

## `RV-007` An engine object crosses as the same object

| Step | Statement |
| --- | --- |
| Given | an engine object Ruby holds, and a node whose script's Ruby object Ruby holds |
| When | the engine hands each back to Ruby |
| Then | Ruby gets an object equal to the engine object, and the script's own Ruby object |

## `RV-008` An answer the engine cannot take reaches it as null

| Step | Statement |
| --- | --- |
| Given | a node script's method answering an Array that holds itself |
| When | Godot calls the method |
| Then | Godot gets null, and the log names the method, the answer and why it cannot cross |

## `RV-009` A container nested too deep never reaches Ruby

| Step | Statement |
| --- | --- |
| Given | a node script's method, and an Array that holds itself |
| When | Godot calls the method with the Array |
| Then | the method is not called, and the log names the method and why the Array cannot cross |
