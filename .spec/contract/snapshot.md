# Snapshot

What a realm publishes of the classes its files define, and the one place Godot's questions about a class's shape are answered from. It holds no Ruby value and takes no lock of the realm, so a script answers on any thread while the realm is busy; a realm writes it, and everything that answers Godot reads it.

## Includes

- `rust/src/snapshot.rs`

## `Signal`

A signal a class declared, with the names its parameters were declared with.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Signal {
    pub name: String,
    pub parameters: Vec<String>,
}
```

## `Class`

What a class has once its file has run: what its body declared, and the methods it defines, those metaprogramming defined included.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Class {
    pub signals: Vec<Signal>,
    pub properties: Vec<Property>,
    pub methods: Vec<String>,
}
```

## `Property`

A property a class exported, as Godot reads it: the name it is written and read by, the type its declared value gave it, that value, kept as bytes since a snapshot is read from any thread, and the hint the editor shows it with.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Property {
    pub name: String,
    pub kind: VariantType,
    pub hint: PropertyHint,
    pub hint_string: String,
}
```

## `Property::new`

The property of that name, taking its type from the value it is declared with and shown as the editor shows a property of that type.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Property {
    pub fn new(name: String, default: &Variant) -> Property {}
}
```

## `Property::hinted`

The property as the editor is to show it, `hint_string` being what the hint is read with.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Property {
    pub fn hinted(self, hint: PropertyHint, hint_string: String) -> Property {}
}
```

## `Property::default_value`

The value the property was declared with.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Property {
    pub fn default_value(&self) -> Variant {}
}
```

## `Snapshot`

What the classes of a realm's files have, as one value that never changes once it is published.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Snapshot;
```

## `Snapshot::signals`

The signals the class of the file at a path declared, in the order it declared them; none for a file that has not run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn signals(&self, path: &str) -> &[Signal] {}
}
```

## `Snapshot::properties`

The properties the class of the file at a path exported, in the order it declared them; none for a file that has not run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn properties(&self, path: &str) -> &[Property] {}
}
```

## `Snapshot::methods`

The methods the class of the file at a path defines; none for a file that has not run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn methods(&self, path: &str) -> &[String] {}
}
```

## `Snapshot::has_method`

Whether the class of the file at a path defines a method of that name.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn has_method(&self, path: &str, name: &str) -> bool {}
}
```

## `Snapshot::ran`

Takes what the class of the file at a path has, now that the file has run, in place of what it had before.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn ran(&mut self, path: &str, class: Class) {}
}
```

## `latest`

The snapshot published last, which is what answers Godot without entering a realm.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn latest() -> Arc<Snapshot> {}
```

## `publish`

Publishes a snapshot for every thread to read from.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn publish(snapshot: Arc<Snapshot>) {}
```
