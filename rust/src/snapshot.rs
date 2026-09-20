//! What a realm has published of the classes its files define. Godot asks a
//! script its shape on any thread, so the answers are kept here, outside the
//! realm: asking never waits for the thread inside it, and what is read is
//! whole, since a realm publishes a class only once its file has run.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

use godot::builtin::{PackedByteArray, Variant, VariantType};
use godot::global::{bytes_to_var, var_to_bytes};

/// A signal a class declared, with the names its parameters were declared
/// with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signal {
    pub name: String,
    pub parameters: Vec<String>,
}

/// A property a class exported, as Godot reads it: the name it is written
/// and read by, the type its declared value gave it, and that value. The
/// value is kept as the bytes the engine writes it as, since a snapshot is
/// read from any thread while a Variant belongs to the one holding it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub kind: VariantType,
    default: Vec<u8>,
}

impl Property {
    /// The property `name`, taking its type from the value it is declared
    /// with.
    pub fn new(name: String, default: &Variant) -> Self {
        Self {
            name,
            kind: default.get_type(),
            default: var_to_bytes(default).to_vec(),
        }
    }

    /// The value the property was declared with.
    pub fn default_value(&self) -> Variant {
        bytes_to_var(&PackedByteArray::from(self.default.as_slice()))
    }
}

/// What a class has once its file has run: what its body declared, and the
/// methods it defines, those metaprogramming defined included.
#[derive(Clone, Debug, Default)]
pub struct Class {
    pub signals: Vec<Signal>,
    pub properties: Vec<Property>,
    pub methods: Vec<String>,
}

/// What the classes of a realm's files have, as one value that never changes
/// once it is published.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    classes: HashMap<String, Class>,
}

impl Snapshot {
    /// The signals the class of the file at `path` declared, in the order it
    /// declared them; none for a file that has not run.
    pub fn signals(&self, path: &str) -> &[Signal] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.signals.as_slice())
    }

    /// The properties the class of the file at `path` exported, in the order
    /// it declared them; none for a file that has not run.
    pub fn properties(&self, path: &str) -> &[Property] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.properties.as_slice())
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
    pub fn ran(&mut self, path: &str, class: Class) {
        self.classes.insert(path.to_owned(), class);
    }
}

// What the game's realm has published. A mod's realm will publish its own,
// keyed by the mod as its bookkeeping is.
static PUBLISHED: LazyLock<RwLock<Arc<Snapshot>>> = LazyLock::new(RwLock::default);

/// The snapshot published last, for answering Godot without entering a realm.
pub fn latest() -> Arc<Snapshot> {
    Arc::clone(&PUBLISHED.read().unwrap_or_else(PoisonError::into_inner))
}

/// Publishes `snapshot` for every thread to read from.
pub fn publish(snapshot: Arc<Snapshot>) {
    *PUBLISHED.write().unwrap_or_else(PoisonError::into_inner) = snapshot;
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
            default: Vec::new(),
        }
    }

    fn bell() -> Class {
        Class {
            signals: vec![rung()],
            properties: vec![tone()],
            methods: vec!["ring".to_owned()],
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
    fn a_class_answers_the_signals_its_file_declared() {
        let mut snapshot = Snapshot::default();

        snapshot.ran("res://bell.rb", bell());

        assert_eq!(snapshot.signals("res://bell.rb"), [rung()]);
    }

    #[test]
    fn a_file_that_has_not_run_has_no_properties() {
        let snapshot = Snapshot::default();

        assert!(snapshot.properties("res://bell.rb").is_empty());
    }

    #[test]
    fn a_class_answers_the_properties_its_file_exported() {
        let mut snapshot = Snapshot::default();

        snapshot.ran("res://bell.rb", bell());

        assert_eq!(snapshot.properties("res://bell.rb"), [tone()]);
    }

    #[test]
    fn a_class_answers_the_methods_it_defines() {
        let mut snapshot = Snapshot::default();

        snapshot.ran("res://bell.rb", bell());

        assert!(snapshot.has_method("res://bell.rb", "ring"));
    }

    #[test]
    fn a_file_running_again_stands_in_place_of_what_it_had_before() {
        let mut snapshot = Snapshot::default();

        snapshot.ran("res://bell.rb", bell());
        snapshot.ran("res://bell.rb", Class::default());

        assert!(snapshot.signals("res://bell.rb").is_empty());
        assert!(snapshot.properties("res://bell.rb").is_empty());
        assert!(!snapshot.has_method("res://bell.rb", "ring"));
    }
}
