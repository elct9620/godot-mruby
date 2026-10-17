//! How a script is announced to the editor, which lists it by name as
//! it lists a GDScript's `class_name`: read from headers and ancestries, so
//! the editor scans a project without running any of it.

use std::collections::BTreeSet;

use crate::ancestry::{self, Ancestry};
use crate::game;
use crate::header::Header;
use crate::realm::{self, Files};
use crate::settings;

/// What the editor lists a script by.
#[derive(Debug, PartialEq, Eq)]
pub struct Announcement {
    /// The class's own name, without its namespaces, since the editor's list
    /// of classes is flat.
    pub name: String,
    /// The nearest announced class the class inherits from, or else its
    /// engine class.
    pub base: String,
    /// The icon's path from the project's root, as the header carries it.
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
    NotScript,
    /// Scripts in other files share the name, so no one of them is
    /// listed by it, whatever order the editor scans them in.
    SharedName { name: String, others: Vec<String> },
}

/// The project a script is announced in: its files, its test directories
/// and template directory, and which names are engine classes.
pub struct Project<'a, F: Files> {
    files: &'a F,
    paths: Vec<String>,
    test_directories: Vec<String>,
    template_directory: String,
    is_engine_class: &'a dyn Fn(&str) -> bool,
}

impl<'a, F: Files> Project<'a, F> {
    pub fn new(
        files: &'a F,
        test_directories: Vec<String>,
        template_directory: String,
        is_engine_class: &'a dyn Fn(&str) -> bool,
    ) -> Self {
        Self {
            files,
            paths: files.paths(),
            test_directories,
            template_directory,
            is_engine_class,
        }
    }

    /// The announcement of the file at `path`, or why it has none.
    pub fn announcement_by_path(&self, path: &str) -> Result<Announcement, Omission> {
        if self.is_in_test_directory(path) {
            return Err(Omission::InTestDirectory);
        }
        if self.is_in_template_directory(path) {
            return Err(Omission::InTemplateDirectory);
        }
        let (header, ancestry) = self.read_script(path).ok_or(Omission::NotScript)?;
        let others = self.find_name_sharers(path);
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
                !self.is_in_test_directory(ancestor) && self.find_name_sharers(ancestor).is_empty()
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

    // The header and ancestry of the file at `path`, when it is a script.
    fn read_script(&self, path: &str) -> Option<(Header, Ancestry)> {
        let header = Header::from_source(
            path,
            &self.files.source_by_path(path).ok()?,
            &self.files.roots(),
        );
        let ancestry = ancestry::trace_ancestry(path, &header, self.files).ok()?;
        (self.is_engine_class)(ancestry.engine_class()).then_some((header, ancestry))
    }

    // The other game files whose scripts share the name of the file at
    // `path`: only a file whose last segment spells the same name can.
    fn find_name_sharers(&self, path: &str) -> Vec<String> {
        let name = read_file_name(path);
        self.paths
            .iter()
            .filter(|other| other.as_str() != path && read_file_name(other) == name)
            .filter(|other| !self.is_in_test_directory(other) && self.read_script(other).is_some())
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

/// The announcement of the file at `path` in the game's project, read from
/// disk as the editor scans it.
pub fn announcement_by_path(path: &str) -> Result<Announcement, Omission> {
    let listing = game::listing();
    listing.take_in(path);
    Project::new(
        listing,
        settings::test_directories(),
        settings::template_directory(),
        &game::is_engine_class,
    )
    .announcement_by_path(path)
}

/// The names scripts share, each with the files sharing it, already
/// warned of: the editor scans the project again and again and asks of every
/// file, and each shared name is warned of once while it lasts.
pub struct NameClashes(BTreeSet<(String, Vec<String>)>);

impl NameClashes {
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    /// Notes that the scripts in `files` share `name`, answering whether
    /// that is news.
    pub fn note(&mut self, name: &str, files: &[String]) -> bool {
        self.0.insert((name.to_owned(), files.to_vec()))
    }

    /// Forgets the shared names the file at `path` was in, once it is
    /// announced or no script: a shared name coming back is news again.
    pub fn forget(&mut self, path: &str) {
        self.0
            .retain(|(_, files)| !files.iter().any(|file| file == path));
    }
}

/// The warning that scripts in `others` share `name` with the file at
/// `path`.
pub fn describe_shared_name(path: &str, name: &str, others: &[String]) -> String {
    format!(
        "{path} and {} define scripts named {name}, so none is listed by that name",
        others.join(" and ")
    )
}

/// The files at `paths` whose announcement a file in `changes` may have
/// changed by arriving or leaving: those sharing its file name, which the
/// editor does not ask about again by itself. Files under
/// `test_directories` are never announced, so they neither change one nor
/// are told.
pub fn collect_name_sharers(
    paths: &[String],
    changes: &[String],
    test_directories: &[String],
) -> Vec<String> {
    let is_announceable = |path: &&String| !settings::is_in_directory(path, test_directories);
    let names: BTreeSet<String> = changes
        .iter()
        .filter(is_announceable)
        .map(|path| read_file_name(path))
        .collect();
    if names.is_empty() {
        return Vec::new();
    }
    let changes: BTreeSet<&String> = changes.iter().collect();
    paths
        .iter()
        .filter(is_announceable)
        .filter(|path| !changes.contains(path) && names.contains(&read_file_name(path)))
        .cloned()
        .collect()
}

// The last segment of the file at `path`, as the class index matches it.
fn read_file_name(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    realm::normalize(name.trim_end_matches(".rb"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use super::{Announcement, NameClashes, Omission, Project, collect_name_sharers};
    use crate::ancestry::tests::Sources;
    use crate::game::Listing;
    use crate::realm::{Declarations, Files, Roots};

    fn announce_sources(
        path: &str,
        sources: &[(&'static str, &'static str)],
    ) -> Result<Announcement, Omission> {
        let files = Sources(sources.iter().copied().collect());
        let test_directories = vec!["res://test".to_owned()];
        let is_engine_class = |_: &str| true;
        let template_directory = "res://script_templates".to_owned();
        Project::new(
            &files,
            test_directories,
            template_directory,
            &is_engine_class,
        )
        .announcement_by_path(path)
    }

    const ENEMY: (&str, &str) = ("res://enemy.rb", "class Enemy < Godot::Node2D\nend\n");

    // @behavior RN-001
    #[test]
    fn a_node_script_is_announced_by_its_classs_name_without_its_namespaces() {
        let sources = [(
            "res://enemies/boss.rb",
            "module Enemies\n  class Boss < Godot::Node2D\n  end\nend\n",
        )];

        let announcement = announce_sources("res://enemies/boss.rb", &sources).unwrap();

        assert_eq!(announcement.name, "Boss");
    }

    // @behavior RN-002
    #[test]
    fn a_node_script_inheriting_from_no_announced_file_is_announced_with_its_engine_class_as_base()
    {
        let announcement = announce_sources("res://enemy.rb", &[ENEMY]).unwrap();

        assert_eq!(announcement.base, "Node2D");
    }

    // @behavior RN-003
    #[test]
    fn a_node_script_extending_an_announced_class_is_announced_with_that_class_as_base() {
        let sources = [("res://boss.rb", "class Boss < Enemy\nend\n"), ENEMY];

        let announcement = announce_sources("res://boss.rb", &sources).unwrap();

        assert_eq!(announcement.base, "Enemy");
    }

    // @behavior RN-012
    #[test]
    fn a_file_under_the_template_directory_is_not_announced() {
        let path = "res://script_templates/node/hero.rb";
        let source =
            "module ScriptTemplates\nmodule Node\nclass Hero < Godot::Node\nend\nend\nend\n";

        let omission = announce_sources(path, &[(path, source)]).unwrap_err();

        assert_eq!(omission, Omission::InTemplateDirectory);
    }

    // @behavior RN-004
    #[test]
    fn a_file_under_a_test_directory_is_not_announced() {
        let sources = [("res://test/dummy.rb", "class Dummy < Godot::Node\nend\n")];

        let omission = announce_sources("res://test/dummy.rb", &sources).unwrap_err();

        assert_eq!(omission, Omission::InTestDirectory);
    }

    // @behavior RN-005
    #[test]
    fn a_library_file_is_not_announced() {
        let sources = [("res://save.rb", "class Save\nend\n")];

        let omission = announce_sources("res://save.rb", &sources).unwrap_err();

        assert_eq!(omission, Omission::NotScript);
    }

    // @behavior RN-019
    #[test]
    fn an_object_script_is_announced_with_its_engine_class_as_base() {
        let sources = [("res://save.rb", "class Save < Godot::Resource\nend\n")];

        let announcement = announce_sources("res://save.rb", &sources).unwrap();

        assert_eq!(
            (announcement.name.as_str(), announcement.base.as_str()),
            ("Save", "Resource")
        );
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

        let omission = announce_sources("res://enemies/boss.rb", &sources).unwrap_err();

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

        let announcement = announce_sources("res://boss.rb", &sources).unwrap();

        assert_eq!(announcement.base, "Node2D");
    }

    // @behavior RN-008
    #[test]
    fn an_announcement_carries_whether_the_class_is_a_tool() {
        let sources = [("res://gizmo.rb", "class Gizmo < Godot::Node\n  tool\nend\n")];

        let announcement = announce_sources("res://gizmo.rb", &sources).unwrap();

        assert!(announcement.is_tool);
    }

    // @behavior RN-009
    #[test]
    fn an_announcement_carries_whether_the_class_is_abstract() {
        let sources = [(
            "res://enemy.rb",
            "class Enemy < Godot::Node\n  abstract\nend\n",
        )];

        let announcement = announce_sources("res://enemy.rb", &sources).unwrap();

        assert!(announcement.is_abstract);
    }

    // @behavior RN-010
    #[test]
    fn an_announcement_carries_the_classs_icon() {
        let sources = [(
            "res://enemy.rb",
            "class Enemy < Godot::Node\n  icon \"enemy.svg\"\nend\n",
        )];

        let announcement = announce_sources("res://enemy.rb", &sources).unwrap();

        assert_eq!(announcement.icon.as_deref(), Some("res://enemy.svg"));
    }

    // @behavior RN-011
    #[test]
    fn a_shared_name_that_came_back_is_warned_of_again() {
        let files = ["res://a/twin.rb".to_owned(), "res://b/twin.rb".to_owned()];
        let mut shared_names = NameClashes::new();
        shared_names.note("Twin", &files);
        shared_names.forget("res://a/twin.rb");

        let is_news = shared_names.note("Twin", &files);

        assert!(is_news);
    }

    /// Files on a disk the tests change, under `res://` and `res://src`.
    struct Disk(Mutex<BTreeMap<&'static str, &'static str>>);

    impl Disk {
        fn new(sources: &[(&'static str, &'static str)]) -> Self {
            Self(Mutex::new(sources.iter().copied().collect()))
        }

        fn write(&self, path: &'static str, source: &'static str) {
            self.0.lock().unwrap().insert(path, source);
        }

        fn remove(&self, path: &str) {
            self.0.lock().unwrap().remove(path);
        }
    }

    impl Files for &Disk {
        fn paths(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .keys()
                .map(|path| (*path).to_owned())
                .collect()
        }

        fn roots(&self) -> Roots {
            Roots::new(["res://src".to_owned()])
        }

        fn source_by_path(&self, path: &str) -> Result<String, String> {
            let sources = self.0.lock().unwrap();
            sources
                .get(path)
                .map(|source| (*source).to_owned())
                .ok_or_else(|| "no such file".to_owned())
        }

        fn declarations_by_path(&self, _path: &str) -> Declarations {
            Declarations::default()
        }
    }

    fn announce<F: Files>(listing: &Listing<F>, path: &str) -> Result<Announcement, Omission> {
        let is_engine_class = |_: &str| true;
        listing.take_in(path);
        Project::new(listing, Vec::new(), String::new(), &is_engine_class)
            .announcement_by_path(path)
    }

    // @behavior RN-016
    #[test]
    fn the_files_sharing_a_changed_files_name_outside_test_directories_are_named() {
        let paths = [
            "res://boss.rb",
            "res://enemy.rb",
            "res://levels/enemy.rb",
            "res://test/enemy.rb",
        ]
        .map(str::to_owned);
        let changes = ["res://levels/enemy.rb".to_owned()];

        let sharers = collect_name_sharers(&paths, &changes, &["res://test/".to_owned()]);

        assert_eq!(sharers, ["res://enemy.rb"]);
    }

    // @behavior RN-018
    #[test]
    fn a_changed_file_under_a_test_directory_names_no_file_to_announce_again() {
        let paths = ["res://enemy.rb", "res://test/enemy.rb"].map(str::to_owned);
        let changes = ["res://test/enemy.rb".to_owned()];

        let sharers = collect_name_sharers(&paths, &changes, &["res://test/".to_owned()]);

        assert!(sharers.is_empty());
    }

    // @behavior RN-013
    #[test]
    fn a_file_added_since_the_project_was_listed_is_announced() {
        let disk = Disk::new(&[ENEMY]);
        let listing = Listing::new(&disk, |_| true);
        announce(&listing, "res://enemy.rb").unwrap();
        disk.write(
            "res://levels/enemy.rb",
            "module Levels\n  class Enemy < Godot::Node\n  end\nend\n",
        );

        let added = announce(&listing, "res://levels/enemy.rb");
        let earlier = announce(&listing, "res://enemy.rb");

        assert!(matches!(added, Err(Omission::SharedName { .. })));
        assert!(matches!(earlier, Err(Omission::SharedName { .. })));
    }

    // @behavior RN-013
    #[test]
    fn a_file_added_since_the_project_was_listed_is_found_as_a_superclass() {
        let disk = Disk::new(&[("res://boss.rb", "class Boss < Enemy\nend\n")]);
        let listing = Listing::new(&disk, |_| true);
        announce(&listing, "res://boss.rb").unwrap_err();
        disk.write(ENEMY.0, ENEMY.1);
        announce(&listing, ENEMY.0).unwrap();

        let announcement = announce(&listing, "res://boss.rb").unwrap();

        assert_eq!(announcement.base, "Enemy");
    }

    // @behavior RN-014
    #[test]
    fn a_removed_file_is_forgotten_once_the_project_is_listed_again() {
        let disk = Disk::new(&[
            ("res://boss.rb", "class Boss < Enemy\nend\n"),
            ENEMY,
            ("res://src/enemy.rb", "class Enemy < Godot::Node2D\nend\n"),
        ]);
        let listing = Listing::new(&disk, |_| true);
        announce(&listing, "res://boss.rb").unwrap_err();
        disk.remove("res://src/enemy.rb");

        listing.relist();
        let announcement = announce(&listing, "res://boss.rb").unwrap();

        assert_eq!(announcement.base, "Enemy");
    }
}
