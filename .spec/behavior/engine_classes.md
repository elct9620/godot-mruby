# Engine classes

How Ruby names and uses the engine's classes: every class the engine registers is a Ruby class under `Godot`, made at its first use and inheriting as the engine's class does, so a node script can name the class it extends, and a class extending one can mark itself in its body with `tool`, `abstract` and `icon`, which Godot reads from the file's source. Making an object of an engine class makes the engine's object, and Ruby calls the engine's methods, properties, constants, singletons and static methods by their names, failing the way GDScript's untyped calls fail.

## Includes

- `godot/test/unit/engine_classes/**/*_test.rb`
- `tasks/support/runner.rb`
- `rust/src/bridge/bound_method.rs`

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
| When | Ruby calls one of its engine methods with arguments, leaving out any it has defaults for |
| Then | the engine runs the method with those arguments and its defaults for those left out, and Ruby gets what it answers |

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

## `RG-018` A call with the wrong number of arguments raises ArgumentError as Ruby reports it

| Step | Statement |
| --- | --- |
| Given | an engine object |
| When | Ruby calls one of its engine methods with too few arguments, or too many, whether or not the method has default arguments |
| Then | it raises `ArgumentError` saying, as mruby does, how many arguments were given and how many the method requires |

## `RG-019` An engine method named like a Ruby method is reached through call

| Step | Statement |
| --- | --- |
| Given | an engine object whose engine class has a method Ruby's `Object` also defines |
| When | Ruby calls the name, and calls `call` with the name |
| Then | the first keeps Ruby's meaning, and `call` reaches the engine's method |

## `RG-020` Calling a static method writes nothing to the log

| Step | Statement |
| --- | --- |
| Given | an engine class the engine keeps no singleton of |
| When | Ruby calls one of its static methods |
| Then | the log carries nothing about a singleton, as a GDScript call's does not |

## `RG-021` A method only some of a node class's engine objects have stays theirs

| Step | Statement |
| --- | --- |
| Given | a node class extending an engine class, made the script of a node of that class and of a node of a class extending it |
| When | the second node's Ruby object calls an engine method only its engine class has, and the first node's then calls it |
| Then | the second reaches the engine's method, and the first raises `NoMethodError` |

## `RG-022` A node class's object stands only for an engine object of the class it extends

| Step | Statement |
| --- | --- |
| Given | a node class extending an engine class |
| When | Ruby makes the class's object for an engine object of a class that does not extend that one |
| Then | it raises `TypeError`, so no engine method of the class is called on that object |

## `RG-023` An engine object inspects as Godot prints it

| Step | Statement |
| --- | --- |
| Given | an engine object, one holding instance variables, and one that has been freed |
| When | Ruby inspects each |
| Then | each names its Ruby class and the engine object as Godot prints it, `<Class#id>` or `<Freed Object>`, after which the instance variables follow as Ruby inspects them |

## `RG-024` An engine object's copy stands for the same engine object

| Step | Statement |
| --- | --- |
| Given | an object of an engine class |
| When | Ruby calls `dup` or `clone` on it |
| Then | it gets another object of the same class standing for the same engine object, equal to the first and carrying its instance variables |

## `RG-025` A node class's object refuses to be copied

| Step | Statement |
| --- | --- |
| Given | an object of a node class |
| When | Ruby calls `dup` or `clone` on it |
| Then | it raises `TypeError`, since a node has one Ruby object, and `duplicate` copies the node itself |

## `RG-026` An engine class allocates no empty object

| Step | Statement |
| --- | --- |
| Given | an engine class or a node class |
| When | Ruby calls `allocate` on it |
| Then | it raises `TypeError`, so every object stands for an engine object |

## `RG-027` A call with an argument the engine cannot take raises TypeError as GDScript reports it

| Step | Statement |
| --- | --- |
| Given | an engine object |
| When | Ruby calls one of its engine methods with an argument of a type the method cannot take |
| Then | it raises `TypeError` with the message GDScript's untyped call reports for the same call |

## `RG-028` A node class's method calling super into an engine method stays its own

| Step | Statement |
| --- | --- |
| Given | a node class defining a method of an engine method's name that calls `super` |
| When | Ruby calls the method twice on the class's object |
| Then | both calls run the class's method, and each reaches the engine's method |

## `RG-029` An engine method a node class's object called is none of the class's own

| Step | Statement |
| --- | --- |
| Given | a node class whose object has called an engine method |
| When | Ruby lists the methods the class defines itself |
| Then | the engine method is not among them, so the script answers no method of that name to Godot |

## `RG-030` An engine object's clone keeps its singleton methods and frozen state

| Step | Statement |
| --- | --- |
| Given | a frozen object of an engine class with a singleton method |
| When | Ruby calls `clone` on it |
| Then | the copy answers the singleton method and is frozen, as Ruby's `clone` keeps both |

## `RG-031` An engine object's copy runs initialize_copy with the original

| Step | Statement |
| --- | --- |
| Given | an engine class defining `initialize_copy` |
| When | Ruby calls `dup` or `clone` on one of its objects |
| Then | `initialize_copy` runs on the copy with the original, as Ruby's copies run it |

## `RG-032` A clone stands for no engine object while initialize_copy runs

| Step | Statement |
| --- | --- |
| Given | an engine class whose `initialize_copy` calls an engine method on the copy |
| When | Ruby calls `clone` on one of its objects |
| Then | the call raises `TypeError`, since the copy takes the engine object only after `initialize_copy` returns |

## `RG-033` A property the engine reads by index reads and writes as GDScript's

| Step | Statement |
| --- | --- |
| Given | an engine object whose class has a property its getter and setter reach by an index, such as a Control's `offset_left` |
| When | Ruby writes the property by its name, then reads it |
| Then | it reads what was written, as GDScript's `control.offset_left` does |

## `RG-034` A freed engine object answers respond_to? by its engine class

| Step | Statement |
| --- | --- |
| Given | an engine object that has been freed |
| When | Ruby asks whether it responds to a name, one its engine class has a method for and one it has none for |
| Then | it answers true and false, as its engine class has them, rather than raising |

## `RG-035` A node's engine object reaches the engine's method its node class also defines

| Step | Statement |
| --- | --- |
| Given | a node whose node class defines a method of an engine method's name, and Ruby holding the node's engine object rather than the class's object |
| When | Ruby calls that name on the engine object |
| Then | the engine's method runs and the node class's does not, as GDScript's typed call reaches the engine's |

## `RG-037` An engine method's bind is refused a wrong number of arguments before the engine runs it

| Step | Statement |
| --- | --- |
| Given | an engine method the engine gave a bind for, which a game exported without the engine's debug checks calls too |
| When | Ruby calls it with fewer arguments than it requires, or more than it takes |
| Then | the call is refused without reaching the engine |

## `RG-038` An engine method's bind is refused a receiver of a class not declaring it

| Step | Statement |
| --- | --- |
| Given | the bind of an engine method one engine class declares, and an engine object of a class neither that one nor extending it |
| When | Ruby calls the bind on that object |
| Then | it raises `Godot::CallError` without reaching the engine |
