use std::cell::RefCell;

use godot::classes::{DirAccess, FileAccess, INode, Json, Node, Os, ResourceLoader};
use godot::prelude::*;

use crate::error;
use crate::game;
use crate::log::GodotLog;
use crate::minitest::{self, Frame, LoadFailure, Minitest};
use crate::realm::{self, Level, Location, Log, RubyError};
use crate::settings;

/// The node the addon's runner scene holds. Once in the tree it installs the
/// test framework into the game's realm and runs every test file of the test
/// directories it is given there. The tests run from the next process frame
/// on, the run resumed as each process or physics frame begins while a test
/// waits, so every test starts and continues where a frame begins. It quits
/// with 0 when every test passed and 1 otherwise. Given no options, it takes
/// the ones the editor's test panel left in the run file.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct RubyTestRunner {
    base: Base<Node>,
    run: Run,
    // Whether every test file ran without an error, which a passing run
    // still needs.
    is_loaded: bool,
}

// Where the run stands, which decides what a beginning frame does to it.
#[derive(Default)]
enum Run {
    // Not begun, or over: no frame moves it.
    #[default]
    Stopped,
    // What the run starts with, as the next process frame begins.
    Starting(minitest::Options),
    Running,
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
            Ok((tests, mut options)) => {
                options.load_failures = load(&tests);
                self.is_loaded = options.load_failures.is_empty();
                self.run = Run::Starting(options);
                let tree = self.base().get_tree();
                tree.signals()
                    .process_frame()
                    .connect_other(&*self, |runner| runner.run_frame(Frame::Process));
                tree.signals()
                    .physics_frame()
                    .connect_other(&*self, |runner| runner.run_frame(Frame::Physics));
            }
            Err(refused) => {
                error!("{refused}");
                self.quit(false);
            }
        }
    }
}

impl RubyTestRunner {
    // Starts the run as a process frame begins, or resumes it where a test
    // waits as any frame begins; quits once it is over.
    fn run_frame(&mut self, frame: Frame) {
        let over = match (std::mem::take(&mut self.run), frame) {
            (Run::Starting(options), Frame::Process) => log_outcome(realm::enter(|realm| {
                realm.call("Minitest", c"start", [options])
            })),
            (Run::Running, frame) => log_outcome(realm::enter(|realm| {
                realm.call("Minitest", c"resume", [frame])
            })),
            (run, _) => {
                self.run = run;
                return;
            }
        };
        match over {
            Some(passed) => self.quit(passed && self.is_loaded),
            None => self.run = Run::Running,
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

// Installs the test framework and runs every test file, answering the ones
// that did not load. Every file runs, so each one's problems are in the log
// before any test runs, where a reader of the run's output looks for them.
fn load(tests: &[String]) -> Vec<LoadFailure> {
    let failures = LoadFailures::default();
    let report = |failed: RubyError| {
        failed.write(&GodotLog);
        failed.write(&failures);
    };
    let outcome = realm::enter(|realm| {
        realm.install::<Minitest>()?;
        for path in tests {
            realm.run(path).unwrap_or_else(report);
        }
        Ok(())
    });
    outcome.unwrap_or_else(report);
    failures.0.into_inner()
}

// What a test file that did not load reported, kept for the run's results.
#[derive(Default)]
struct LoadFailures(RefCell<Vec<LoadFailure>>);

impl Log for LoadFailures {
    fn print_line(&self, _text: &str) {}

    fn print(&self, _text: &str) {}

    fn record(&self, _level: Level, at: Option<&Location>, text: &str) {
        self.0.borrow_mut().push(LoadFailure {
            message: text.to_owned(),
            at: at.cloned(),
        });
    }
}

// An outcome Ruby could not reach counts as a failed run, written to the log.
fn log_outcome<T: From<bool>>(outcome: Result<T, RubyError>) -> T {
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

// What follows `--` on Godot's command line, or what the run file holds
// when nothing does.
fn user_args() -> Vec<String> {
    let args: Vec<String> = Os::singleton()
        .get_cmdline_user_args()
        .as_slice()
        .iter()
        .map(GString::to_string)
        .collect();
    if args.is_empty() {
        run_file_args()
    } else {
        args
    }
}

/// Where the editor's test panel leaves the options of the run it plays, as
/// a JSON array of what would follow `--`.
pub const RUN_FILE: &str = "user://godot_mruby/run.json";

// The options the run file holds, removed as they are read, so a later run
// is not given them again.
fn run_file_args() -> Vec<String> {
    if !FileAccess::file_exists(RUN_FILE) {
        return Vec::new();
    }
    let text = FileAccess::get_file_as_string(RUN_FILE);
    DirAccess::remove_absolute(RUN_FILE);
    Json::parse_string(&text)
        .try_to::<VarArray>()
        .map(|args| args.iter_shared().map(|arg| arg.to_string()).collect())
        .unwrap_or_default()
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
        // @option --results
        results: option(args, &["--results"]),
        load_failures: Vec::new(),
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
