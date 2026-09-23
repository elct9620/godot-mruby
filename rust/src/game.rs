//! The game's Ruby files: every `.rb` file Godot lists under `res://`, with
//! the source Godot holds for each as the game's realm is given them, or as
//! each is on disk for what the editor asks while it scans.

use godot::classes::{FileAccess, Os, ResourceLoader, Script};
use godot::global::Error;
use godot::obj::Singleton;

use crate::header::Header;
use crate::realm::{Declared, Extends, Files};
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
        let left_out = left_out();
        let left_out: Vec<&str> = left_out
            .iter()
            .map(|dir| dir.trim_end_matches('/'))
            .collect();
        let mut paths = Vec::new();
        collect(ROOT, &left_out, &mut paths);
        paths.sort();
        paths
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
    fn declared(&self, path: &str) -> Declared {
        let Ok(source) = self.source(path) else {
            return Declared::default();
        };
        let header = Header::read(path, &source);
        let extends = ancestry::read(path, &header, self)
            .ok()
            .filter(|ancestry| bridge::is_node_class(ancestry.engine_class()))
            .map(|ancestry| match ancestry.files().first() {
                Some((file, _)) => Extends::File(file.clone()),
                None => {
                    Extends::Constant(vec!["Godot".to_owned(), ancestry.engine_class().to_owned()])
                }
            });
        Declared {
            writes: header.writes().to_vec(),
            extends,
        }
    }
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

    fn source(&self, path: &str) -> Result<String, String> {
        let source = FileAccess::get_file_as_string(path);
        match FileAccess::get_open_error() {
            Error::OK => Ok(source.to_string()),
            error => Err(format!("{error:?}")),
        }
    }

    // A scan runs no file.
    fn declared(&self, _path: &str) -> Declared {
        Declared::default()
    }
}

/// Whether this is an exported game: Godot runs one on an export template's
/// build, which carries the `template` feature.
pub fn is_exported() -> bool {
    Os::singleton().has_feature("template")
}

// The directories the class index leaves out: an exported game's test
// directories. Tests run only on the editor's build, where every file is
// indexed so a test reaches any file by name.
fn left_out() -> Vec<String> {
    if is_exported() {
        settings::test_directories()
    } else {
        Vec::new()
    }
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
