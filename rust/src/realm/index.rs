use std::collections::{BTreeMap, BTreeSet};

use godot::classes::DirAccess;

use crate::log::Location;
use crate::warn;

const ROOT: &str = "res://";

/// A realm's map from constant paths to the files named after them, by
/// Zeitwerk's rules: `res://` is the root, every directory a namespace, and
/// a file names the constant its path spells. Each segment is matched without
/// underscores or case, so `http_client.rb` may define `HttpClient` or
/// `HTTPClient`. A name the index cannot hold is warned about as it arrives.
#[derive(Default)]
pub struct ClassIndex {
    files: BTreeMap<Key, String>,
    namespaces: BTreeMap<Key, Namespace>,
    refused: BTreeSet<Key>,
}

/// A constant path as the index matches it: one segment per namespace, each
/// without underscores and in lower case.
pub type Key = Vec<String>;

/// What a constant path names in the index.
pub enum Named {
    /// The file defining it.
    File(String),
    /// A directory with no file of its own name, which is an empty module.
    Namespace(Namespace),
}

#[derive(Clone)]
pub struct Namespace {
    /// The directory, as `res://ui/`.
    pub directory: String,
    /// The module's name as Zeitwerk camelizes the directory's: `Ui`.
    pub name: String,
}

impl ClassIndex {
    /// Takes in the files at `paths`, refusing a name two files spell and a
    /// name `defined` says the realm already has.
    pub fn add(
        &mut self,
        paths: impl IntoIterator<Item = String>,
        defined: impl Fn(&[String]) -> bool,
    ) {
        let mut added = BTreeSet::new();
        for path in paths {
            let key = key_of(&path);
            self.add_namespaces(&path);
            if self.refused.contains(&key) {
                continue;
            }
            if let Some(other) = self.files.remove(&key) {
                warn!(at: &at(&path), "{other} and {path} both name {}, so neither loads by name", name_of(&other));
                self.refused.insert(key);
                continue;
            }
            if defined(&key) {
                warn!(at: &at(&path), "{path} names {}, which the realm already has, so it never loads by name", name_of(&path));
                self.refused.insert(key);
                continue;
            }
            self.files.insert(key.clone(), path);
            added.insert(key);
        }
        self.warn_of_shadows(&added);
    }

    /// What `key` names, if anything: a file is what a namespace's own file
    /// is too, so it answers before the directory.
    pub fn named(&self, key: &[String]) -> Option<Named> {
        match self.files.get(key) {
            Some(path) => Some(Named::File(path.clone())),
            None => self.namespaces.get(key).cloned().map(Named::Namespace),
        }
    }

    /// Whether the file at `path` is one the index names.
    pub fn names(&self, path: &str) -> bool {
        self.files
            .get(&key_of(path))
            .is_some_and(|named| named == path)
    }

    // Every directory a file sits in is a namespace, named by the first
    // directory to spell it.
    fn add_namespaces(&mut self, path: &str) {
        let segments = segments(path);
        for depth in 1..segments.len() {
            let key = segments[..depth]
                .iter()
                .map(|segment| normalize(segment))
                .collect();
            self.namespaces.entry(key).or_insert_with(|| Namespace {
                directory: format!("{ROOT}{}/", segments[..depth].join("/")),
                name: camelize(segments[depth - 1]),
            });
        }
    }

    // Ruby finds an outer constant before asking for an inner one of the same
    // name, so once the outer file has loaded the inner one never does. Only
    // files sharing a last segment can hide one another, and a pair already
    // warned about has no file among `added`.
    fn warn_of_shadows(&self, added: &BTreeSet<Key>) {
        let mut by_name: BTreeMap<&str, Vec<(&Key, &String)>> = BTreeMap::new();
        for (key, path) in &self.files {
            if let Some(name) = key.last() {
                by_name.entry(name).or_default().push((key, path));
            }
        }
        for files in by_name.values() {
            for &(inner, inner_path) in files {
                for &(outer, outer_path) in files {
                    let new = added.contains(inner) || added.contains(outer);
                    if new && hides(outer, inner) {
                        warn!(
                            at: &at(inner_path),
                            "{outer_path} names {}, which hides {} from Ruby inside {} once it has loaded",
                            name_of(outer_path),
                            name_of(inner_path),
                            namespace_of(inner_path)
                        );
                    }
                }
            }
        }
    }
}

// Whether `outer` shares `inner`'s last segment from a namespace `inner`'s
// namespace sits inside.
fn hides(outer: &Key, inner: &Key) -> bool {
    outer.len() < inner.len()
        && outer.last() == inner.last()
        && inner.starts_with(&outer[..outer.len() - 1])
}

/// Every `.rb` file under `res://` outside `excluded`, the directories the
/// game's index leaves out.
pub fn game_files(excluded: &[String]) -> Vec<String> {
    let excluded: Vec<&str> = excluded
        .iter()
        .map(|dir| dir.trim_end_matches('/'))
        .collect();
    let mut paths = Vec::new();
    collect(ROOT, &excluded, &mut paths);
    paths
}

fn collect(directory: &str, excluded: &[&str], paths: &mut Vec<String>) {
    for file in DirAccess::get_files_at(directory).as_slice() {
        let file = file.to_string();
        if file.ends_with(".rb") {
            paths.push(format!("{directory}{file}"));
        }
    }
    for child in DirAccess::get_directories_at(directory).as_slice() {
        let child = format!("{directory}{child}");
        if !excluded.contains(&child.as_str()) {
            collect(&format!("{child}/"), excluded, paths);
        }
    }
}

fn segments(path: &str) -> Vec<&str> {
    path.trim_start_matches(ROOT)
        .trim_end_matches(".rb")
        .split('/')
        .collect()
}

/// The constant path the file at `path` spells, as the index matches it.
pub fn key_of(path: &str) -> Key {
    segments(path)
        .iter()
        .map(|segment| normalize(segment))
        .collect()
}

/// A segment as the index matches it.
pub fn normalize(segment: &str) -> String {
    segment
        .chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

// The constant a path spells as Zeitwerk camelizes it: `http_client` is
// `HttpClient`.
fn name_of(path: &str) -> String {
    segments(path)
        .iter()
        .map(|segment| camelize(segment))
        .collect::<Vec<_>>()
        .join("::")
}

fn namespace_of(path: &str) -> String {
    let name = name_of(path);
    name.rsplit_once("::")
        .map_or(String::from("Object"), |(namespace, _)| {
            namespace.to_owned()
        })
}

fn camelize(segment: &str) -> String {
    segment
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or(String::new(), |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}

// A warning about a file as a whole points at its first line.
fn at(path: &str) -> Location {
    Location {
        file: path.to_owned(),
        line: 1,
    }
}
