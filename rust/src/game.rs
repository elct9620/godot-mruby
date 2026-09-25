//! The game's Ruby files: every `.rb` file Godot lists under `res://`, with
//! the source Godot holds for each as the game's realm is given them, or as
//! each is on disk for what the editor asks while it scans.

use godot::classes::{Engine, FileAccess, Os, ResourceLoader, Script};
use godot::global::Error;
use godot::obj::Singleton;

use crate::header::Header;
use crate::realm::{Declarations, Extends, Files, Roots};
use crate::settings;
use crate::{ancestry, bridge};

const ROOT: &str = "res://";

/// The files the game ships under `res://`.
pub struct GameFiles;

impl Files for GameFiles {
    /// Every `.rb` file under `res://` outside the directories the game
    /// leaves out, as Godot lists its resources: by the names they had before
    /// an export remapped them. Sorted, since Godot promises no order and the
    /// first directory to spell a namespace names it.
    fn paths(&self) -> Vec<String> {
        let left_out = left_out_directories();
        let left_out: Vec<&str> = left_out
            .iter()
            .map(|dir| dir.trim_end_matches('/'))
            .collect();
        let mut paths = Vec::new();
        collect(ROOT, &left_out, &mut paths);
        paths.sort();
        paths
    }

    // A test directory is a root directory whether the project lists it as
    // one or not.
    fn roots(&self) -> Roots {
        Roots::new(
            settings::root_directories()
                .into_iter()
                .chain(settings::test_directories()),
        )
    }

    // Through `ResourceLoader`, so an exported game follows its remaps.
    fn source(&self, path: &str) -> Result<String, String> {
        ResourceLoader::singleton()
            .load_ex(path)
            .type_hint("Script")
            .done()
            .and_then(|resource| resource.try_cast::<Script>().ok())
            .map(|script| script.get_source_code().to_string())
            .ok_or_else(|| "Godot did not load it as a script".to_owned())
    }

    // Read from the source, so a file that cannot be read declares nothing and
    // fails where the realm reads its source to run it. Only a node script's
    // class is held to its superclass: Godot is told its ancestry before it
    // runs, and a library file's is Ruby's to look up.
    fn declarations(&self, path: &str) -> Declarations {
        let Ok(source) = self.source(path) else {
            return Declarations::default();
        };
        let header = Header::from_source(path, &source, &self.roots());
        let extends = ancestry::ancestry_of(path, &header, self)
            .ok()
            .filter(|ancestry| bridge::is_node_class(ancestry.engine_class()))
            .map(|ancestry| match ancestry.files().first() {
                Some((file, _)) => Extends::File(file.clone()),
                None => {
                    Extends::Constant(vec!["Godot".to_owned(), ancestry.engine_class().to_owned()])
                }
            });
        Declarations {
            writes: header.writes().to_vec(),
            extends,
            exports: header.export_names().map(str::to_owned).collect(),
        }
    }
}

/// The game's files as its realm is given them: the editor leaves the test
/// directories out as an exported game does, since tests run only in a game
/// it plays, so no file of theirs runs there, by path or by name.
pub struct RealmFiles;

impl Files for RealmFiles {
    fn paths(&self) -> Vec<String> {
        let test_directories = settings::test_directories();
        GameFiles
            .paths()
            .into_iter()
            .filter(|path| !is_left_out_in_editor(path, &test_directories))
            .collect()
    }

    fn roots(&self) -> Roots {
        GameFiles.roots()
    }

    fn source(&self, path: &str) -> Result<String, String> {
        GameFiles.source(path)
    }

    fn declarations(&self, path: &str) -> Declarations {
        GameFiles.declarations(path)
    }
}

/// Whether this is the editor and the file at `path` sits under one of
/// `test_directories`, which the editor never runs.
pub fn is_left_out_in_editor(path: &str, test_directories: &[String]) -> bool {
    Engine::singleton().is_editor_hint() && settings::is_in_directory(path, test_directories)
}

/// The game's files as they are on disk, for answering what the editor asks
/// of a file as it scans the project. GDScript reads its own this way: the
/// editor asks while it holds the language, so a failed load, which Godot
/// prints, cannot happen here, and a file that cannot be read is no class.
pub struct FilesOnDisk;

impl Files for FilesOnDisk {
    fn paths(&self) -> Vec<String> {
        GameFiles.paths()
    }

    fn roots(&self) -> Roots {
        GameFiles.roots()
    }

    fn source(&self, path: &str) -> Result<String, String> {
        let source = FileAccess::get_file_as_string(path);
        match FileAccess::get_open_error() {
            Error::OK => Ok(source.to_string()),
            error => Err(format!("{error:?}")),
        }
    }

    // A scan runs no file.
    fn declarations(&self, _path: &str) -> Declarations {
        Declarations::default()
    }
}

/// Whether this is an exported game: Godot runs one on an export template's
/// build, which carries the `template` feature.
pub fn is_exported() -> bool {
    Os::singleton().has_feature("template")
}

// The directories the class index leaves out: the project's script
// templates, and an exported game's test directories. Tests run only on the
// editor's build, where every other file is indexed so a test reaches any
// file by name.
fn left_out_directories() -> Vec<String> {
    let mut left_out = vec![settings::template_directory()];
    if is_exported() {
        left_out.extend(settings::test_directories());
    }
    left_out
}

fn collect(directory: &str, left_out: &[&str], paths: &mut Vec<String>) {
    for entry in ResourceLoader::singleton()
        .list_directory(directory)
        .as_slice()
    {
        let entry = entry.to_string();
        if let Some(child) = entry.strip_suffix('/') {
            let child = format!("{directory}{child}");
            if !left_out.contains(&child.as_str()) {
                collect(&format!("{child}/"), left_out, paths);
            }
        } else if entry.ends_with(".rb") {
            paths.push(format!("{directory}{entry}"));
        }
    }
}
