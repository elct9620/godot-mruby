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

## `Group`

A heading the editor shows the properties under, as `export_group` and its kin write one, or as a class is headed by the file it is written in: the name it is headed with, what the heading is read with — the prefix a group takes its properties by, or the path a class's category is at — and which kind of heading it is.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Group {
    pub name: String,
    pub hint_string: String,
    pub usage: PropertyUsageFlags,
}
```

## `Member`

What a class body declared for the editor to show, in the order it was declared: a property, or a heading the properties after it are under.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub enum Member {
    Property(Property),
    Group(Group),
}
```

## `Member::property`

The property this member is, unless it is a heading.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Member {
    pub fn property(&self) -> Option<&Property> {}
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
    pub members: Vec<Member>,
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

## `Property::of_class`

The property as an object of the class `class` names, which Godot fills in: its type is the object's rather than the declared value's, as a class names a type no value has to carry.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Property {
    pub fn of_class(self, hint: PropertyHint, class: String) -> Property {}
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

## `Snapshot::members`

What the class of the file at a path declared for the editor, in the order it declared it; none for a file that has not run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn members(&self, path: &str) -> &[Member] {}
}
```

## `Snapshot::properties`

The properties the class of the file at a path exported, in the order it declared them; none for a file that has not run.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn properties(&self, path: &str) -> impl Iterator<Item = &Property> {}
}
```

## `Snapshot::properties_of`

The properties the classes of the files at the paths exported, the first file's first and one to a name, as a class has what it exported and what it inherits.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn properties_of<'a>(&self, paths: impl IntoIterator<Item = &'a str>) -> Vec<Property> {}
}
```

## `Snapshot::members_of`

What the classes of the files at the paths have the editor show: each class's own category and then what it declared, the first file's first, as GDScript lists a script's members before the ones it inherits. A property is listed once, the nearest class's, while a heading belongs to the class that wrote it. Only a build of the editor carries the categories, as GDScript's are compiled out of a game's.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn members_of<'a>(&self, paths: impl IntoIterator<Item = &'a str>) -> Vec<Member> {}
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
