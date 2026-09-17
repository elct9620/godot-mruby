//! How a node script is announced to the editor, which lists it by name as
//! it lists a GDScript's `class_name`: read from headers and ancestries, so
//! the editor scans a project without running any of it.

use crate::ancestry::{self, Ancestry};
use crate::parser::Header;
use crate::realm::{self, Files};

/// What the editor lists a node script by.
#[derive(Debug, PartialEq, Eq)]
pub struct Announcement {
    /// The class's own name, without its namespaces, since the editor's list
    /// of classes is flat.
    pub name: String,
    /// The nearest announced class the class inherits from, or else its
    /// engine class.
    pub base: String,
    /// The path `icon` is called with, as written.
    pub icon: Option<String>,
    pub is_tool: bool,
    pub is_abstract: bool,
}

/// Why a file is not announced.
#[derive(Debug, PartialEq, Eq)]
pub enum Unannounced {
    /// A file under a test directory, which belongs to no game's class list.
    InTestDirectory,
    /// A library file, or one whose ancestry is broken.
    NotNodeScript,
    /// Node scripts in other files share the name, so no one of them is
    /// listed by it, whatever order the editor scans them in.
    SharedName { name: String, others: Vec<String> },
}

/// The project a node script is announced in: its files, its test
/// directories, and which engine classes are node classes.
pub struct Project<'a, F: Files> {
    files: &'a F,
    paths: Vec<String>,
    test_directories: &'a [String],
    is_node: &'a dyn Fn(&str) -> bool,
}

impl<'a, F: Files> Project<'a, F> {
    pub fn new(
        files: &'a F,
        test_directories: &'a [String],
        is_node: &'a dyn Fn(&str) -> bool,
    ) -> Self {
        Self {
            files,
            paths: files.paths(),
            test_directories,
            is_node,
        }
    }

    /// The announcement of the file at `path`, or why it has none.
    pub fn announce(&self, path: &str) -> Result<Announcement, Unannounced> {
        if self.in_test_directory(path) {
            return Err(Unannounced::InTestDirectory);
        }
        let (header, ancestry) = self.node_script(path).ok_or(Unannounced::NotNodeScript)?;
        let others = self.sharing_name(path);
        if !others.is_empty() {
            return Err(Unannounced::SharedName {
                name: header.name().to_owned(),
                others,
            });
        }
        let base = ancestry
            .files()
            .iter()
            .find(|(ancestor, _)| {
                !self.in_test_directory(ancestor) && self.sharing_name(ancestor).is_empty()
            })
            .map_or_else(
                || ancestry.engine_class().to_owned(),
                |(_, header)| header.name().to_owned(),
            );
        Ok(Announcement {
            name: header.name().to_owned(),
            base,
            icon: header.icon().map(str::to_owned),
            is_tool: header.is_tool(),
            is_abstract: header.is_abstract(),
        })
    }

    // The header and ancestry of the file at `path`, when it is a node script.
    fn node_script(&self, path: &str) -> Option<(Header, Ancestry)> {
        let header = Header::read(path, &self.files.source(path).ok()?);
        let ancestry = ancestry::read(path, &header, self.files).ok()?;
        (self.is_node)(ancestry.engine_class()).then_some((header, ancestry))
    }

    // The other game files whose node scripts share the name of the file at
    // `path`: only a file whose last segment spells the same name can.
    fn sharing_name(&self, path: &str) -> Vec<String> {
        let name = realm::key_of(path).pop();
        self.paths
            .iter()
            .filter(|other| other.as_str() != path && realm::key_of(other).pop() == name)
            .filter(|other| !self.in_test_directory(other) && self.node_script(other).is_some())
            .cloned()
            .collect()
    }

    fn in_test_directory(&self, path: &str) -> bool {
        self.test_directories.iter().any(|directory| {
            path.strip_prefix(directory.trim_end_matches('/'))
                .is_some_and(|rest| rest.starts_with('/'))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Announcement, Project, Unannounced};
    use crate::ancestry::tests::Sources;

    fn announce(
        path: &str,
        sources: &[(&'static str, &'static str)],
    ) -> Result<Announcement, Unannounced> {
        let files = Sources(sources.iter().copied().collect());
        let test_directories = ["res://test".to_owned()];
        let is_node = |class: &str| class != "Resource";
        Project::new(&files, &test_directories, &is_node).announce(path)
    }

    const ENEMY: (&str, &str) = ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n");

    // @behavior RN-001
    #[test]
    fn a_node_script_is_announced_by_its_classs_name_without_its_namespaces() {
        let sources = [(
            "res://enemies/boss.rb",
            "module Enemies\n  class Boss < Godot::Node2D\n  end\nend\n",
        )];

        let announcement = announce("res://enemies/boss.rb", &sources).unwrap();

        assert_eq!(announcement.name, "Boss");
    }

    // @behavior RN-002
    #[test]
    fn a_node_script_inheriting_from_no_announced_file_is_announced_with_its_engine_class_as_base()
    {
        let announcement = announce("res://enemy.rb", &[ENEMY]).unwrap();

        assert_eq!(announcement.base, "Node2D");
    }

    // @behavior RN-003
    #[test]
    fn a_node_script_extending_an_announced_class_is_announced_with_that_class_as_base() {
        let sources = [("res://boss.rb", "class Boss < Enemy\nend\n"), ENEMY];

        let announcement = announce("res://boss.rb", &sources).unwrap();

        assert_eq!(announcement.base, "Enemy");
    }

    // @behavior RN-004
    #[test]
    fn a_file_under_a_test_directory_is_not_announced() {
        let sources = [("res://test/dummy.rb", "class Dummy < Godot::Node\nend\n")];

        let unannounced = announce("res://test/dummy.rb", &sources).unwrap_err();

        assert_eq!(unannounced, Unannounced::InTestDirectory);
    }

    // @behavior RN-005
    #[test]
    fn a_library_file_is_not_announced() {
        let sources = [("res://save.rb", "class Save < Godot::Resource\nend\n")];

        let unannounced = announce("res://save.rb", &sources).unwrap_err();

        assert_eq!(unannounced, Unannounced::NotNodeScript);
    }

    // @behavior RN-006
    #[test]
    fn node_scripts_sharing_a_name_are_not_announced() {
        let sources = [
            (
                "res://enemies/boss.rb",
                "module Enemies\n  class Boss < Godot::Node\n  end\nend\n",
            ),
            (
                "res://levels/boss.rb",
                "module Levels\n  class Boss < Godot::Node\n  end\nend\n",
            ),
        ];

        let unannounced = announce("res://enemies/boss.rb", &sources).unwrap_err();

        assert_eq!(
            unannounced,
            Unannounced::SharedName {
                name: "Boss".to_owned(),
                others: vec!["res://levels/boss.rb".to_owned()],
            }
        );
    }

    // @behavior RN-007
    #[test]
    fn a_base_sharing_its_name_is_passed_over_for_the_next() {
        let sources = [
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ENEMY,
            (
                "res://levels/enemy.rb",
                "module Levels\n  class Enemy < Godot::Node\n  end\nend\n",
            ),
        ];

        let announcement = announce("res://boss.rb", &sources).unwrap();

        assert_eq!(announcement.base, "Node2D");
    }

    // @behavior RN-008
    #[test]
    fn an_announcement_carries_whether_the_class_is_a_tool() {
        let sources = [("res://gizmo.rb", "class Gizmo < Godot::Node\n  tool\nend\n")];

        let announcement = announce("res://gizmo.rb", &sources).unwrap();

        assert!(announcement.is_tool);
    }

    // @behavior RN-009
    #[test]
    fn an_announcement_carries_whether_the_class_is_abstract() {
        let sources = [(
            "res://enemy.rb",
            "class Enemy < Godot::Node\n  abstract\nend\n",
        )];

        let announcement = announce("res://enemy.rb", &sources).unwrap();

        assert!(announcement.is_abstract);
    }

    // @behavior RN-010
    #[test]
    fn an_announcement_carries_the_classs_icon() {
        let sources = [(
            "res://enemy.rb",
            "class Enemy < Godot::Node\n  icon \"enemy.svg\"\nend\n",
        )];

        let announcement = announce("res://enemy.rb", &sources).unwrap();

        assert_eq!(announcement.icon.as_deref(), Some("enemy.svg"));
    }
}
