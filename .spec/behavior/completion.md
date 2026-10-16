# Code completion and lookup

How the script editor completes and looks up a name in a Ruby file: the language reads where the caret stands from the tokens Prism lexes, never running the file, and offers the names Ruby resolves there, which the editor narrows by what is typed; looking a name up goes where completion's name is defined, a line of the project or the engine's page.

### What it does not see

- A receiver other than `Godot` names no class Prism can tell, so a call on a local variable or a method's result completes to nothing.

## Includes

- `rust/src/completion.rs`
- `rust/src/lookup.rs`
- `tasks/support/completion.rb`

## `RQ-001` A name under Godot completes to the engine's classes and value types

| Step | Statement |
| --- | --- |
| Given | a name written after `Godot::` from the top level |
| When | the name is completed |
| Then | the engine's classes and value types are offered |

## `RQ-002` A Godot inside another namespace completes to no engine class

| Step | Statement |
| --- | --- |
| Given | a name written after `Godot::` inside another namespace, or inside a string |
| When | the name is completed |
| Then | no engine class is offered |

## `RQ-003` The script editor offers an engine class after Godot

| Step | Statement |
| --- | --- |
| Given | a node script open in the script editor |
| When | `Godot::Nod` is typed on a blank line and completion is asked for |
| Then | the editor offers `Node2D` |

## `RQ-004` The script editor offers a value type after Godot

| Step | Statement |
| --- | --- |
| Given | a node script open in the script editor |
| When | `Godot::Vec` is typed on a blank line and completion is asked for |
| Then | the editor offers `Vector2` |

## `RQ-005` A name after self completes to the script's and its engine class's members

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines and exports names, and whose engine class has methods, properties and constants |
| When | a name written after `self.` is completed |
| Then | the class's own methods and exports, then the engine class's methods and properties, are offered once each |

## `RQ-006` A bare name completes to the members and the constants in scope

| Step | Statement |
| --- | --- |
| Given | a node script whose class defines and exports names, whose engine class has methods, properties and constants, and constants the class index names around it |
| When | a name written with no receiver is completed |
| Then | the members after `self.` are offered, then the engine class's constants and the class index's |

## `RQ-007` A name after another receiver completes to nothing

| Step | Statement |
| --- | --- |
| Given | a name written after a local variable and a dot |
| When | the name is completed |
| Then | nothing is offered |

## `RQ-008` A file extending no engine class completes to its own names alone

| Step | Statement |
| --- | --- |
| Given | a file whose class extends no class under `Godot` |
| When | a name written with no receiver is completed |
| Then | only the file's methods and exports and the class index's constants are offered |

## `RQ-009` An engine method is written as Ruby calls it

| Step | Statement |
| --- | --- |
| Given | an engine method offered for completion |
| When | the editor lists and writes it |
| Then | one taking arguments is listed with `(…)` and opens its parentheses, and one taking none is listed with `()` and written by its name alone |

## `RQ-010` The script editor offers a node script's method after self

| Step | Statement |
| --- | --- |
| Given | a node script defining `point`, open in the script editor |
| When | `self.poi` is typed on a blank line and completion is asked for |
| Then | the editor offers `point` |

## `RQ-011` The script editor offers an engine method after self

| Step | Statement |
| --- | --- |
| Given | a node script extending `Godot::Node2D`, open in the script editor |
| When | `self.add_ch` is typed on a blank line and completion is asked for |
| Then | the editor offers `add_child(…)` |

## `RQ-012` The script editor offers a project's constant on a bare name

| Step | Statement |
| --- | --- |
| Given | a project whose root directory holds `battle.rb`, and a node script open in the script editor |
| When | `Batt` is typed on a blank line and completion is asked for |
| Then | the editor offers `Battle` |

## `RQ-013` A file's names include its own and its ancestors' methods

| Step | Statement |
| --- | --- |
| Given | a file being typed whose class extends a project class that extends `Godot::Node2D` |
| When | the names the file reaches are read |
| Then | its engine class is `Node2D`, and the methods its source as typed and its ancestor's file define are named |

## `RQ-014` The constants around a file are spelled as their files write them

| Step | Statement |
| --- | --- |
| Given | a file under a namespace directory, beside a top-level `http_client.rb` defining `HTTPClient` |
| When | the names the file reaches are read |
| Then | the constants at the top level and in its namespace are named, `HTTPClient` as its file spells it and a directory by its module's name |

## `RQ-015` The script editor offers a node script's export after self

| Step | Statement |
| --- | --- |
| Given | a node script exporting `width`, open in the script editor |
| When | `self.wid` is typed on a blank line and completion is asked for |
| Then | the editor offers `width` |

## `RQ-016` A method of the file or its ancestors leads to its def line

| Step | Statement |
| --- | --- |
| Given | a file being typed whose class defines a method and inherits another from a project class |
| When | either method's name is looked up |
| Then | it leads to the line of its `def`, in the file defining it |

## `RQ-017` A project's constant leads to the statement naming it

| Step | Statement |
| --- | --- |
| Given | a top-level library module in a file, and a directory with no file of its own name |
| When | either constant is looked up from a file beside them |
| Then | the module leads to its `module` statement's line, and the directory's module leads nowhere |

## `RQ-018` An engine name leads to the engine's page for it

| Step | Statement |
| --- | --- |
| Given | a node script extending `Godot::Node2D` |
| When | `add_child`, or `Node2D` after `Godot::`, is looked up |
| Then | it leads to the engine's page for that member of `Node2D`, or for the class |

## `RQ-019` A name completion would not offer leads nowhere

| Step | Statement |
| --- | --- |
| Given | a method name written after a local variable and a dot |
| When | the name is looked up |
| Then | it leads nowhere |

## `RQ-020` Looking up a node script's method reaches its def line

| Step | Statement |
| --- | --- |
| Given | a node script open in the script editor, with `point` typed on a blank line above its `def point` |
| When | `point` is looked up as a Ctrl-click does |
| Then | the caret moves to the line of `def point` |

## `RQ-021` Looking up an engine method opens its class's help page

| Step | Statement |
| --- | --- |
| Given | a node script extending `Godot::Node2D` open in the script editor, with `add_child` typed on a blank line |
| When | `add_child` is looked up as a Ctrl-click does |
| Then | the help page of `Node`, which declares it, opens |

## `RQ-022` Looking up a library constant opens its file at the statement naming it

| Step | Statement |
| --- | --- |
| Given | a node script open in the script editor, beside a file defining the library module `Milestone` |
| When | `Milestone` is looked up as a Ctrl-click does |
| Then | the editor opens that file with the caret on its `module Milestone` line |
