use godot::classes::ProjectSettings;
use godot::prelude::*;

// @setting mruby/test/directories
const TEST_DIRECTORIES: &str = "mruby/test/directories";
const DEFAULT_TEST_DIRECTORY: &str = "res://test";
// @setting mruby/test/pattern
const TEST_PATTERN: &str = "mruby/test/pattern";
const DEFAULT_TEST_PATTERN: &str = "*_test.rb";

/// Adds the extension's settings with their defaults. A default is not saved
/// into project.godot, so every process defines them again when the
/// extension starts, keeping whatever value the project set.
pub fn register() {
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
    let info = vdict! { "name" => name, "type" => kind.ord() };
    settings.add_property_info(&info.upcast_any_dictionary());
}

/// The directories the test runner runs.
pub fn test_directories() -> Vec<String> {
    ProjectSettings::singleton()
        .get_setting(TEST_DIRECTORIES)
        .to::<PackedStringArray>()
        .as_slice()
        .iter()
        .map(GString::to_string)
        .collect()
}

/// The glob a file name under a test directory matches to be a test file.
pub fn test_pattern() -> GString {
    ProjectSettings::singleton().get_setting(TEST_PATTERN).to()
}
