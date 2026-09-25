//! What a realm has published of the classes its files define. Godot asks a
//! script its shape on any thread, so the answers are kept here, outside the
//! realm: asking never waits for the thread inside it, and what is read is
//! whole, since a realm publishes a class only once its file has run.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, OnceLock, PoisonError, RwLock};

use godot::builtin::{PackedByteArray, Variant, VariantType};
use godot::classes::Os;
use godot::global::{bytes_to_var, var_to_bytes};
use godot::obj::Singleton;
use godot::register::info::{PropertyHint, PropertyUsageFlags};

use crate::header::Export;

/// A signal a class declared, with the names its parameters were declared
/// with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signal {
    pub name: String,
    pub parameters: Vec<String>,
}

/// A property a class exported, as Godot reads it: the name it is written
/// and read by, the type its declared value gave it, that value, and the
/// hint the editor shows it with. The value is kept as the bytes the engine
/// writes it as, since a snapshot is read from any thread while a Variant
/// belongs to the one holding it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub kind: VariantType,
    pub hint: PropertyHint,
    pub hint_string: String,
    default: Vec<u8>,
}

impl Property {
    /// The property `name`, taking its type from the value it is declared
    /// with and shown as the editor shows a property of that type.
    pub fn new(name: String, default: &Variant) -> Self {
        Self {
            name,
            kind: default.get_type(),
            hint: PropertyHint::NONE,
            hint_string: String::new(),
            default: var_to_bytes(default).to_vec(),
        }
    }

    /// The property as the editor is to show it, `hint_string` being what
    /// the hint is read with.
    pub fn with_hint(self, hint: PropertyHint, hint_string: String) -> Self {
        Self {
            hint,
            hint_string,
            ..self
        }
    }

    /// The property as an object of the class `class` names, which Godot
    /// fills in: its type is the object's rather than the declared value's,
    /// as a class names a type no value has to carry.
    pub fn with_class(self, hint: PropertyHint, class: String) -> Self {
        Self {
            kind: VariantType::OBJECT,
            hint,
            hint_string: class,
            ..self
        }
    }

    /// The value the property was declared with.
    pub fn default_value(&self) -> Variant {
        bytes_to_var(&PackedByteArray::from(self.default.as_slice()))
    }
}

/// A heading the editor shows the properties under: a category, as
/// `export_category` writes one or as a class is headed by the file at
/// `path`, or a group or subgroup, as `export_group` and `export_subgroup`
/// write one, taking the properties whose names begin with `prefix`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heading {
    Category { name: String, path: String },
    Group { name: String, prefix: String },
    Subgroup { name: String, prefix: String },
}

impl Heading {
    /// The name the heading is shown with.
    pub fn name(&self) -> &str {
        match self {
            Self::Category { name, .. }
            | Self::Group { name, .. }
            | Self::Subgroup { name, .. } => name,
        }
    }

    /// What Godot reads the heading with: a category's path, or the prefix a
    /// group or subgroup takes its properties by.
    pub fn hint_string(&self) -> &str {
        match self {
            Self::Category { path, .. } => path,
            Self::Group { prefix, .. } | Self::Subgroup { prefix, .. } => prefix,
        }
    }

    /// The usage that tells Godot which kind of heading it is.
    pub fn usage(&self) -> PropertyUsageFlags {
        match self {
            Self::Category { .. } => PropertyUsageFlags::CATEGORY,
            Self::Group { .. } => PropertyUsageFlags::GROUP,
            Self::Subgroup { .. } => PropertyUsageFlags::SUBGROUP,
        }
    }
}

/// What a class body declared for the editor to show, in the order it was
/// declared: a property, or a heading the properties after it are under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Member {
    Property(Property),
    Heading(Heading),
}

impl Member {
    /// The property this member is, unless it is a heading.
    pub fn property(&self) -> Option<&Property> {
        match self {
            Self::Property(property) => Some(property),
            Self::Heading(_) => None,
        }
    }
}

/// What a class has once its file has run: what its body declared, and the
/// methods it defines, those metaprogramming defined included, with the
/// digest of the source it ran and the names that source's `export` calls
/// write, which tell what the source Godot holds now has changed and what it
/// cannot answer for.
#[derive(Clone, Debug, Default)]
pub struct Class {
    pub signals: Vec<Signal>,
    pub members: Vec<Member>,
    pub methods: Vec<String>,
    pub digest: u64,
    pub export_names: Vec<String>,
}

impl Class {
    // The property of that name the class declared.
    fn property(&self, name: &str) -> Option<&Property> {
        self.members
            .iter()
            .filter_map(Member::property)
            .find(|property| property.name == name)
    }

    // Whether `member` is a property the class exported through a name no
    // `export` call of the source it ran spelled, which only running tells.
    fn is_exported_unwritten(&self, member: &Member) -> bool {
        member
            .property()
            .is_some_and(|property| !self.export_names.contains(&property.name))
    }
}

/// What a file's source writes, for its class to be answered from: what its
/// header declares and the digest of that source.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub exports: &'a [Export],
    pub digest: u64,
}

/// A digest of a file's source, which differs whenever the source does.
pub fn digest(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// What the classes of a realm's files have, as one value that never changes
/// once it is published.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    classes: HashMap<String, Class>,
}

impl Snapshot {
    /// Whether the file at `path` has run, which is what tells a class with
    /// nothing to declare from one whose declarations are still to come: a
    /// file that has not run is answered from its header instead.
    pub fn has_run(&self, path: &str) -> bool {
        self.classes.contains_key(path)
    }

    /// The signals the class of the file at `path` declared, in the order it
    /// declared them; none for a file that has not run.
    pub fn signals(&self, path: &str) -> &[Signal] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.signals.as_slice())
    }

    /// What the class of the file at `path` declared for the editor, in the
    /// order it declared it; none for a file that has not run.
    pub fn members(&self, path: &str) -> &[Member] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.members.as_slice())
    }

    /// The properties the class of the file at `path` exported, in the order
    /// it declared them; none for a file that has not run.
    pub fn properties(&self, path: &str) -> impl Iterator<Item = &Property> {
        self.members(path).iter().filter_map(Member::property)
    }

    /// The properties the classes of the files at `files` exported, the
    /// first file's first and one to a name, as a class has what it
    /// exported and what it inherits. Each file comes with what its source
    /// writes, as `members_of` takes it.
    pub fn properties_of<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Property> {
        let mut properties: Vec<Property> = Vec::new();
        for (path, source) in files {
            for member in self.file_members(path, source) {
                if let Member::Property(property) = member
                    && !properties.iter().any(|kept| kept.name == property.name)
                {
                    properties.push(property);
                }
            }
        }
        properties
    }

    /// What the classes of the files at `files` have the editor show: each
    /// class's own category and then what it declared, the first file's
    /// first, as GDScript lists a script's members before the ones it
    /// inherits. A property is listed once, the nearest class's, while a
    /// heading belongs to the class that wrote it. Each file comes with what
    /// its source writes, which answers for it until it has run. Once it
    /// has, what the file declared as it ran answers while the source is the
    /// one it ran; a source changed since answers in the order it writes,
    /// taking what the run declared for an export it does not write out, and
    /// keeping what the run declared through a name no `export` call spelled.
    pub fn members_of<'a>(
        &self,
        files: impl IntoIterator<Item = (&'a str, Source<'a>)>,
    ) -> Vec<Member> {
        let mut members: Vec<Member> = Vec::new();
        for (path, source) in files {
            if is_editor_build() {
                members.push(Member::Heading(category(path)));
            }
            for member in self.file_members(path, source) {
                let listed = member.property().is_some_and(|property| {
                    members
                        .iter()
                        .filter_map(Member::property)
                        .any(|kept| kept.name == property.name)
                });
                if !listed {
                    members.push(member);
                }
            }
        }
        members
    }

    // What the class of the file at `path` declared for the editor, as
    // `members_of` answers each file.
    fn file_members(&self, path: &str, source: Source) -> Vec<Member> {
        let Some(class) = self.classes.get(path) else {
            return source.exports.iter().filter_map(Export::member).collect();
        };
        if class.digest == source.digest {
            return class.members.clone();
        }
        let mut members: Vec<Member> = Vec::new();
        for export in source.exports {
            let member = match export {
                Export::Member(member) => Some(member.clone()),
                Export::Name { name, bare } => class
                    .property(name)
                    .or(bare.as_ref())
                    .cloned()
                    .map(Member::Property),
            };
            members.extend(member);
        }
        members.extend(
            class
                .members
                .iter()
                .filter(|member| class.is_exported_unwritten(member))
                .cloned(),
        );
        members
    }

    /// The methods the class of the file at `path` defines; none for a file
    /// that has not run.
    pub fn methods(&self, path: &str) -> &[String] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.methods.as_slice())
    }

    /// Whether the class of the file at `path` defines a method of that name.
    pub fn has_method(&self, path: &str, name: &str) -> bool {
        self.methods(path).iter().any(|method| method == name)
    }

    /// Takes what the class of the file at `path` has, now that the file has
    /// run, in place of what it had before.
    pub fn record_class(&mut self, path: &str, class: Class) {
        self.classes.insert(path.to_owned(), class);
    }
}

// Whether this is a build of the editor, which is what a category is for:
// GDScript's are compiled out of a game's build, so a game's properties
// carry none either. The build never changes, so the engine is asked once.
fn is_editor_build() -> bool {
    static EDITOR: OnceLock<bool> = OnceLock::new();
    *EDITOR.get_or_init(|| Os::singleton().has_feature("editor"))
}

// The category the editor heads a class's own members with, as it heads a
// GDScript's with the script it is written in: the file's name, read with
// the path it is at.
fn category(path: &str) -> Heading {
    Heading::Category {
        name: path.rsplit('/').next().unwrap_or(path).to_owned(),
        path: path.to_owned(),
    }
}

// What the game's realm has published. A mod's realm will publish its own,
// keyed by the mod as its bookkeeping is.
static SNAPSHOT: LazyLock<RwLock<Arc<Snapshot>>> = LazyLock::new(RwLock::default);

/// The snapshot published last, for answering Godot without entering a realm.
pub fn latest() -> Arc<Snapshot> {
    Arc::clone(&SNAPSHOT.read().unwrap_or_else(PoisonError::into_inner))
}

/// Publishes `snapshot` for every thread to read from.
pub fn publish(snapshot: Arc<Snapshot>) {
    *SNAPSHOT.write().unwrap_or_else(PoisonError::into_inner) = snapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rung() -> Signal {
        Signal {
            name: "rung".to_owned(),
            parameters: vec!["times".to_owned()],
        }
    }

    // A property as a class exported it; its value is the bytes the engine
    // would write, which no test of the snapshot alone can ask it for.
    fn tone() -> Property {
        Property {
            name: "tone".to_owned(),
            kind: VariantType::FLOAT,
            hint: PropertyHint::NONE,
            hint_string: String::new(),
            default: Vec::new(),
        }
    }

    fn bell() -> Class {
        Class {
            signals: vec![rung()],
            members: vec![Member::Property(tone())],
            methods: vec!["ring".to_owned()],
            ..Class::default()
        }
    }

    #[test]
    fn a_file_that_has_not_run_has_no_signals() {
        let snapshot = Snapshot::default();

        assert!(snapshot.signals("res://bell.rb").is_empty());
    }

    #[test]
    fn a_file_that_has_not_run_has_no_methods() {
        let snapshot = Snapshot::default();

        assert!(!snapshot.has_method("res://bell.rb", "ring"));
    }

    #[test]
    fn a_file_that_ran_without_declaring_anything_has_still_run() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", Class::default());

        assert!(snapshot.has_run("res://bell.rb"));
        assert!(!snapshot.has_run("res://lamp.rb"));
    }

    #[test]
    fn a_class_answers_the_signals_its_file_declared() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());

        assert_eq!(snapshot.signals("res://bell.rb"), [rung()]);
    }

    #[test]
    fn a_file_that_has_not_run_has_no_properties() {
        let snapshot = Snapshot::default();

        assert_eq!(snapshot.properties("res://bell.rb").count(), 0);
    }

    #[test]
    fn a_class_answers_the_properties_its_file_exported() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());

        assert_eq!(
            snapshot.properties("res://bell.rb").collect::<Vec<_>>(),
            [&tone()]
        );
    }

    #[test]
    fn a_class_answers_the_methods_it_defines() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());

        assert!(snapshot.has_method("res://bell.rb", "ring"));
    }

    #[test]
    fn a_file_running_again_stands_in_place_of_what_it_had_before() {
        let mut snapshot = Snapshot::default();

        snapshot.record_class("res://bell.rb", bell());
        snapshot.record_class("res://bell.rb", Class::default());

        assert!(snapshot.signals("res://bell.rb").is_empty());
        assert_eq!(snapshot.properties("res://bell.rb").count(), 0);
        assert!(!snapshot.has_method("res://bell.rb", "ring"));
    }
}
