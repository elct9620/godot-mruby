# Debugger

How a game run under Godot's debugger stops its Ruby: at a breakpoint, and line by line as the debugger steps, with the stack of Ruby frames it stopped in, as GDScript stops.

### What a stop does not show

- A frame names its file and line, not its method: the hook a stop runs in cannot read a method's name.
- A stop shows no variables and evaluates no expression: the hook runs no Ruby and reads no value.
- A step into a method that takes arguments stops at its `def` line, where mruby takes them in, before its body.
- A one-line block or loop stops once on its line, not at each pass.

## Includes

- `rust/src/debugger.rs`
- `rust/src/realm/debugger.rs`
- `tasks/support/debugger.rb`

## `SA-001` A breakpoint stops the Ruby at its line

| Step | Statement |
| --- | --- |
| Given | a node script with a breakpoint on a line its callback runs |
| When | the scene runs under the debugger |
| Then | the debugger breaks at the script's `res://` path and that line |

## `SA-002` A step stops at the next line of the frame

| Step | Statement |
| --- | --- |
| Given | the debugger stopped at a line that calls no method |
| When | the debugger steps |
| Then | it breaks at the frame's next line |

## `SA-003` A step into a call stops where the called method starts

| Step | Statement |
| --- | --- |
| Given | the debugger stopped at a line calling a Ruby method |
| When | the debugger steps |
| Then | it breaks in the called method, at the line it starts on |

## `SA-004` A stop's stack holds each Ruby frame, nearest first

| Step | Statement |
| --- | --- |
| Given | the debugger stopped in a method a callback called |
| When | the debugger lists the stack |
| Then | it lists the method's line, then the callback's line that called it, and no other frame |

## `SA-005` Stepping out stops at the caller's next line

| Step | Statement |
| --- | --- |
| Given | the debugger stopped in a method a callback called |
| When | the debugger steps out |
| Then | it breaks at the callback's line after the call |

## `SA-006` Stepping over a call stops at the frame's next line

| Attribute | Value |
| --- | --- |
| relates | `SA-003` |

| Step | Statement |
| --- | --- |
| Given | the debugger stopped at a line calling a Ruby method |
| When | the debugger steps over it |
| Then | it breaks at the frame's next line, not in the method |

## `SA-007` Continuing runs the Ruby on to its end

| Step | Statement |
| --- | --- |
| Given | the debugger stopped at a breakpoint no line after it holds |
| When | the debugger continues |
| Then | the callback runs to its end without breaking again |
