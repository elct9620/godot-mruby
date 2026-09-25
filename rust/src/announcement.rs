//! How a node script is announced to the editor, which lists it by name as
//! it lists a GDScript's `class_name`: read from headers and ancestries, so
//! the editor scans a project without running any of it.

use std::collections::BTreeSet;

use crate::ancestry::{self, Ancestry};
use crate::header::Header;
use crate::realm::{self, Files};
use crate::settings;

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
pub enum Omission {
    /// A file under a test directory, which belongs to no game's class list.
    InTestDirectory,
    /// A script template of the project's, which the editor fills in as text.
    InTemplateDirectory,
    /// A library file, or one whose ancestry is broken.
    NotNodeScript,
    /// Node scripts in other files share the name, so no one of them is
    /// listed by it, whatever order the editor scans them in.
    SharedName { name: String, others: Vec<String> },
}

/// The project a node script is announced in: its files, its test
/// directories and template directory, and which engine classes are node
/// classes.
pub struct Project<'a, F: Files> {
    files: &'a F,
    paths: Vec<String>,
    test_directories: Vec<String>,
    template_directory: String,
    is_node: &'a dyn Fn(&str) -> bool,
}

impl<'a, F: Files> Project<'a, F> {
    pub fn new(
        files: &'a F,
        test_directories: Vec<String>,
        template_directory: String,
        is_node: &'a dyn Fn(&str) -> bool,
    ) -> Self {
        Self {
            files,
            paths: files.paths(),
            test_directories,
            template_directory,
            is_node,
        }
    }

    /// The announcement of the file at `path`, or why it has none.
    pub fn announcement(&self, path: &str) -> Result<Announcement, Omission> {
        if self.is_in_test_directory(path) {
            return Err(Omission::InTestDirectory);
        }
        if self.is_in_template_directory(path) {
            return Err(Omission::InTemplateDirectory);
        }
        let (header, ancestry) = self.node_script(path).ok_or(Omission::NotNodeScript)?;
        let others = self.namesakes(path);
        if !others.is_empty() {
            return Err(Omission::SharedName {
                name: header.name().to_owned(),
                others,
            });
        }
        let base = ancestry
            .files()
            .iter()
            .find(|(ancestor, _)| {
                !self.is_in_test_directory(ancestor) && self.namesakes(ancestor).is_empty()
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
        let header = Header::from_source(path, &self.files.source(path).ok()?, &self.files.roots());
        let ancestry = ancestry::ancestry_of(path, &header, self.files).ok()?;
        (self.is_node)(ancestry.engine_class()).then_some((header, ancestry))
    }

    // The other game files whose node scripts share the name of the file at
    // `path`: only a file whose last segment spells the same name can.
    fn namesakes(&self, path: &str) -> Vec<String> {
        let name = file_name(path);
        self.paths
            .iter()
            .filter(|other| other.as_str() != path && file_name(other) == name)
            .filter(|other| !self.is_in_test_directory(other) && self.node_script(other).is_some())
            .cloned()
            .collect()
    }

    fn is_in_test_directory(&self, path: &str) -> bool {
        settings::is_in_directory(path, &self.test_directories)
    }

    fn is_in_template_directory(&self, path: &str) -> bool {
        settings::is_in_directory(path, std::slice::from_ref(&self.template_directory))
    }
}

/// The names node scripts share, each with the files sharing it, already
/// warned of: the editor scans the project again and again and asks of every
/// file, and each clash is warned of once while it lasts.
pub struct Clashes(BTreeSet<(String, Vec<String>)>);

impl Clashes {
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    /// Notes that the node scripts in `files` share `name`, answering whether
    /// that is news.
    pub fn note(&mut self, name: &str, files: &[String]) -> bool {
        self.0.insert((name.to_owned(), files.to_vec()))
    }

    /// Forgets the clashes the file at `path` was in, once it is announced or
    /// no node script: a clash coming back is news again.
    pub fn forget(&mut self, path: &str) {
        self.0
            .retain(|(_, files)| !files.iter().any(|file| file == path));
    }
}

/// The warning that node scripts in `others` share `name` with the file at
/// `path`.
pub fn shared_name_warning(path: &str, name: &str, others: &[String]) -> String {
    format!(
        "{path} and {} define node scripts named {name}, so none is listed by that name",
        others.join(" and ")
    )
}

// The last segment of the file at `path`, as the class index matches it.
fn file_name(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    realm::normalize(name.trim_end_matches(".rb"))
}

#[cfg(test)]
mod tests {
    use super::{Announcement, Clashes, Omission, Project};
    use crate::ancestry::tests::Sources;

    fn announcement(
        path: &str,
        sources: &[(&'static str, &'static str)],
    ) -> Result<Announcement, Omission> {
        let files = Sources(sources.iter().copied().collect());
        let test_directories = vec!["res://test".to_owned()];
        let is_node = |class: &str| class != "Resource";
        let template_directory = "res://script_templates".to_owned();
        Project::new(&files, test_directories, template_directory, &is_node).announcement(path)
    }

    const ENEMY: (&str, &str) = ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n");

    // @behavior RN-001
    #[test]
    fn a_node_script_is_announced_by_its_classs_name_without_its_namespaces() {
        let sources = [(
            "res://enemies/boss.rb",
            "module Enemies\n  class Boss < Godot::Node2D\n  end\nend\n",
        )];

        let announcement = announcement("res://enemies/boss.rb", &sources).unwrap();

        assert_eq!(announcement.name, "Boss");
    }

    // @behavior RN-002
    #[test]
    fn a_node_script_inheriting_from_no_announced_file_is_announced_with_its_engine_class_as_base()
    {
        let announcement = announcement("res://enemy.rb", &[ENEMY]).unwrap();

        assert_eq!(announcement.base, "Node2D");
    }

    // @behavior RN-003
    #[test]
    fn a_node_script_extending_an_announced_class_is_announced_with_that_class_as_base() {
        let sources = [("res://boss.rb", "class Boss < Enemy\nend\n"), ENEMY];

        let announcement = announcement("res://boss.rb", &sources).unwrap();

        assert_eq!(announcement.base, "Enemy");
    }

    // @behavior RN-012
    #[test]
    fn a_file_under_the_template_directory_is_not_announced() {
        let path = "res://script_templates/node/hero.rb";
        let source =
            "module ScriptTemplates\nmodule Node\nclass Hero < Godot::Node\nend\nend\nend\n";

        let omission = announcement(path, &[(path, source)]).unwrap_err();

        assert_eq!(omission, Omission::InTemplateDirectory);
    }

    // @behavior RN-004
    #[test]
    fn a_file_under_a_test_directory_is_not_announced() {
        let sources = [("res://test/dummy.rb", "class Dummy < Godot::Node\nend\n")];

        let omission = announcement("res://test/dummy.rb", &sources).unwrap_err();

        assert_eq!(omission, Omission::InTestDirectory);
    }

    // @behavior RN-005
    #[test]
    fn a_library_file_is_not_announced() {
        let sources = [("res://save.rb", "class Save < Godot::Resource\nend\n")];

        let omission = announcement("res://save.rb", &sources).unwrap_err();

        assert_eq!(omission, Omission::NotNodeScript);
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

        let omission = announcement("res://enemies/boss.rb", &sources).unwrap_err();

        assert_eq!(
            omission,
            Omission::SharedName {
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

        let announcement = announcement("res://boss.rb", &sources).unwrap();

        assert_eq!(announcement.base, "Node2D");
    }

    // @behavior RN-008
    #[test]
    fn an_announcement_carries_whether_the_class_is_a_tool() {
        let sources = [("res://gizmo.rb", "class Gizmo < Godot::Node\n  tool\nend\n")];

        let announcement = announcement("res://gizmo.rb", &sources).unwrap();

        assert!(announcement.is_tool);
    }

    // @behavior RN-009
    #[test]
    fn an_announcement_carries_whether_the_class_is_abstract() {
        let sources = [(
            "res://enemy.rb",
            "class Enemy < Godot::Node\n  abstract\nend\n",
        )];

        let announcement = announcement("res://enemy.rb", &sources).unwrap();

        assert!(announcement.is_abstract);
    }

    // @behavior RN-010
    #[test]
    fn an_announcement_carries_the_classs_icon() {
        let sources = [(
            "res://enemy.rb",
            "class Enemy < Godot::Node\n  icon \"enemy.svg\"\nend\n",
        )];

        let announcement = announcement("res://enemy.rb", &sources).unwrap();

        assert_eq!(announcement.icon.as_deref(), Some("enemy.svg"));
    }

    // @behavior RN-011
    #[test]
    fn a_name_clash_that_came_back_is_warned_of_again() {
        let files = ["res://a/twin.rb".to_owned(), "res://b/twin.rb".to_owned()];
        let mut clashes = Clashes::new();
        clashes.note("Twin", &files);
        clashes.forget("res://a/twin.rb");

        let is_news = clashes.note("Twin", &files);

        assert!(is_news);
    }
}
