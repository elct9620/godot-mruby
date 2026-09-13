use godot::classes::{DirAccess, INode, Node, Os};
use godot::prelude::*;

use crate::error;
use crate::minitest::{self, Minitest};
use crate::realm::{self, RubyError};
use crate::settings;

/// The node the addon's runner scene holds. Once in the tree it installs the
/// test framework into the game's realm, runs every test file of the test
/// directories it is given there, then quits with 0 when every test passed
/// and 1 otherwise.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct RubyTestRunner {
    base: Base<Node>,
}

#[godot_api]
impl INode for RubyTestRunner {
    fn ready(&mut self) {
        let args = user_args();
        let run_as_asked =
            test_directories(&args).and_then(|directories| Ok((directories, test_options(&args)?)));
        let passed = match run_as_asked {
            Ok((directories, options)) => run(&directories, options),
            Err(refused) => {
                error!("{refused}");
                false
            }
        };
        self.base()
            .get_tree()
            .quit_ex()
            .exit_code(if passed { 0 } else { 1 })
            .done();
    }
}

fn run(directories: &[String], options: minitest::Options) -> bool {
    let pattern = settings::test_pattern();
    let mut paths = Vec::new();
    for directory in directories {
        if !DirAccess::dir_exists_absolute(directory) {
            error!("The test directory {directory} does not exist");
            return false;
        }
        collect_test_files(directory, &pattern, &mut paths);
    }
    paths.sort();
    // Every test file runs, so each one's problems are in the log before any
    // test runs, where a reader of the run's output looks for them.
    let loaded = logged(realm::enter(|realm| {
        realm.install::<Minitest>()?;
        Ok(paths
            .iter()
            .map(|path| logged(realm.run_file(path).map(|()| true)))
            .fold(true, |all, loaded| all & loaded))
    }));
    let passed = logged(realm::enter(|realm| {
        realm.call("Minitest", c"run", options)
    }));
    loaded && passed
}

// An outcome Ruby could not reach counts as a failure, written to the log.
fn logged(outcome: Result<bool, RubyError>) -> bool {
    outcome.unwrap_or_else(|failed| {
        failed.log();
        false
    })
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

fn collect_test_files(directory: &str, pattern: &GString, paths: &mut Vec<String>) {
    for file in DirAccess::get_files_at(directory).as_slice() {
        if file.match_glob(pattern) {
            paths.push(join(directory, &file.to_string()));
        }
    }
    for child in DirAccess::get_directories_at(directory).as_slice() {
        collect_test_files(&join(directory, &child.to_string()), pattern, paths);
    }
}

fn join(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}
