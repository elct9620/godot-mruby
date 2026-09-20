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
    pub methods: Vec<String>,
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
