//! The game's Ruby files: every `.rb` file Godot lists under `res://`, with
//! the source Godot holds for each as the game's realm is given them, or as
//! each is on disk for what the editor asks while it scans.

use std::borrow::Cow;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use godot::classes::{ClassDb, Engine, FileAccess, Os, ResourceLoader, Script};
use godot::global::Error;
use godot::obj::Singleton;

use crate::ancestry;
use crate::bridge;
use crate::header::Header;
#[cfg(feature = "editor")]
use crate::realm::{self, Definition};
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
    // fails where the realm reads its source to run it. Only a script's
    // class is held to its superclass: Godot is told its ancestry before it
    // runs, and a library file's is Ruby's to look up.
    fn declarations_by_path(&self, path: &str) -> Declarations {
        let Ok(source) = self.source_by_path(path) else {
            return Declarations::default();
        };
        let header = Header::from_source(path, &source, &self.roots());
        let parent = ancestry::trace_ancestry(path, &header, self)
            .ok()
            .filter(|ancestry| is_engine_class(ancestry.engine_class()))
            .map(|ancestry| match ancestry.files().first() {
                Some((file, _)) => Parent::File(file.clone()),
                None => {
                    Parent::Constant(vec!["Godot".to_owned(), ancestry.engine_class().to_owned()])
                }
            });
        Declarations {
            constant_paths: header.constant_paths().to_vec(),
            parent,
            exports: header.export_names().map(str::to_owned).collect(),
            signals: header.signal_names().to_vec(),
            method_calls: header.method_calls().to_vec(),
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

    // A project file's assignments to a member of a member write the value
    // types they change back, as GDScript's do.
    fn rewrite_source<'a>(&self, source: &'a str) -> Cow<'a, str> {
        bridge::rewrite_member_writes(source)
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
/// index of the listing is kept until the listing changes, and the files
/// that joined it are kept until they are taken, so what comes after a
/// listing costs as many files as joined, not as the project holds.
pub struct Listing<F: Files> {
    files: F,
    is_listed: fn(&str) -> bool,
    paths: Mutex<Option<Vec<String>>>,
    index: Mutex<Option<Arc<ClassIndex>>>,
    arrivals: Mutex<Vec<String>>,
    #[cfg(any(test, feature = "editor"))]
    departures: Mutex<Vec<String>>,
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
            arrivals: Mutex::new(Vec::new()),
            #[cfg(any(test, feature = "editor"))]
            departures: Mutex::new(Vec::new()),
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
            self.lock_arrivals().push(path.to_owned());
        }
    }

    #[cfg(any(test, feature = "editor"))]
    /// Lists the project again, walking its directories outside the lock so
    /// no question waits on the walk.
    pub fn relist(&self) {
        let paths = self.files.paths();
        let mut listing = self.lock_paths();
        if let Some(listed) = listing.as_ref() {
            self.lock_arrivals().extend(collect_missing(&paths, listed));
            self.lock_departures()
                .extend(collect_missing(listed, &paths));
        }
        *listing = Some(paths);
        drop(listing);
        self.lock_index().take();
    }

    #[cfg(any(test, feature = "editor"))]
    /// The files that joined the listing since this was last asked, each
    /// once, as the editor asked about it or a listing found it.
    pub fn take_arrivals(&self) -> Vec<String> {
        std::mem::take(&mut *self.lock_arrivals())
    }

    #[cfg(any(test, feature = "editor"))]
    /// The files that left the listing since this was last asked, each once,
    /// as a listing found them gone.
    pub fn take_departures(&self) -> Vec<String> {
        std::mem::take(&mut *self.lock_departures())
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

    fn lock_arrivals(&self) -> MutexGuard<'_, Vec<String>> {
        self.arrivals.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[cfg(any(test, feature = "editor"))]
    fn lock_departures(&self) -> MutexGuard<'_, Vec<String>> {
        self.departures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(any(test, feature = "editor"))]
// The paths in `paths` that `others` lacks, walking both sorted lists once.
fn collect_missing(paths: &[String], others: &[String]) -> Vec<String> {
    let mut others = others.iter().peekable();
    let mut missing = Vec::new();
    for path in paths {
        while others.next_if(|other| *other < path).is_some() {}
        if others.next_if(|other| *other == path).is_none() {
            missing.push(path.clone());
        }
    }
    missing
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

    #[cfg(feature = "editor")]
    fn collect_definitions_around(&self, path: &str) -> Vec<Definition> {
        self.index().collect_definitions_around(path)
    }
}

static LISTING: Listing<FilesOnDisk> = Listing::new(FilesOnDisk, is_listed);

/// The game's files on disk as last listed, for what the editor asks while
/// it holds the language.
pub fn listing() -> &'static Listing<FilesOnDisk> {
    &LISTING
}

#[cfg(feature = "editor")]
/// Lists the game's files again, as the editor found the project's files
/// changed, and gives the game's realm those added since it opened.
/// Answers the files that were added or removed.
pub fn relist_files() -> Vec<String> {
    LISTING.relist();
    let arrivals = LISTING.take_arrivals();
    let test_directories = settings::test_directories();
    realm::take_in_files(
        arrivals
            .iter()
            .filter(|path| !is_left_out_in_editor(path, &test_directories))
            .cloned()
            .collect(),
    );
    arrivals
        .into_iter()
        .chain(LISTING.take_departures())
        .collect()
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
        collect_paths(ROOT, &left_out, &mut paths);
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

/// Whether the engine has a class named `class`, which a script's class may
/// extend, as a GDScript may extend any of them.
pub fn is_engine_class(class: &str) -> bool {
    ClassDb::singleton().class_exists(class)
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

fn collect_paths(directory: &str, left_out: &[&str], paths: &mut Vec<String>) {
    for entry in ResourceLoader::singleton()
        .list_directory(directory)
        .as_slice()
    {
        let entry = entry.to_string();
        if let Some(child) = entry.strip_suffix('/') {
            let child = format!("{directory}{child}");
            if !left_out.contains(&child.as_str()) {
                collect_paths(&format!("{child}/"), left_out, paths);
            }
        } else if entry.ends_with(".rb") {
            paths.push(format!("{directory}{entry}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Files at sorted paths a test changes, as a project's files on disk.
    struct Folder(Mutex<Vec<&'static str>>);

    impl Folder {
        fn add(&self, path: &'static str) {
            let mut paths = self.0.lock().unwrap();
            paths.push(path);
            paths.sort_unstable();
        }
    }

    impl Files for &Folder {
        fn paths(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .map(|path| (*path).to_owned())
                .collect()
        }

        fn source_by_path(&self, _path: &str) -> Result<String, String> {
            Ok(String::new())
        }

        fn declarations_by_path(&self, _path: &str) -> Declarations {
            Declarations::default()
        }
    }

    // A listing of `folder` that has listed it once.
    fn first_listing(folder: &Folder) -> Listing<&Folder> {
        let listing = Listing::new(folder, |_| true);
        listing.paths();
        listing
    }

    // @behavior RL-041
    #[test]
    fn a_file_a_listing_finds_arrives_once() {
        let folder = Folder(Mutex::new(vec!["res://a.rb", "res://c.rb"]));
        let listing = first_listing(&folder);
        folder.add("res://b.rb");
        listing.relist();
        listing.relist();

        let arrivals = listing.take_arrivals();

        assert_eq!(arrivals, ["res://b.rb"]);
    }

    // @behavior RN-015
    #[test]
    fn a_file_a_listing_finds_gone_departs_once() {
        let folder = Folder(Mutex::new(vec!["res://a.rb", "res://b.rb"]));
        let listing = first_listing(&folder);
        folder
            .0
            .lock()
            .unwrap()
            .retain(|path| *path != "res://a.rb");
        listing.relist();
        listing.relist();

        let departures = listing.take_departures();

        assert_eq!(departures, ["res://a.rb"]);
    }

    // @behavior RL-042
    #[test]
    fn a_file_the_editor_asked_about_arrives_once() {
        let folder = Folder(Mutex::new(vec!["res://a.rb"]));
        let listing = first_listing(&folder);
        folder.add("res://b.rb");
        listing.take_in("res://b.rb");
        listing.relist();

        let arrivals = listing.take_arrivals();

        assert_eq!(arrivals, ["res://b.rb"]);
    }
}
