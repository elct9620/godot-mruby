//! A file's ancestry: the files its class inherits from, read from their
//! headers without running any of them, so a class extending an engine class
//! through other files is known as a node script before it runs.

use std::fmt;

use crate::parser::Header;
use crate::realm::{self, Files};

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

    /// Whether a file the class inherits from defines the method `name`.
    pub fn has_method(&self, name: &str) -> bool {
        self.files.iter().any(|(_, header)| header.has_method(name))
    }
}

/// Why a file has no ancestry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Broken {
    /// A superclass, as written, that names no one file.
    Unnamed(String),
    /// The file at this path comes back as its own ancestor.
    Cycle(String),
    /// No engine node class is reached: a file on the way writes no superclass
    /// that is a constant, or the engine class reached is no node's.
    NoEngineClass,
    /// The source of the file at `path` could not be read.
    Unread { path: String, reason: String },
}

/// Says why, following the path of the file whose ancestry broke.
impl fmt::Display for Broken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unnamed(superclass) => write!(f, "extends {superclass}, which no one file names"),
            Self::Cycle(path) => write!(
                f,
                "inherits from {path} again on the way to an engine class"
            ),
            Self::NoEngineClass => write!(f, "defines no class extending an engine node class"),
            Self::Unread { path, reason } => {
                write!(f, "inherits from {path}, which cannot be read: {reason}")
            }
        }
    }
}

/// The ancestry of the file at `path`, whose header is `header`, among
/// `files`, each superclass found as the realm's loader would find it.
pub fn read(path: &str, header: &Header, files: &impl Files) -> Result<Ancestry, Broken> {
    let paths = files.paths();
    let mut passed = vec![path.to_owned()];
    let mut ancestors = Vec::new();
    let mut superclass = header.superclass().cloned();
    loop {
        let written = superclass.ok_or(Broken::NoEngineClass)?;
        if let [godot, engine_class] = written.names()
            && godot == "Godot"
        {
            return Ok(Ancestry {
                files: ancestors,
                engine_class: engine_class.clone(),
            });
        }
        let file = realm::file_named(paths.clone(), written.scope(), written.names())
            .ok_or_else(|| Broken::Unnamed(written.names().join("::")))?;
        if passed.contains(&file) {
            return Err(Broken::Cycle(file));
        }
        let source = files.source(&file).map_err(|reason| Broken::Unread {
            path: file.clone(),
            reason,
        })?;
        let ancestor = Header::read(&file, &source);
        superclass = ancestor.superclass().cloned();
        passed.push(file.clone());
        ancestors.push((file, ancestor));
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::BTreeMap;

    use super::{Ancestry, Broken, read};
    use crate::parser::Header;
    use crate::realm::Files;

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
    }

    fn ancestry(path: &str, sources: &[(&'static str, &'static str)]) -> Result<Ancestry, Broken> {
        let files = Sources(sources.iter().copied().collect());
        let header = Header::read(path, &files.source(path).unwrap());
        read(path, &header, &files)
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

        assert_eq!(broken, Broken::Unnamed("Enemy".to_owned()));
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

        assert_eq!(broken, Broken::Unnamed("HttpEnemy".to_owned()));
    }

    // @behavior RI-007
    #[test]
    fn an_ancestry_that_comes_back_to_a_file_it_passed_is_broken() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ("res://enemy.rb", "class Enemy < Boss\nend\n"),
        ];

        let broken = ancestry("res://boss.rb", &sources).unwrap_err();

        assert_eq!(broken, Broken::Cycle("res://boss.rb".to_owned()));
    }

    // @behavior RI-008
    #[test]
    fn an_ancestry_that_reaches_no_engine_class_is_broken() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ("res://enemy.rb", "class Enemy\nend\n"),
        ];

        let broken = ancestry("res://boss.rb", &sources).unwrap_err();

        assert_eq!(broken, Broken::NoEngineClass);
    }
}
