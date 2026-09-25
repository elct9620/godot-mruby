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

## `Heading`

A heading the editor shows the properties under: a category, as `export_category` writes one or as a class is headed by the file at `path`, or a group or subgroup, as `export_group` and `export_subgroup` write one, taking the properties whose names begin with `prefix`.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub enum Heading {
    Category { name: String, path: String },
    Group { name: String, prefix: String },
    Subgroup { name: String, prefix: String },
}
```

## `Heading::name`

The name the heading is shown with.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Heading {
    pub fn name(&self) -> &str {}
}
```

## `Heading::hint_string`

What Godot reads the heading with: a category's path, or the prefix a group or subgroup takes its properties by.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Heading {
    pub fn hint_string(&self) -> &str {}
}
```

## `Heading::usage`

The usage that tells Godot which kind of heading it is.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Heading {
    pub fn usage(&self) -> PropertyUsageFlags {}
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
    Heading(Heading),
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

What a class has once its file has run: what its body declared, and the methods it defines, those metaprogramming defined included, with the digest of the source it ran and the names that source's `export` calls write, which tell what the source Godot holds now has changed and what it cannot answer for.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Class {
    pub signals: Vec<Signal>,
    pub members: Vec<Member>,
    pub methods: Vec<String>,
    pub digest: u64,
    pub export_names: Vec<String>,
}
```

## `Source`

What a file's source writes, for its class to be answered from: what its header declares and the digest of that source.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub struct Source<'a> {
    pub exports: &'a [Export],
    pub digest: u64,
}
```

## `digest`

A digest of a file's source, which differs whenever the source does.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
pub fn digest(source: &str) -> u64 {}
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

## `Property::with_hint`

The property as the editor is to show it, `hint_string` being what the hint is read with.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Property {
    pub fn with_hint(self, hint: PropertyHint, hint_string: String) -> Property {}
}
```

## `Property::with_class`

The property as an object of the class `class` names, which Godot fills in: its type is the object's rather than the declared value's, as a class names a type no value has to carry.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Property {
    pub fn with_class(self, hint: PropertyHint, class: String) -> Property {}
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

## `Snapshot::has_run`

Whether the file at a path has run, which tells a class with nothing to declare from one whose declarations are still to come: a file that has not run is answered from its header instead.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn has_run(&self, path: &str) -> bool {}
}
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

The properties the classes of the files exported, the first file's first and one to a name, as a class has what it exported and what it inherits. Each file comes with what its source writes, as `members_of` takes it.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn properties_of<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Property> {
    }
}
```

## `Snapshot::members_of`

What the classes of the files have the editor show: each class's own category and then what it declared, the first file's first, as GDScript lists a script's members before the ones it inherits. A property is listed once, the nearest class's, while a heading belongs to the class that wrote it. Only a build of the editor carries the categories, as GDScript's are compiled out of a game's. Each file comes with what its source writes, which answers for it until it has run. Once it has, what the file declared as it ran answers while the source is the one it ran; a source changed since answers in the order it writes, taking what the run declared for an export it does not write out, and keeping what the run declared through a name no `export` call spelled.

| Attribute | Value |
| --- | --- |
| internal | yes |

```rust
impl Snapshot {
    pub fn members_of<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Member> {
    }
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
