# Values

How a value crosses between Ruby and the engine, either way: data is copied and objects are shared, so each side keeps what it holds and an object is the same object on both. A container either crosses whole or not at all, so neither side goes on with data the other never sent. The engine's own value types, such as `Vector2`, `Color` and `NodePath`, are classes under `Godot` whose values Ruby builds and computes with as GDScript does, but never changes, since each side holds its own copy.

## Includes

- `godot/test/unit/values/**/*.rb`
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

## `RV-010` An engine value type crosses as a value of its class under Godot

| Step | Statement |
| --- | --- |
| Given | an engine property holding a value type, such as a node's position |
| When | Ruby reads it |
| Then | it gets a value of the class under `Godot` named as the engine names the type, equal to one built from the same components |

## `RV-011` Ruby builds a value with the engine's constructors

| Step | Statement |
| --- | --- |
| Given | a value type under `Godot` |
| When | Ruby calls `new` with arguments one of the engine's constructors takes |
| Then | it gets the value that constructor builds, which the engine takes where it wants the type |

## `RV-012` A constructor no arguments fit raises Godot::CallError as GDScript reports it

| Step | Statement |
| --- | --- |
| Given | a value type under `Godot` |
| When | Ruby calls `new` with arguments none of the engine's constructors takes |
| Then | it raises `Godot::CallError` with the message GDScript reports for the same call |

## `RV-013` A value answers its members, methods and constants

| Step | Statement |
| --- | --- |
| Given | a value of a value type |
| When | Ruby reads a member, calls a method, calls a static method on the class, and names a constant of the class |
| Then | each answers what the engine answers for the type |

## `RV-014` A value answers the engine's operators

| Step | Statement |
| --- | --- |
| Given | values of value types, and a number |
| When | Ruby adds, multiplies, negates and compares them |
| Then | each answers what the engine's operator answers, and an operator the engine lacks for the operands raises `TypeError` with GDScript's message |

## `RV-015` A value cannot be changed

| Step | Statement |
| --- | --- |
| Given | a value of a value type with a member |
| When | Ruby assigns the member |
| Then | it raises `FrozenError`, and the value is as it was |

## `RV-016` A value prints as the engine prints it

| Step | Statement |
| --- | --- |
| Given | a value of a value type |
| When | Ruby turns it into a String |
| Then | it gets the String the engine prints for the value |

## `RV-017` A Proc or Method crosses as a Callable running it

| Step | Statement |
| --- | --- |
| Given | a Proc, and a Method Ruby took from an object |
| When | Ruby hands each to the engine and the engine calls it with arguments |
| Then | the Proc or Method runs with those arguments in the realm that made it, and the engine gets what it answered |

## `RV-018` A Proc connected to a signal runs when the signal is emitted

| Step | Statement |
| --- | --- |
| Given | an engine object's signal Ruby connected a Proc to |
| When | the engine emits the signal |
| Then | the Proc runs |

## `RV-019` Any other Ruby object crosses as the same object

| Step | Statement |
| --- | --- |
| Given | a Ruby object of a class of its own |
| When | Ruby hands it to the engine, alone and inside an Array, and takes each back |
| Then | Ruby gets the very object it handed, and the engine held a `RubyObject` |

## `RV-020` An engine Callable crosses as a value Ruby calls

| Step | Statement |
| --- | --- |
| Given | a Callable the engine hands Ruby |
| When | Ruby calls `call` on it with arguments |
| Then | the engine calls it with those arguments, and Ruby gets what it answered |
