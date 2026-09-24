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

A signal of a class extending an engine class, named as Godot names it, with a name for each value it is emitted with. A node of the class has it as it has the engine's own signals, so it is connected to and emitted by those names.

```ruby
module Godot
  class Object
    def self.signal(name, *parameters)
    end
  end
end
```

## `Godot::Object.export`

A property of a class extending an engine class, taking its type from the value it is declared with. A node of the class has it as it has the engine's own properties, so the editor and the engine read and write it by that name, and the class reads and writes it as the instance variable of that name. A keyword tells the editor how to show it, each one named as the `@export_*` annotation it answers to: `range:` with a Range and an optional `step:`, `enum:` and `flags:` with the names a value may take, `file:` with the files to choose among, `dir:` and `multiline:` with `true`, and `placeholder:` with the text an empty field shows. `type:` names the class of an object Godot fills in, in place of a value naming the type itself.

```ruby
module Godot
  class Object
    def self.export(name, default, **keywords)
    end
  end
end
```

## `is_node_class`

Whether the engine class a name spells is a node class, the only kind a node script extends.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn is_node_class(class: &str) -> bool {}
```

## `node_key`

The key a realm holds a node's Ruby object under: the node's instance id, so the object is found from the node alone.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn node_key(node: InstanceId) -> Key {}
```

## `Owner`

A node as Ruby is given it, for the node script's class to make the node's Ruby object from.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Owner(pub InstanceId);
```

## `ToRuby`

An engine value as Ruby is given it, made only once the value is known to reach Ruby, so a container nested too deep never does.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct ToRuby;
```

## `ToRuby::try_new`

An engine value for Ruby, or why it cannot reach Ruby.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl ToRuby {
    pub fn try_new(variant: &Variant) -> Result<Self, String> {}
}
```

## `ToEngine`

A Ruby value as the engine takes it, its type following the Ruby value's own class.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct ToEngine(pub Variant);
```
