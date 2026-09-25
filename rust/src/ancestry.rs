//! A file's ancestry: the files its class inherits from, read from their
//! headers without running any of them, so a class extending an engine class
//! through other files is known as a node script before it runs.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::header::Header;
use crate::realm::{self, Files};
use crate::snapshot::{Member, Property, Snapshot, Source};

/// The files a class inherits from, nearest first, and the engine class the
/// farthest of them extends.
#[derive(Debug)]
pub struct Ancestry {
    files: Vec<(String, Header)>,
    engine_class: String,
}

impl Ancestry {
    /// Each file the class inherits from, by path, with its header.
    pub fn files(&self) -> &[(String, Header)] {
        &self.files
    }

    /// The class under `Godot::` the farthest file extends, as `Node2D`.
    pub fn engine_class(&self) -> &str {
        &self.engine_class
    }
}

// How many times a source has changed. An ancestry is read from the sources
// of the files a class inherits from, so any change may change it.
static CHANGES: AtomicU64 = AtomicU64::new(0);

/// Lets go of every ancestry kept so far, since a source changed.
pub fn expire() {
    CHANGES.fetch_add(1, Ordering::AcqRel);
}

/// An ancestry kept from the first question that needs it until a source
/// changes.
#[derive(Default)]
pub struct Cache(Mutex<Option<Entry>>);

// An ancestry as read, and how many changes there had been when it was read.
type Entry = (u64, Result<Arc<Ancestry>, Break>);

impl Cache {
    /// The ancestry kept, or what `read` answers when none is kept since the
    /// last change. It reads outside the lock, so a source changing while it
    /// reads leaves what it read to be read again.
    pub fn ancestry(
        &self,
        read: impl FnOnce() -> Result<Ancestry, Break>,
    ) -> Result<Arc<Ancestry>, Break> {
        let changes = CHANGES.load(Ordering::Acquire);
        if let Some((kept_at, ancestry)) = &*self.entry()
            && *kept_at == changes
        {
            return ancestry.clone();
        }
        let ancestry = read().map(Arc::new);
        *self.entry() = Some((changes, ancestry.clone()));
        ancestry
    }

    fn entry(&self) -> MutexGuard<'_, Option<Entry>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The files a class takes its shape from, each with its header: its own,
/// then the ones it inherits from, nearest first, since a declaration is
/// inherited. A file whose ancestry broke takes its shape from itself alone.
pub struct Lineage<'a> {
    path: String,
    header: &'a Header,
    ancestry: Option<Arc<Ancestry>>,
}

impl<'a> Lineage<'a> {
    /// The lineage of the class of the file at `path`, read as `header`,
    /// whose ancestry is `ancestry` when it has one.
    pub fn new(path: String, header: &'a Header, ancestry: Option<Arc<Ancestry>>) -> Self {
        Self {
            path,
            header,
            ancestry,
        }
    }

    /// Whether the class is, or inherits from, the class of the file at
    /// `path`.
    pub fn has_file(&self, path: &str) -> bool {
        self.files().any(|(file, _)| file == path)
    }

    /// Each file by path, with its header.
    pub fn files(&self) -> impl Iterator<Item = (&str, &Header)> {
        let inherited = self
            .ancestry
            .as_ref()
            .map_or(&[][..], |ancestry| ancestry.files.as_slice());
        std::iter::once((self.path.as_str(), self.header)).chain(
            inherited
                .iter()
                .map(|(path, header)| (path.as_str(), header)),
        )
    }

    /// The properties the class exported, its ancestors' included, nearest
    /// first: what each file declared as it ran, or what its header writes
    /// while it has not run.
    pub fn properties(&self, snapshot: &Snapshot) -> Vec<Property> {
        snapshot.properties_of(self.sources())
    }

    /// What the class declared for the editor, its ancestors' included and in
    /// the order each class wrote it; a file that has not run has the
    /// properties its header writes and no heading.
    pub fn members(&self, snapshot: &Snapshot) -> Vec<Member> {
        snapshot.members_of(self.sources())
    }

    // Each file by path, with what its source writes.
    fn sources(&self) -> impl Iterator<Item = (&str, Source<'_>)> {
        self.files().map(|(path, header)| {
            (
                path,
                Source {
                    exports: header.exports(),
                    digest: header.digest(),
                },
            )
        })
    }

    /// Whether a file of the lineage defines the method `name`, as its source
    /// writes it or as its class defined it while it ran.
    pub fn has_method(&self, snapshot: &Snapshot, name: &str) -> bool {
        self.files()
            .any(|(path, header)| header.has_method(name) || snapshot.has_method(path, name))
    }
}

/// Why a file has no ancestry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Break {
    /// A superclass, as written, that names no one file.
    NoFile(String),
    /// The file at this path comes back as its own ancestor.
    Cycle(String),
    /// No engine node class is reached: a file on the way writes no superclass
    /// that is a constant, or the engine class reached is no node's.
    NoEngineClass,
    /// The source of the file at `path` could not be read.
    NoSource { path: String, reason: String },
}

/// Says why, following the path of the file whose ancestry broke.
impl fmt::Display for Break {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoFile(superclass) => write!(f, "extends {superclass}, which no one file names"),
            Self::Cycle(path) => write!(
                f,
                "inherits from {path} again on the way to an engine class"
            ),
            Self::NoEngineClass => write!(f, "defines no class extending an engine node class"),
            Self::NoSource { path, reason } => {
                write!(f, "inherits from {path}, which cannot be read: {reason}")
            }
        }
    }
}

/// The ancestry of the file at `path`, whose header is `header`, among
/// `files`, each superclass found as the realm's loader would find it.
pub fn ancestry_of(path: &str, header: &Header, files: &impl Files) -> Result<Ancestry, Break> {
    let paths = files.paths();
    let roots = files.roots();
    let mut passed = vec![path.to_owned()];
    let mut ancestors = Vec::new();
    let mut superclass = header.superclass().cloned();
    loop {
        let written = superclass.ok_or(Break::NoEngineClass)?;
        if let [godot, engine_class] = written.names()
            && godot == "Godot"
        {
            return Ok(Ancestry {
                files: ancestors,
                engine_class: engine_class.clone(),
            });
        }
        let file = realm::file_by_name(
            paths.clone(),
            roots.clone(),
            written.scope(),
            written.names(),
        )
        .ok_or_else(|| Break::NoFile(written.names().join("::")))?;
        if passed.contains(&file) {
            return Err(Break::Cycle(file));
        }
        let source = files.source(&file).map_err(|reason| Break::NoSource {
            path: file.clone(),
            reason,
        })?;
        let ancestor = Header::from_source(&file, &source, &roots);
        superclass = ancestor.superclass().cloned();
        passed.push(file.clone());
        ancestors.push((file, ancestor));
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::{Ancestry, Break, Lineage, ancestry_of};
    use crate::header::Header;
    use crate::realm::{Declarations, Files};

    /// Files kept in memory, by path.
    pub struct Sources(pub BTreeMap<&'static str, &'static str>);

    impl Files for Sources {
        fn paths(&self) -> Vec<String> {
            self.0.keys().map(|path| (*path).to_owned()).collect()
        }

        fn source(&self, path: &str) -> Result<String, String> {
            self.0
                .get(path)
                .map(|source| (*source).to_owned())
                .ok_or_else(|| "no such file".to_owned())
        }

        fn declarations(&self, _path: &str) -> Declarations {
            Declarations::default()
        }
    }

    fn ancestry(path: &str, sources: &[(&'static str, &'static str)]) -> Result<Ancestry, Break> {
        let files = Sources(sources.iter().copied().collect());
        let header = Header::from_source(path, &files.source(path).unwrap(), &files.roots());
        ancestry_of(path, &header, &files)
    }

    fn nearest(ancestry: &Ancestry) -> &str {
        &ancestry.files()[0].0
    }

    const BOSS: &str = "res://enemies/boss.rb";
    const NESTED_BOSS: &str = "module Enemies\n  class Boss < Enemy\n  end\nend\n";

    // @behavior RI-001
    #[test]
    fn a_superclass_is_found_in_the_innermost_namespace_first() {
        let sources = [
            (BOSS, NESTED_BOSS),
            (
                "res://enemies/enemy.rb",
                "module Enemies\n  class Enemy < Godot::Node2D\n  end\nend\n",
            ),
            ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n"),
        ];

        let ancestry = ancestry(BOSS, &sources).unwrap();

        assert_eq!(nearest(&ancestry), "res://enemies/enemy.rb");
    }

    // @behavior RI-002
    #[test]
    fn a_superclass_the_innermost_namespace_lacks_is_found_further_out() {
        let sources = [
            (BOSS, NESTED_BOSS),
            ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n"),
        ];

        let ancestry = ancestry(BOSS, &sources).unwrap();

        assert_eq!(nearest(&ancestry), "res://enemy.rb");
    }

    // @behavior RI-003
    #[test]
    fn a_superclass_written_inside_a_namespace_is_found_in_that_namespace() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemies::Enemy\nend\n"),
            (
                "res://enemies/enemy.rb",
                "module Enemies\n  class Enemy < Godot::Node2D\n  end\nend\n",
            ),
        ];

        let ancestry = ancestry("res://boss.rb", &sources).unwrap();

        assert_eq!(nearest(&ancestry), "res://enemies/enemy.rb");
    }

    // @behavior RI-004
    #[test]
    fn an_ancestry_ends_at_the_engine_class_its_farthest_file_extends() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n"),
        ];

        let ancestry = ancestry("res://boss.rb", &sources).unwrap();

        assert_eq!(ancestry.engine_class(), "Node2D");
    }

    // @behavior RI-005
    #[test]
    fn a_superclass_no_file_names_breaks_the_ancestry() {
        let sources = [("res://boss.rb", "class Boss < Enemy\nend\n")];

        let broken = ancestry("res://boss.rb", &sources).unwrap_err();

        assert_eq!(broken, Break::NoFile("Enemy".to_owned()));
    }

    // @behavior RI-006
    #[test]
    fn a_superclass_two_files_name_breaks_the_ancestry() {
        let sources = [
            ("res://boss.rb", "class Boss < HttpEnemy\nend\n"),
            (
                "res://http_enemy.rb",
                "class HttpEnemy < Godot::Node\nend\n",
            ),
            ("res://httpenemy.rb", "class HttpEnemy < Godot::Node\nend\n"),
        ];

        let broken = ancestry("res://boss.rb", &sources).unwrap_err();

        assert_eq!(broken, Break::NoFile("HttpEnemy".to_owned()));
    }

    // @behavior RI-007
    #[test]
    fn an_ancestry_that_comes_back_to_a_file_it_passed_is_broken() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ("res://enemy.rb", "class Enemy < Boss\nend\n"),
        ];

        let broken = ancestry("res://boss.rb", &sources).unwrap_err();

        assert_eq!(broken, Break::Cycle("res://boss.rb".to_owned()));
    }

    // @behavior RI-008
    #[test]
    fn an_ancestry_that_reaches_no_engine_class_is_broken() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ("res://enemy.rb", "class Enemy\nend\n"),
        ];

        let broken = ancestry("res://boss.rb", &sources).unwrap_err();

        assert_eq!(broken, Break::NoEngineClass);
    }

    // @behavior RI-009
    #[test]
    fn a_class_has_the_files_it_is_and_inherits_from_and_no_other() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n"),
            ("res://ally.rb", "class Ally < Godot::Node2D\nend\n"),
        ];
        let files = Sources(sources.iter().copied().collect());
        let header = Header::from_source("res://boss.rb", sources[0].1, &files.roots());
        let ancestry = ancestry_of("res://boss.rb", &header, &files)
            .map(Arc::new)
            .ok();

        let lineage = Lineage::new("res://boss.rb".to_owned(), &header, ancestry);

        assert!(lineage.has_file("res://boss.rb"));
        assert!(lineage.has_file("res://enemy.rb"));
        assert!(!lineage.has_file("res://ally.rb"));
    }
}
