# Header

How a file's header is read from its source without running it: the constants its `module` and `class` statements write, the class the file's path names and the name it is written with, the superclass written on it and the namespaces it is looked up from, the names of the methods it defines, the signals, properties and headings its body declares, and the `tool`, `abstract` and `icon` its body calls.

## Includes

- `rust/src/header.rs`

## `RH-001` A class written inside its path's namespaces is the file's class

| Step | Statement |
| --- | --- |
| Given | a file whose class is written inside `module` statements spelling its path's namespaces |
| When | its header is read |
| Then | the header carries the superclass written on that class |

## `RH-002` A class written with its whole constant path is the file's class

| Step | Statement |
| --- | --- |
| Given | a file whose class statement spells the constant its path names in full |
| When | its header is read |
| Then | the header carries the superclass written on that class |

## `RH-003` A class whose name only matches its path apart from underscores and case is the file's class

| Step | Statement |
| --- | --- |
| Given | a file `http_client.rb` defining `HTTPClient` |
| When | its header is read |
| Then | the header carries the superclass written on that class |

## `RH-004` A method of a class nested in the file's class is not the file's

| Step | Statement |
| --- | --- |
| Given | a file whose class holds another class defining a method |
| When | its header is read |
| Then | the header does not carry that method |

## `RH-005` A file with a syntax error still has a header

| Step | Statement |
| --- | --- |
| Given | a file whose class and its methods come before a syntax error |
| When | its header is read |
| Then | the header carries the class's superclass and methods |

## `RH-006` A module file has no superclass

| Step | Statement |
| --- | --- |
| Given | a file whose path names a module |
| When | its header is read |
| Then | the header carries no superclass |

## `RH-007` A superclass that is not a constant is not carried

| Step | Statement |
| --- | --- |
| Given | a file whose class extends what a method call returns |
| When | its header is read |
| Then | the header carries no superclass |

## `RH-008` `tool` called in the class body makes the header a tool's

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `tool` |
| When | its header is read |
| Then | the header says the class is a tool |

## `RH-009` `abstract` called in the class body makes the header an abstract class's

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `abstract` |
| When | its header is read |
| Then | the header says the class is abstract |

## `RH-010` A call inside a method of the class is not the class body's

| Step | Statement |
| --- | --- |
| Given | a file whose class defines a method calling `tool` |
| When | its header is read |
| Then | the header does not say the class is a tool |

## `RH-011` A superclass is looked up from the namespaces its class statement is written in

| Step | Statement |
| --- | --- |
| Given | a file whose class is written inside `module` statements, extending a constant |
| When | its header is read |
| Then | the header carries those modules as the namespaces the superclass is looked up from |

## `RH-012` A superclass on a class written with its whole constant path is looked up from the top level

| Step | Statement |
| --- | --- |
| Given | a file whose top-level class statement spells its constant path in full, extending a constant |
| When | its header is read |
| Then | the header carries no namespace the superclass is looked up from |

## `RH-013` `icon` called in the class body with a string carries that path

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `icon` with a string literal |
| When | its header is read |
| Then | the header carries the path that string names as the class's icon |

## `RH-014` The class's name is the last name its class statement writes

| Step | Statement |
| --- | --- |
| Given | a file whose class statement spells its constant path in full |
| When | its header is read |
| Then | the header carries the last name of that path as the class's name |

## `RH-015` Every constant a `module` or `class` statement writes is carried

| Step | Statement |
| --- | --- |
| Given | a file whose statements open its path's namespaces, its own class, a class inside that class, and another constant |
| When | its header is read |
| Then | the header carries each of those constants, spelled as written from the top level |

## `RH-016` A `class` statement inside a method writes nothing the header carries

| Step | Statement |
| --- | --- |
| Given | a file whose method body holds a `class` statement |
| When | its header is read |
| Then | the header does not carry that constant |

## `RH-017` `signal` called in the class body carries the signal it declares

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `signal` with a name and a parameter keyword |
| When | its header is read |
| Then | the header carries that signal with that parameter name, of any type until the file runs |

## `RH-018` A signal declared with a name that is not written out is not carried

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `signal` with a local variable |
| When | its header is read |
| Then | the header carries no signal |

## `RH-019` An icon written relative to the file is taken from the file's directory

| Attribute | Value |
| --- | --- |
| refines | `RS-076`, `RS-025` |

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `icon` with a path relative to the file's directory |
| When | its header is read |
| Then | the header carries the icon's path from the project's root |

## `RH-020` An icon written from the project's root keeps its path

| Attribute | Value |
| --- | --- |
| refines | `RS-076` |

| Step | Statement |
| --- | --- |
| Given | a file whose class body calls `icon` with a `res://` path |
| When | its header is read |
| Then | the header carries that path, simplified, as the class's icon |

## `RH-021` `abstract` given methods marks them, and the header holds them

| Step | Statement |
| --- | --- |
| Given | a file whose class body writes `abstract def draw`, `abstract :area, "edges"`, `abstract def self.count`, `def self.total` before `abstract :total`, and `def self.sum` and `def sum` before `abstract :sum` |
| When | its header is read |
| Then | the header holds `draw`, `area`, `edges` and `sum` among the class's methods, marked abstract, and `count` and `total` only among the methods on the class itself |

## `RH-022` A class body holding a statement outside the read forms has unknown methods

| Step | Statement |
| --- | --- |
| Given | a file whose class body holds `attr_reader`, `alias`, `define_method`, `include`, `class << self`, a project's own call, a call on a receiver or with a block, `def` on another object, a known call or constant given an operand that is itself such a call or a path off one, or a nested class with such a superclass or a body with `rescue` |
| When | its header is read |
| Then | the header says the class has methods its source may not name |

## `RH-023` A class body of only the read forms has known methods

| Step | Statement |
| --- | --- |
| Given | a file whose class body holds only `def`, `def self.`, `Godot::Object`'s DSL, `private`, `protected` or `public` over `def`s and names, constants given literals, collections, ranges or values made under `Godot`, and nested classes |
| When | its header is read |
| Then | the header knows every method the class defines, `private def` ones included |

## `RH-024` `abstract` judges each name as Ruby evaluates the call

| Step | Statement |
| --- | --- |
| Given | a file whose class body writes `def x` before `abstract def self.x`, and `abstract :y, def self.y` |
| When | its header is read |
| Then | the header marks only `x` abstract |

## `RH-025` A file holding a statement outside its class's nesting has unknown methods

| Step | Statement |
| --- | --- |
| Given | a file whose top level or a module around its class holds anything but `module` and `class` statements, or that opens its class again, gives its class a body with `rescue` or a superclass outside the read forms, or holds another class whose name, superclass or body is outside them |
| When | its header is read |
| Then | the header says the class has methods its source may not name |

## `RH-026` A `def` after `abstract` takes the name back

| Step | Statement |
| --- | --- |
| Given | a file whose class body writes `abstract :area`, then `def area` |
| When | its header is read |
| Then | `area` is among the class's methods and not marked abstract |

## `RH-027` A name a visibility call is given is the objects' beside a `def self.`

| Step | Statement |
| --- | --- |
| Given | a file whose class extends one defining `area` and whose body writes `def self.area`, `private :area`, then `abstract :area` |
| When | its header is read |
| Then | the header marks `area` abstract |

## `RH-028` A name `export` is given is not the objects' beside a `def self.`

| Step | Statement |
| --- | --- |
| Given | a file whose class body writes `export :area`, `def self.area`, then `abstract :area` |
| When | its header is read |
| Then | the header marks nothing abstract, since whether `export` defines `area` on the class turns on ancestors the header does not read |
