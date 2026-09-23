# Engine classes

How Ruby names and uses the engine's classes: every class the engine registers is a Ruby class under `Godot`, made at its first use and inheriting as the engine's class does, so a node script can name the class it extends, and a class extending one can mark itself in its body with `tool`, `abstract` and `icon`, which Godot reads from the file's source. Making an object of an engine class makes the engine's object, and Ruby calls the engine's methods, properties, constants, singletons and static methods by their names, failing the way GDScript's untyped calls fail.

## Includes

- `godot/test/unit/engine_classes/**/*_test.rb`

## `RG-001` An engine class is a class under Godot

| Step | Statement |
| --- | --- |
| Given | a class the engine registers |
| When | Ruby first uses its name under `Godot` |
| Then | it gets a class of that name |

## `RG-002` An engine class inherits from its engine parent

| Step | Statement |
| --- | --- |
| Given | an engine class whose engine parent is another engine class |
| When | Ruby asks the class under `Godot` for its superclass |
| Then | it gets the parent's class under `Godot` |

## `RG-003` A name the engine has no class for raises NameError

| Step | Statement |
| --- | --- |
| Given | a name the engine registers no class for |
| When | Ruby uses it under `Godot` |
| Then | it raises `NameError` |

## `RG-004` An engine class stays after a file that first used it raises

| Step | Statement |
| --- | --- |
| Given | a file loaded by name that first uses an engine class, then raises |
| When | Ruby uses the failing file's constant and the exception is rescued |
| Then | the engine class is still defined under `Godot` |

## `RG-005` A class extending an engine class calls `tool`, `abstract` and `icon` in its body

| Step | Statement |
| --- | --- |
| Given | a class extending an engine class |
| When | its body calls `tool`, `abstract` and `icon` with a path |
| Then | the class is defined |

## `RG-006` An abstract class cannot be made

| Step | Statement |
| --- | --- |
| Given | a class extending an engine class whose body calls `abstract` |
| When | Ruby makes an object of it |
| Then | it raises `NotImplementedError` |

## `RG-007` A class extending an abstract class is not abstract itself

| Step | Statement |
| --- | --- |
| Given | a class extending an engine class whose body calls `abstract` |
| Given | a class extending that class |
| When | Ruby makes an object of the second class |
| Then | it gets that object |

## `RG-008` An engine class makes the engine's object

| Step | Statement |
| --- | --- |
| Given | an engine class the engine can make objects of |
| When | Ruby calls `new` on its class under `Godot` |
| Then | it gets an object of that class whose engine methods answer |

## `RG-009` An engine class the engine cannot make raises NotImplementedError

| Step | Statement |
| --- | --- |
| Given | an engine class the engine makes no objects of |
| When | Ruby calls `new` on its class under `Godot` |
| Then | it raises `NotImplementedError` |

## `RG-010` Ruby calls an engine object's method by its name

| Step | Statement |
| --- | --- |
| Given | an engine object |
| When | Ruby calls one of its engine methods with arguments |
| Then | the engine runs the method with those arguments and Ruby gets what it answers |

## `RG-011` A name the engine object has no method for raises NoMethodError

| Step | Statement |
| --- | --- |
| Given | an engine object |
| When | Ruby calls a method neither Ruby nor the engine defines on it |
| Then | it raises `NoMethodError` |

## `RG-012` A freed engine object raises Godot::CallError

| Step | Statement |
| --- | --- |
| Given | an engine object that has been freed |
| When | Ruby calls one of its engine methods |
| Then | it raises `Godot::CallError` saying, as GDScript does, that the method was called on a previously freed instance |

## `RG-013` A reference-counted object lives while Ruby holds it

| Step | Statement |
| --- | --- |
| Given | a reference-counted engine object Ruby made and nothing else refers to |
| When | Ruby runs the garbage collector and calls one of its engine methods |
| Then | the engine object answers |

## `RG-014` Ruby reads and writes an engine property by its name

| Step | Statement |
| --- | --- |
| Given | an engine object with a property |
| When | Ruby assigns the property by its name and reads it back |
| Then | Ruby gets the value it assigned |

## `RG-015` An engine class names its integer constants

| Step | Statement |
| --- | --- |
| Given | an engine class with an integer constant and an enum |
| When | Ruby names the constant and one of the enum's values under the class |
| Then | it gets the engine's integer for each |

## `RG-016` An engine singleton answers its methods on its class

| Step | Statement |
| --- | --- |
| Given | an engine class the engine keeps a singleton of |
| When | Ruby calls one of the singleton's methods on the class under `Godot` |
| Then | it gets what the singleton answers |

## `RG-017` An engine class answers its static methods

| Step | Statement |
| --- | --- |
| Given | an engine class with a static method |
| When | Ruby calls the method on the class under `Godot` |
| Then | it gets what the method answers |

## `RG-018` A call the engine refuses raises Godot::CallError as GDScript reports it

| Step | Statement |
| --- | --- |
| Given | an engine object |
| When | Ruby calls one of its engine methods with too few arguments, or with an argument of a type the method cannot take |
| Then | it raises `Godot::CallError` with the message GDScript's untyped call reports for the same call |

## `RG-019` An engine method named like a Ruby method is reached through call

| Step | Statement |
| --- | --- |
| Given | an engine object whose engine class has a method Ruby's `Object` also defines |
| When | Ruby calls the name, and calls `call` with the name |
| Then | the first keeps Ruby's meaning, and `call` reaches the engine's method |
