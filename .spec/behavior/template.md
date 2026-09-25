# Script templates

What the editor offers when a Ruby script is created: the templates GDScript has built in, for the base classes a node script can extend, written in Ruby, and made into a script whose class is named after its file and extends the base the dialog was given.

### Left out

- GDScript's templates for `EditorScenePostImport`, `EditorScript`, `RichTextEffect` and `VisualShaderNodeCustom`: none of them is a node class, and only a node class can be a script.

## Includes

- `rust/src/template.rs`
- `tasks/support/template.rb`

## `RY-001` A base class offers the templates GDScript has for it

| Step | Statement |
| --- | --- |
| Given | a base class GDScript has built-in templates for that a node script can extend |
| When | the editor asks the language for that class's built-in templates |
| Then | it gets GDScript's templates of that class by name: `Object`'s Empty, `Node`'s Default, the Basic Movement of `CharacterBody2D` and of `CharacterBody3D`, and `EditorPlugin`'s Plugin |

## `RY-002` A template made for a file names its class after the file

| Step | Statement |
| --- | --- |
| Given | a template and the name of the file the dialog creates, such as `enemy_ship` |
| When | it is made into a script |
| Then | its class is the file's name as Zeitwerk camelizes it, `EnemyShip` |

## `RY-003` A template made on an engine class extends it under Godot

| Step | Statement |
| --- | --- |
| Given | the dialog's base class is an engine class, such as `Node2D` |
| When | a template is made into a script |
| Then | its class extends `Godot::Node2D` |

## `RY-004` A template made on a Ruby script extends that script's class

| Step | Statement |
| --- | --- |
| Given | the dialog's base is a Ruby script's path, as `"res://enemies/boss.rb"` |
| When | a template is made into a script |
| Then | its class extends the constant the path spells, written from the top level as `::Enemies::Boss` |

## `RY-005` Every built-in template compiles once made

| Step | Statement |
| --- | --- |
| Given | each built-in template |
| When | it is made into a script on its own base class |
| Then | its source compiles without an error |

## `RY-006` A template indents as the editor does

| Step | Statement |
| --- | --- |
| Given | the editor's indentation |
| When | a template is made into a script |
| Then | each level of its body is indented by it |

## `RY-007` A template saved under a namespace opens the namespaces its path spells

| Step | Statement |
| --- | --- |
| Given | a template made for a new file, which knows only the file's name |
| When | it is first saved at a path under a namespace, as `res://enemies/ships/hero_ship.rb` |
| Then | its class is written inside a module for each namespace the path spells, `Enemies` then `Ships`, each a level deeper, and a file under a root directory stays as it was made |

## `RY-008` A project's own Ruby template is listed by its meta lines

| Step | Statement |
| --- | --- |
| Given | a `.rb` template in the project's template directory whose `# meta-name:` line names it |
| When | the script dialog lists the templates for its base class |
| Then | it is listed under the project by that name |
