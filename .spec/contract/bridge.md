# Bridge

How a node's script instance and the rest of the extension reach what Ruby sees of the engine: the `Godot` gem a game realm installs, and the values and objects that cross between the engine and Ruby. The instance keeps no Ruby value, so everything it hands Ruby or takes back passes through these.

## Includes

- `rust/src/bridge.rs`
- `rust/src/bridge/**/*.rs`
- `rust/src/bridge/**/*.rb`

## `Godot`

The gem every game realm installs: the `Godot` module, its engine classes, value types and utility functions.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Godot;
```

## `Godot::Object.signal`

A signal of a class extending an engine class, named as Godot names it, with a keyword for each value it is emitted with, naming the class of that value as GDScript types a signal's parameter: a Ruby class for the engine type it converts to, a class under `Godot` for that type or engine class, a node script's class for its objects, `Godot::Bool` for `true` and `false`, and `Godot::Variant` for a value of any type. A node of the class has it as it has the engine's own signals, so it is connected to and emitted by those names.

```ruby
module Godot
  class Object
    def self.signal(name, **parameters)
    end
  end
end
```

## `Godot::Bool`

What `true` and `false` are, as a class names a type: `TrueClass` and `FalseClass` take it in, since Ruby has no class of both.

```ruby
module Godot
  module Bool
  end
end
```

## `Godot::Variant`

A value of any type, as a class names a type: a signal's parameter of it is untyped, as one GDScript writes without a type.

```ruby
module Godot
  module Variant
  end
end
```

## `Godot::Object.rpc`

Lets peers call a method of a class extending an engine class over the network, as GDScript's `@rpc` does, taking its mode, sync, transfer mode and transfer channel by keywords of their names, the words as `@rpc` writes them, and each left out at `@rpc`'s default. Godot reads them from the script's rpc config.

```ruby
module Godot
  class Object
    def self.rpc(name, mode: :authority, sync: :call_remote, transfer_mode: :reliable, transfer_channel: 0)
    end
  end
end
```

## `Godot::Object#await`

Waits until a signal is emitted, in a method Godot calls whose body calls `await` or in a block or method Godot calls back, and answers what the signal was emitted with: nil, its one value, or its values in an array, as GDScript's `await` does. Godot gets nil from the call at once, and the method runs on once the signal is emitted. The signal is a `Godot::Signal`, or an object followed by the name of its signal; anywhere else, `await` raises `FiberError`.

```ruby
module Godot
  class Object
    def await(signal, name = nil)
    end
  end
end
```

## `Godot::Object.export`

A property of a class extending an engine class, taking its type from the value it is declared with. A node of the class has it as it has the engine's own properties, so the editor and the engine read and write it by that name, and the class reads and writes it as the instance variable of that name. A keyword tells the editor how to show it, each one named as the `@export_*` annotation it answers to and taking what that annotation takes: `range:` with a Range, an optional `step:` and `extra_hints:`; `enum:`, `flags:`, `exp_easing:` and `node_path:` with a list; `file:`, `file_path:` and `global_file:` with their filters; `placeholder:` and `tool_button:` with their text, a tool button declared with the name of the method it calls; `custom:` with a hint, what it is read with and its usage; and the others with `true`. `type:` names the class of an object Godot fills in, or in an array or a hash the classes a typed container holds.

```ruby
module Godot
  class Object
    def self.export(name, default, **keywords)
    end
  end
end
```

## `derive_object_key`

The key a realm holds a scripted object's Ruby object under: the object's instance id, so the Ruby object is found from the engine object alone.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn derive_object_key(object: InstanceId) -> Key {}
```

## `release_collected`

Lets go of the reference-counted engine objects Ruby let go of since the last call, which the collector leaves for it, since freeing one may call its script; the extension calls it every frame and once the realm closes.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn release_collected() {}
```

## `Owner`

A node as Ruby is given it, for the node script's class to make the node's Ruby object from.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Owner(pub InstanceId);
```

## `RubyCrossing`

An engine value crossing into Ruby, which always reaches it: a container crosses as the one the engine holds, however deep it nests.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct RubyCrossing<'a>(pub &'a Variant);
```

## `EngineCrossing`

A Ruby value crossing into the engine, its type following the Ruby value's own class.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct EngineCrossing(pub Variant);
```
