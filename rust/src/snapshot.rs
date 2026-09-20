//! What a realm has published of the classes its files define. Godot asks a
//! script its shape on any thread, so the answers are kept here, outside the
//! realm: asking never waits for the thread inside it, and what is read is
//! whole, since a realm publishes a class only once its file has run.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};

/// A signal a class declared, with the names its parameters were declared
/// with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signal {
    pub name: String,
    pub parameters: Vec<String>,
}

/// What the classes of a realm's files declared, as one value that never
/// changes once it is published.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    classes: HashMap<String, Class>,
}

#[derive(Clone, Debug, Default)]
struct Class {
    signals: Vec<Signal>,
}

impl Snapshot {
    /// The signals the class of the file at `path` declared, in the order it
    /// declared them; none for a file that has not run.
    pub fn signals(&self, path: &str) -> &[Signal] {
        self.classes
            .get(path)
            .map_or(&[], |class| class.signals.as_slice())
    }

    /// Takes what the class of the file at `path` declared as it ran, in
    /// place of what the file declared before it.
    pub fn declared(&mut self, path: &str, signals: Vec<Signal>) {
        self.classes.insert(path.to_owned(), Class { signals });
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

    #[test]
    fn a_file_that_declared_nothing_has_no_signals() {
        let snapshot = Snapshot::default();

        assert!(snapshot.signals("res://bell.rb").is_empty());
    }

    #[test]
    fn a_class_answers_the_signals_its_file_declared() {
        let mut snapshot = Snapshot::default();

        snapshot.declared("res://bell.rb", vec![rung()]);

        assert_eq!(snapshot.signals("res://bell.rb"), [rung()]);
    }

    #[test]
    fn a_file_running_again_declares_in_place_of_what_it_declared_before() {
        let mut snapshot = Snapshot::default();

        snapshot.declared("res://bell.rb", vec![rung()]);
        snapshot.declared("res://bell.rb", Vec::new());

        assert!(snapshot.signals("res://bell.rb").is_empty());
    }
}
