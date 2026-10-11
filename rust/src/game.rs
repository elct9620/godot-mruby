//! The game's Ruby files: every `.rb` file Godot lists under `res://`, with
//! the source Godot holds for each as the game's realm is given them, or as
//! each is on disk for what the editor asks while it scans.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use godot::classes::{ClassDb, Engine, FileAccess, Os, ResourceLoader, Script};
use godot::global::Error;
use godot::obj::Singleton;

use crate::ancestry;
use crate::header::Header;
use crate::realm::{ClassIndex, Declarations, Files, Parent, Roots};
use crate::settings;

const ROOT: &str = "res://";

/// The files the game ships under `res://`.
pub struct GameFiles;

impl Files for GameFiles {
    /// The game's files as last listed: a file runs and a script answers
    /// its ancestry once per file, and walking every directory for each
    /// would cost the game time in the square of its files.
    fn paths(&self) -> Vec<String> {
        LISTING.paths()
    }

    fn file_by_name(&self, scope: &[String], names: &[String]) -> Option<String> {
        LISTING.file_by_name(scope, names)
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
    fn source_by_path(&self, path: &str) -> Result<String, String> {
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
    fn declarations_by_path(&self, path: &str) -> Declarations {
        let Ok(source) = self.source_by_path(path) else {
            return Declarations::default();
        };
        let header = Header::from_source(path, &source, &self.roots());
        let parent = ancestry::trace_ancestry(path, &header, self)
            .ok()
            .filter(|ancestry| is_node_class(ancestry.engine_class()))
            .map(|ancestry| match ancestry.files().first() {
                Some((file, _)) => Parent::File(file.clone()),
                None => {
                    Parent::Constant(vec!["Godot".to_owned(), ancestry.engine_class().to_owned()])
                }
            });
        Declarations {
            writes: header.writes().to_vec(),
            parent,
            exports: header.export_names().map(str::to_owned).collect(),
            signals: header.signal_names().to_vec(),
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

    fn source_by_path(&self, path: &str) -> Result<String, String> {
        GameFiles.source_by_path(path)
    }

    fn declarations_by_path(&self, path: &str) -> Declarations {
        GameFiles.declarations_by_path(path)
    }
}

/// Whether this is the editor and the file at `path` sits under one of
/// `test_directories`, which the editor never runs.
pub fn is_left_out_in_editor(path: &str, test_directories: &[String]) -> bool {
    Engine::singleton().is_editor_hint() && settings::is_in_directory(path, test_directories)
}

/// A project's files as last listed, for what the editor asks while it
/// scans: listing walks every directory, which the editor would otherwise
/// wait on for each file it asks about. A file asked about joins the listing,
/// and a removed one leaves it once the project is listed again. The class
/// index of the listing is kept until the listing changes.
pub struct Listing<F: Files> {
    files: F,
    is_listed: fn(&str) -> bool,
    paths: Mutex<Option<Vec<String>>>,
    index: Mutex<Option<Arc<ClassIndex>>>,
}

impl<F: Files> Listing<F> {
    /// `files`, listed the first time a question needs them; `is_listed`
    /// says whether a file asked about belongs to the listing.
    pub const fn new(files: F, is_listed: fn(&str) -> bool) -> Self {
        Self {
            files,
            is_listed,
            paths: Mutex::new(None),
            index: Mutex::new(None),
        }
    }

    /// Takes the file at `path` into the listing, as the editor asks about
    /// it: it may have been added since the project was listed.
    pub fn take_in(&self, path: &str) {
        if !(self.is_listed)(path) {
            return;
        }
        if let Some(paths) = self.lock_paths().as_mut()
            && let Err(at) = paths.binary_search_by(|other| other.as_str().cmp(path))
        {
            paths.insert(at, path.to_owned());
            self.lock_index().take();
        }
    }

    /// Lists the project again, walking its directories outside the lock so
    /// no question waits on the walk.
    pub fn relist(&self) {
        let paths = self.files.paths();
        *self.lock_paths() = Some(paths);
        self.lock_index().take();
    }

    // The class index of the listing, built outside the lock once the
    // listing changed.
    fn index(&self) -> Arc<ClassIndex> {
        if let Some(index) = &*self.lock_index() {
            return Arc::clone(index);
        }
        let index = Arc::new(ClassIndex::with_files(self.paths(), self.roots()));
        Arc::clone(self.lock_index().get_or_insert(index))
    }

    fn lock_paths(&self) -> MutexGuard<'_, Option<Vec<String>>> {
        self.paths.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn lock_index(&self) -> MutexGuard<'_, Option<Arc<ClassIndex>>> {
        self.index.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<F: Files> Files for Listing<F> {
    fn paths(&self) -> Vec<String> {
        if let Some(paths) = &*self.lock_paths() {
            return paths.clone();
        }
        let paths = self.files.paths();
        self.lock_paths().get_or_insert(paths).clone()
    }

    fn roots(&self) -> Roots {
        self.files.roots()
    }

    fn source_by_path(&self, path: &str) -> Result<String, String> {
        self.files.source_by_path(path)
    }

    fn declarations_by_path(&self, path: &str) -> Declarations {
        self.files.declarations_by_path(path)
    }

    fn file_by_name(&self, scope: &[String], names: &[String]) -> Option<String> {
        self.index().file_by_name(scope, names)
    }
}

static LISTING: Listing<FilesOnDisk> = Listing::new(FilesOnDisk, is_listed);

/// The game's files on disk as last listed, for what the editor asks while
/// it holds the language.
pub fn listing() -> &'static Listing<FilesOnDisk> {
    &LISTING
}

/// Lists the game's files again, as the editor found the project's files
/// changed.
pub fn relist_files() {
    LISTING.relist();
}

// Whether `GameFiles` lists the file at `path`.
fn is_listed(path: &str) -> bool {
    path.ends_with(".rb") && !settings::is_in_directory(path, &left_out_directories())
}

/// The game's files as they are on disk, for answering what the editor asks
/// of a file as it scans the project. GDScript reads its own this way: the
/// editor asks while it holds the language, so a failed load, which Godot
/// prints, cannot happen here, and a file that cannot be read is no class.
pub struct FilesOnDisk;

impl Files for FilesOnDisk {
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

    fn roots(&self) -> Roots {
        GameFiles.roots()
    }

    fn source_by_path(&self, path: &str) -> Result<String, String> {
        let source = FileAccess::get_file_as_string(path);
        match FileAccess::get_open_error() {
            Error::OK => Ok(source.to_string()),
            error => Err(format!("{error:?}")),
        }
    }

    // A scan runs no file.
    fn declarations_by_path(&self, _path: &str) -> Declarations {
        Declarations::default()
    }
}

/// Whether the engine class `class` is a node class, which only a node
/// script's class extends.
pub fn is_node_class(class: &str) -> bool {
    ClassDb::singleton().is_parent_class(class, "Node")
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
