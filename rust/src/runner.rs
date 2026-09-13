use godot::classes::{DirAccess, FileAccess, INode, Node, Os};
use godot::prelude::*;

use crate::interpreter;

/// The node the addon's runner scene holds. Once in the tree it runs every
/// test file under the test directory in the game's interpreter, then quits
/// with 0 when every test passed and 1 otherwise.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct RubyTestRunner {
    base: Base<Node>,
}

const DEFAULT_DIRECTORY: &str = "res://test";
const TEST_FILE_SUFFIX: &str = "_test.rb";

#[godot_api]
impl INode for RubyTestRunner {
    fn ready(&mut self) {
        let passed = run(&test_directory());
        self.base()
            .get_tree()
            .quit_ex()
            .exit_code(if passed { 0 } else { 1 })
            .done();
    }
}

fn run(directory: &str) -> bool {
    if !DirAccess::dir_exists_absolute(directory) {
        godot_error!("The test directory {directory} does not exist");
        return false;
    }
    let mut paths = Vec::new();
    collect_test_files(directory, &mut paths);
    paths.sort();
    // What mruby says about a test file is reported before any test runs,
    // where a reader of the run's output looks for it.
    let (diagnostics, loaded) = interpreter::load_tests(&paths, |path| {
        FileAccess::get_file_as_string(path).to_string()
    });
    for diagnostic in diagnostics {
        diagnostic.report();
    }
    match interpreter::run_tests() {
        Ok(passed) => loaded && passed,
        Err(failed) => {
            failed.report();
            false
        }
    }
}

// @option --dir
fn test_directory() -> String {
    let args: Vec<String> = Os::singleton()
        .get_cmdline_user_args()
        .as_slice()
        .iter()
        .map(GString::to_string)
        .collect();
    args.windows(2)
        .find(|pair| pair[0] == "--dir")
        .map_or_else(|| DEFAULT_DIRECTORY.to_owned(), |pair| pair[1].clone())
}

fn collect_test_files(directory: &str, paths: &mut Vec<String>) {
    for file in DirAccess::get_files_at(directory).as_slice() {
        let file = file.to_string();
        if file.ends_with(TEST_FILE_SUFFIX) {
            paths.push(join(directory, &file));
        }
    }
    for child in DirAccess::get_directories_at(directory).as_slice() {
        collect_test_files(&join(directory, &child.to_string()), paths);
    }
}

fn join(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}
