//! The game's Ruby files, as the game's realm is given them: every `.rb`
//! file Godot lists under `res://`, with the source Godot holds for each.

use godot::classes::{Os, ResourceLoader, Script};
use godot::obj::Singleton;

use crate::parser::Header;
use crate::realm::{Declared, Files};
use crate::settings;

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
    // fails where the realm reads its source to run it.
    fn declared(&self, path: &str) -> Declared {
        let Ok(source) = self.source(path) else {
            return Declared::default();
        };
        Declared {
            writes: Header::read(path, &source).writes().to_vec(),
        }
    }
}

// The directories the class index leaves out: an exported game's test
// directories. Tests run only on the editor's build, where every file is
// indexed so a test reaches any file by name.
fn left_out() -> Vec<String> {
    if Os::singleton().has_feature("template") {
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
