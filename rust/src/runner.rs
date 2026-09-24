use godot::classes::{DirAccess, INode, Node, Os, ResourceLoader};
use godot::prelude::*;

use crate::error;
use crate::game;
use crate::log::GodotLog;
use crate::minitest::{self, Minitest};
use crate::realm::{self, RubyError};
use crate::settings;

/// The node the addon's runner scene holds. Once in the tree it installs the
/// test framework into the game's realm and runs every test file of the test
/// directories it is given there. The tests run from the next process frame
/// on, the run resumed as each frame begins while a test waits, so every test
/// starts and continues where a frame begins. It quits with 0 when every test
/// passed and 1 otherwise.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct RubyTestRunner {
    base: Base<Node>,
    // What the run starts with, until it starts.
    options: Option<minitest::Options>,
    // Whether every test file ran without an error, which a passing run
    // still needs.
    loaded: bool,
}

#[godot_api]
impl INode for RubyTestRunner {
    fn ready(&mut self) {
        let args = user_args();
        let run_as_asked = refuse_exported_game()
            .and_then(|()| test_directories(&args))
            .and_then(|directories| test_files(&directories))
            .and_then(|tests| Ok((tests, test_options(&args)?)));
        match run_as_asked {
            Ok((tests, options)) => {
                self.loaded = load(&tests);
                self.options = Some(options);
                let tree = self.base().get_tree();
                tree.signals()
                    .process_frame()
                    .connect_other(&*self, Self::run_frame);
            }
            Err(refused) => {
                error!("{refused}");
                self.quit(false);
            }
        }
    }
}

impl RubyTestRunner {
    // Starts the run, or resumes it where a test waits; quits once it is over.
    fn run_frame(&mut self) {
        let options = self.options.take();
        let over = logged(realm::enter(|realm| match options {
            Some(options) => realm.call("Minitest", c"start", [options]),
            None => realm.call("Minitest", c"resume", std::iter::empty::<bool>()),
        }));
        if let Some(passed) = over {
            self.quit(passed && self.loaded);
        }
    }

    fn quit(&self, passed: bool) {
        self.base()
            .get_tree()
            .quit_ex()
            .exit_code(if passed { 0 } else { 1 })
            .done();
    }
}

// The test files under `directories`, in name order.
fn test_files(directories: &[String]) -> Result<Vec<String>, String> {
    let pattern = settings::test_pattern();
    let mut tests = Vec::new();
    for directory in directories {
        if !DirAccess::dir_exists_absolute(directory) {
            return Err(format!("The test directory {directory} does not exist"));
        }
        collect_tests(directory, &pattern, &mut tests);
    }
    tests.sort();
    Ok(tests)
}

// Installs the test framework and runs every test file, answering whether
// each one ran without an error. Every file runs, so each one's problems are
// in the log before any test runs, where a reader of the run's output looks
// for them.
fn load(tests: &[String]) -> bool {
    logged(realm::enter(|realm| {
        realm.install::<Minitest>()?;
        Ok(tests
            .iter()
            .map(|path| logged(realm.run(path).map(|()| true)))
            .fold(true, |all, loaded| all & loaded))
    }))
}

// An outcome Ruby could not reach counts as a failed run, written to the log.
fn logged<T: From<bool>>(outcome: Result<T, RubyError>) -> T {
    outcome.unwrap_or_else(|failed| {
        failed.write(&GodotLog);
        T::from(false)
    })
}

// An exported game ships no tests, and a runner it still has is refused
// rather than run against whatever reached the game.
fn refuse_exported_game() -> Result<(), String> {
    if game::is_exported() {
        Err("The test runner does not run in an exported game".to_owned())
    } else {
        Ok(())
    }
}

// What follows `--` on Godot's command line.
fn user_args() -> Vec<String> {
    Os::singleton()
        .get_cmdline_user_args()
        .as_slice()
        .iter()
        .map(GString::to_string)
        .collect()
}

// @option --dir
// The directories this run covers: the one `--dir` names, which has to be
// one of the project's test directories, or every one of them.
fn test_directories(args: &[String]) -> Result<Vec<String>, String> {
    let project = settings::test_directories();
    let named = option(args, &["--dir"]);
    let directories = named
        .clone()
        .map_or_else(|| project.clone(), |dir| vec![dir]);
    if let Some(root) = directories
        .iter()
        .find(|dir| dir.trim_end_matches('/') == "res:")
    {
        return Err(format!(
            "{root} cannot be a test directory: it is the whole project"
        ));
    }
    match named {
        Some(dir) if !project.contains(&dir) => Err(format!(
            "{dir} is not one of the project's test directories in mruby/test/directories"
        )),
        _ => Ok(directories),
    }
}

fn test_options(args: &[String]) -> Result<minitest::Options, String> {
    // @option --seed
    let seed = option(args, &["-s", "--seed"])
        .map(|seed| {
            seed.parse()
                .map_err(|_| format!("--seed takes a whole number, not {seed}"))
        })
        .transpose()?;
    Ok(minitest::Options {
        // @option --include
        include: option(args, &["-i", "--include"]),
        // @option --exclude
        exclude: option(args, &["-e", "--exclude"]),
        seed,
    })
}

// The value after the first of `names` on the command line.
fn option(args: &[String], names: &[&str]) -> Option<String> {
    args.windows(2)
        .find(|pair| names.contains(&pair[0].as_str()))
        .map(|pair| pair[1].clone())
}

// The test files under `directory`, as Godot lists its resources: each one
// whose name matches the project's test pattern.
fn collect_tests(directory: &str, pattern: &GString, tests: &mut Vec<String>) {
    for entry in ResourceLoader::singleton()
        .list_directory(directory)
        .as_slice()
    {
        if let Some(child) = entry.to_string().strip_suffix('/') {
            collect_tests(&join(directory, child), pattern, tests);
        } else if entry.match_glob(pattern) {
            tests.push(join(directory, &entry.to_string()));
        }
    }
}

fn join(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}
