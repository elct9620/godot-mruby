# Engine classes

How Ruby names the engine's classes: every class the engine registers is a Ruby class under `Godot`, made at its first use and inheriting as the engine's class does, so a node script can name the class it extends.

## Includes

- `godot/test/engine_classes/**/*.rb`

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
