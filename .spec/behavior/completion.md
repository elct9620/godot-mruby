# Code completion

How the script editor completes a name in a Ruby file: the language reads where the caret stands from the tokens Prism lexes, never running the file, and offers the names Ruby resolves there, which the editor narrows by what is typed.

### What it does not see

- A receiver other than `Godot` names no class Prism can tell, so a call on a local variable or a method's result completes to nothing.

## Includes

- `rust/src/completion.rs`
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
