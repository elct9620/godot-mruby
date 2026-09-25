use godot::classes::ProjectSettings;
use godot::prelude::*;

// @setting mruby/loader/root_directories
const ROOT_DIRECTORIES: &str = "mruby/loader/root_directories";
// @setting mruby/test/directories
const TEST_DIRECTORIES: &str = "mruby/test/directories";
const DEFAULT_TEST_DIRECTORY: &str = "res://test";
// @setting mruby/test/pattern
const TEST_PATTERN: &str = "mruby/test/pattern";
const DEFAULT_TEST_PATTERN: &str = "*_test.rb";

/// Adds the extension's settings with their defaults, each shown without the
/// Project Settings dialog's Advanced Settings toggle. A default is not saved
/// into project.godot, so every process defines them again when the
/// extension starts, keeping whatever value the project set.
pub fn register() {
    define(
        ROOT_DIRECTORIES,
        &PackedStringArray::new().to_variant(),
        VariantType::PACKED_STRING_ARRAY,
    );
    let directories = PackedStringArray::from(&[GString::from(DEFAULT_TEST_DIRECTORY)]);
    define(
        TEST_DIRECTORIES,
        &directories.to_variant(),
        VariantType::PACKED_STRING_ARRAY,
    );
    define(
        TEST_PATTERN,
        &DEFAULT_TEST_PATTERN.to_variant(),
        VariantType::STRING,
    );
}

fn define(name: &str, default: &Variant, kind: VariantType) {
    let mut settings = ProjectSettings::singleton();
    if !settings.has_setting(name) {
        settings.set_setting(name, default);
    }
    settings.set_initial_value(name, default);
    settings.set_as_basic(name, true);
    let info = vdict! { "name" => name, "type" => kind.ord() };
    settings.add_property_info(&info.upcast_any_dictionary());
}

/// The root directories the project names besides `res://`.
pub fn root_directories() -> Vec<String> {
    directories(ROOT_DIRECTORIES)
}

// The editor's own setting, with the default it registers it with; a game
// that is not the editor never registers it.
const TEMPLATES_SEARCH_PATH: &str = "editor/script/templates_search_path";
const DEFAULT_TEMPLATE_DIRECTORY: &str = "res://script_templates";

/// The directories the test runner runs.
pub fn test_directories() -> Vec<String> {
    directories(TEST_DIRECTORIES)
}

fn directories(setting: &str) -> Vec<String> {
    ProjectSettings::singleton()
        .get_setting(setting)
        .to::<PackedStringArray>()
        .as_slice()
        .iter()
        .map(GString::to_string)
        .collect()
}

/// Whether `path` lies under one of `directories`, each written with
/// or without its trailing slash.
pub fn is_in_directory(path: &str, directories: &[String]) -> bool {
    directories.iter().any(|directory| {
        path.strip_prefix(directory.trim_end_matches('/'))
            .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Where the project keeps its own script templates, which the editor reads
/// as text and fills in, so none of them is a file of the game.
pub fn template_directory() -> String {
    let directory = ProjectSettings::singleton().get_setting(TEMPLATES_SEARCH_PATH);
    if directory.is_nil() {
        DEFAULT_TEMPLATE_DIRECTORY.to_owned()
    } else {
        directory.to_string()
    }
}

/// The glob a file name under a test directory matches to be a test file.
pub fn test_pattern() -> GString {
    ProjectSettings::singleton().get_setting(TEST_PATTERN).to()
}

#[cfg(test)]
mod tests {
    use super::is_in_directory;

    // @behavior RX-004
    #[test]
    fn a_test_directory_written_with_its_trailing_slash_holds_its_files() {
        let directories = ["res://test/".to_owned()];

        assert!(is_in_directory(
            "res://test/inventory_test.rb",
            &directories
        ));
    }

    // @behavior RX-005
    #[test]
    fn a_directory_whose_name_only_begins_like_a_test_directory_is_not_in_it() {
        let directories = ["res://test".to_owned()];

        assert!(!is_in_directory("res://testing/inventory.rb", &directories));
    }
}
