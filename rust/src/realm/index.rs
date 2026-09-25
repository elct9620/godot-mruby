use std::collections::{BTreeMap, BTreeSet};

use super::{Level, Location, Log};

const ROOT: &str = "res://";

/// A realm's map from constant paths to the files named after them, by
/// Zeitwerk's rules: every directory below a root directory is a namespace,
/// and a file names the constant its path spells from the nearest root
/// directory it sits under. Each segment is matched without underscores or
/// case, so `http_client.rb` may define `HttpClient` or `HTTPClient`. A name
/// the index cannot hold is warned about as it arrives.
#[derive(Default)]
pub struct ClassIndex {
    roots: Roots,
    files: BTreeMap<Key, String>,
    namespaces: BTreeMap<Key, Namespace>,
    refusals: BTreeSet<Key>,
}

/// The root directories files are named from: `res://`, and the directories
/// inside it whose files are named from the top level too, each of which is
/// no namespace of the one it sits in.
#[derive(Clone, Debug, Default)]
pub struct Roots(Vec<String>);

impl Roots {
    /// The root directories `directories` names besides `res://`, each written
    /// with or without its trailing slash; `res://` itself and a directory
    /// outside it add nothing.
    pub fn new(directories: impl IntoIterator<Item = String>) -> Self {
        Self(
            directories
                .into_iter()
                .map(|directory| format!("{}/", directory.trim_end_matches('/')))
                .filter(|directory| directory.starts_with(ROOT) && directory.len() > ROOT.len())
                .collect(),
        )
    }

    /// The constant path the file at `path` spells, as the index matches it.
    pub fn key_of(&self, path: &str) -> Key {
        self.segments(path)
            .1
            .iter()
            .map(|segment| normalize(segment))
            .collect()
    }

    // The nearest root directory `path` sits under, and the segments of its
    // path from there; a path outside `res://`, such as a script's that has
    // no file, has none.
    fn segments<'a>(&'a self, path: &'a str) -> (&'a str, Vec<&'a str>) {
        let root = self
            .0
            .iter()
            .map(String::as_str)
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.len())
            .unwrap_or(ROOT);
        let Some(rest) = path.strip_prefix(root) else {
            return (root, Vec::new());
        };
        let segments = rest.trim_end_matches(".rb").split('/').collect();
        (root, segments)
    }

    /// The constant a path spells as Zeitwerk camelizes it: `http_client` is
    /// `HttpClient`.
    pub fn name_of(&self, path: &str) -> String {
        self.segments(path)
            .1
            .iter()
            .map(|segment| camelize(segment))
            .collect::<Vec<_>>()
            .join("::")
    }
}

/// A constant path as the index matches it: one segment per namespace, each
/// without underscores and in lower case.
pub type Key = Vec<String>;

/// What a constant path names in the index.
pub enum Entry {
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
    /// An index naming the files it takes in from `roots`.
    pub fn new(roots: Roots) -> Self {
        Self {
            roots,
            ..Self::default()
        }
    }

    /// The constant path the file at `path` spells, as the index matches it.
    pub fn key_of(&self, path: &str) -> Key {
        self.roots.key_of(path)
    }

    /// Takes in the files at `paths`, refusing a name two files spell and a
    /// name `defined` says the realm already has.
    pub fn add(
        &mut self,
        paths: impl IntoIterator<Item = String>,
        defined: impl Fn(&[String]) -> bool,
        log: &dyn Log,
    ) {
        for path in paths {
            let key = self.key_of(&path);
            self.add_namespaces(&path);
            if self.refusals.contains(&key) {
                continue;
            }
            if let Some(other) = self.files.remove(&key) {
                log.record(
                    Level::Warn,
                    Some(&first_line_of(&path)),
                    &format!(
                        "{other} and {path} both name {}, so neither loads by name",
                        self.roots.name_of(&other)
                    ),
                );
                self.refusals.insert(key);
                continue;
            }
            if defined(&key) {
                self.warn_of_defined(&path, log);
                self.refusals.insert(key);
                continue;
            }
            self.files.insert(key, path);
        }
    }

    /// The constant paths the index names a file for.
    pub fn file_keys(&self) -> Vec<Key> {
        self.files.keys().cloned().collect()
    }

    /// Refuses the files naming `keys`, constants the realm has come to have
    /// since it took the files in.
    pub fn refuse_defined(&mut self, keys: impl IntoIterator<Item = Key>, log: &dyn Log) {
        for key in keys {
            if let Some(path) = self.files.remove(&key) {
                self.warn_of_defined(&path, log);
                self.refusals.insert(key);
            }
        }
    }

    /// What `key` names, if anything: a file is what a namespace's own file
    /// is too, so it answers before the directory.
    pub fn entry(&self, key: &[String]) -> Option<Entry> {
        match self.files.get(key) {
            Some(path) => Some(Entry::File(path.clone())),
            None => self.namespaces.get(key).cloned().map(Entry::Namespace),
        }
    }

    /// What `name` names from inside the namespaces `scope` spells, looked for
    /// from the innermost namespace outward, as Rails' classic autoloader does,
    /// and how many of those namespaces it was found inside.
    pub fn entry_by_name(&self, scope: &[String], name: &str) -> Option<(usize, Entry)> {
        (0..=scope.len()).rev().find_map(|depth| {
            let mut key: Key = scope[..depth]
                .iter()
                .map(|segment| normalize(segment))
                .collect();
            key.push(normalize(name));
            self.entry(&key).map(|entry| (depth, entry))
        })
    }

    /// What the index names directly inside the namespace `key` spells: its
    /// files, and its directories' modules.
    pub fn members(&self, key: &[String]) -> Vec<Key> {
        self.keys()
            .filter(|named| named.len() == key.len() + 1 && named.starts_with(key))
            .collect()
    }

    /// What the index names `name` inside the namespaces below the one
    /// `scope` spells.
    pub fn keys_below(&self, scope: &[String], name: &str) -> Vec<Key> {
        let name = normalize(name);
        self.keys()
            .filter(|named| named.len() > scope.len() + 1 && named.starts_with(scope))
            .filter(|named| named.last() == Some(&name))
            .collect()
    }

    /// Whether the file at `path` is one the index names.
    pub fn is_named(&self, path: &str) -> bool {
        self.files
            .get(&self.key_of(path))
            .is_some_and(|named| named == path)
    }

    fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.files.keys().chain(self.namespaces.keys()).cloned()
    }

    // Every directory a file sits in below its root directory is a
    // namespace, which directories under other root directories may spell
    // too, named by the first directory to spell it.
    fn add_namespaces(&mut self, path: &str) {
        let (root, segments) = self.roots.segments(path);
        for depth in 1..segments.len() {
            let key = segments[..depth]
                .iter()
                .map(|segment| normalize(segment))
                .collect();
            self.namespaces.entry(key).or_insert_with(|| Namespace {
                directory: format!("{root}{}/", segments[..depth].join("/")),
                name: camelize(segments[depth - 1]),
            });
        }
    }

    fn warn_of_defined(&self, path: &str, log: &dyn Log) {
        log.record(
            Level::Warn,
            Some(&first_line_of(path)),
            &format!(
                "{path} names {}, which the realm already has, so it never loads by name",
                self.roots.name_of(path)
            ),
        );
    }
}

/// The file a constant path written inside the namespaces `scope` spells
/// names among `paths` named from `roots`, found as a realm's loader finds
/// it: each name from the innermost namespace outward, and a name two files
/// spell naming none.
pub fn file_by_name(
    paths: Vec<String>,
    roots: Roots,
    scope: &[String],
    names: &[String],
) -> Option<String> {
    let mut index = ClassIndex::new(roots);
    index.add(paths, |_| false, &Silence);
    let mut scope = scope.to_vec();
    let mut named = None;
    for name in names {
        let (depth, found) = index.entry_by_name(&scope, name)?;
        scope.truncate(depth);
        scope.push(name.clone());
        named = Some(found);
    }
    match named? {
        Entry::File(path) => Some(path),
        Entry::Namespace(_) => None,
    }
}

// A log for an index read outside a realm, whose warnings the realm gives.
struct Silence;

impl Log for Silence {
    fn print_line(&self, _text: &str) {}

    fn print(&self, _text: &str) {}

    fn record(&self, _level: Level, _at: Option<&Location>, _text: &str) {}
}

/// A segment as the index matches it.
pub fn normalize(segment: &str) -> String {
    segment
        .chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

/// A path segment as Zeitwerk camelizes it: `http_client` is `HttpClient`.
pub fn camelize(segment: &str) -> String {
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
fn first_line_of(path: &str) -> Location {
    Location {
        file: path.to_owned(),
        line: 1,
        function: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::Roots;

    fn roots(directories: &[&str]) -> Roots {
        Roots::new(directories.iter().map(|directory| (*directory).to_owned()))
    }

    #[test]
    fn a_file_is_named_from_the_nearest_root_directory_it_sits_under() {
        let roots = roots(&["res://src", "res://src/ui/"]);

        assert_eq!(roots.key_of("res://src/ui/hud.rb"), ["hud"]);
        assert_eq!(
            roots.key_of("res://src/items/potion.rb"),
            ["items", "potion"]
        );
        assert_eq!(
            roots.key_of("res://tools/http_client.rb"),
            ["tools", "httpclient"]
        );
    }

    #[test]
    fn a_directory_only_sharing_a_root_directorys_prefix_is_named_from_res() {
        let roots = roots(&["res://src"]);

        assert_eq!(roots.key_of("res://srcs/player.rb"), ["srcs", "player"]);
    }

    #[test]
    fn res_itself_and_a_directory_outside_it_add_no_root_directory() {
        let roots = roots(&["res://", "user://saves"]);

        assert_eq!(roots.key_of("res://saves/slot.rb"), ["saves", "slot"]);
    }

    #[test]
    fn a_path_outside_res_spells_no_constant() {
        let roots = roots(&["res://src"]);

        assert!(roots.key_of("").is_empty());
        assert!(roots.key_of("user://slot.rb").is_empty());
    }
}
