//! The editor's test panel: it plays the test runner's own scene with the
//! options it leaves in the run file, then lists what the run's results file
//! says of each test, and opens the Ruby line a failure names.

use godot::classes::control::SizeFlags;
use godot::classes::editor_dock::DockSlot;
use godot::classes::{
    Button, DirAccess, EditorDock, EditorInterface, FileAccess, HBoxContainer, IEditorDock, Json,
    Label, OptionButton, ResourceLoader, Script, Tree, TreeItem, VBoxContainer,
};
use godot::prelude::*;

use crate::runner::{RUN_FILE, RUNNER_SCENE};
use crate::settings;

/// Where the panel asks the run to write its results.
const RESULTS_FILE: &str = "user://godot_mruby/results.json";
const ALL_DIRECTORIES: &str = "All test directories";

/// Runs a project's Ruby tests from the editor and lists their results. Its
/// controls are made once it is ready: Godot also makes one it never adds to
/// the tree, to read the class's defaults.
#[derive(GodotClass)]
#[class(tool, init, base = EditorDock)]
pub struct RubyTestPanel {
    #[init(val = OnReady::new(OptionButton::new_alloc))]
    directories: OnReady<Gd<OptionButton>>,
    #[init(val = OnReady::new(Button::new_alloc))]
    run: OnReady<Gd<Button>>,
    #[init(val = OnReady::new(Label::new_alloc))]
    status: OnReady<Gd<Label>>,
    #[init(val = OnReady::new(Tree::new_alloc))]
    rows: OnReady<Gd<Tree>>,
    // Whether a run this panel started is playing, so its end is noticed.
    is_waiting: bool,
    base: Base<EditorDock>,
}

#[godot_api]
impl IEditorDock for RubyTestPanel {
    fn ready(&mut self) {
        self.run.set_text("Run");
        self.run
            .signals()
            .pressed()
            .connect_other(&*self, Self::start_run);
        self.status.set_text("Not run yet");
        self.rows.set_columns(3);
        self.rows.set_hide_root(true);
        self.rows.set_column_expand(0, false);
        self.rows.set_column_custom_minimum_width(0, 80);
        self.rows.set_v_size_flags(SizeFlags::EXPAND_FILL);
        self.rows
            .signals()
            .item_activated()
            .connect_other(&*self, Self::open_activated);

        let mut bar = HBoxContainer::new_alloc();
        bar.add_child(&*self.directories);
        bar.add_child(&*self.run);
        bar.add_child(&*self.status);
        let mut column = VBoxContainer::new_alloc();
        column.add_child(&bar);
        column.add_child(&*self.rows);
        self.base_mut().add_child(&column);
        self.list_directories();
    }

    fn process(&mut self, _delta: f64) {
        if self.is_waiting && !EditorInterface::singleton().is_playing_scene() {
            self.is_waiting = false;
            self.run.set_disabled(false);
            self.list_results();
        }
    }
}

impl RubyTestPanel {
    /// A panel titled and placed in the bottom slot, as the editor has to
    /// know before the dock is added.
    pub fn new() -> Gd<Self> {
        let mut panel = Self::new_alloc();
        panel.set_title("Ruby Tests");
        panel.set_default_slot(DockSlot::BOTTOM);
        panel
    }

    // The project's test directories, after the choice to run them all.
    fn list_directories(&mut self) {
        self.directories.clear();
        self.directories.add_item(ALL_DIRECTORIES);
        for directory in settings::test_directories() {
            self.directories.add_item(&directory);
        }
    }

    // Leaves the run file and plays the runner scene, whose results are
    // listed once it stops.
    fn start_run(&mut self) {
        let selected = self.directories.get_selected();
        let directory =
            (selected > 0).then(|| self.directories.get_item_text(selected).to_string());
        if let Err(error) = write_run_file(&run_args(directory.as_deref())) {
            self.status
                .set_text(&format!("The run file was not written: {error:?}"));
            return;
        }
        DirAccess::remove_absolute(RESULTS_FILE);
        self.rows.clear();
        self.status.set_text("Running…");
        self.run.set_disabled(true);
        self.is_waiting = true;
        EditorInterface::singleton().play_custom_scene(RUNNER_SCENE);
    }

    // What the run's results file says: each test file that did not load,
    // then the tests that failed, raised or skipped, and the counts as the
    // run's summary words them.
    fn list_results(&mut self) {
        let Some(results) = results() else {
            self.status
                .set_text("The run wrote no results; its output says why");
            return;
        };
        let root = self.rows.create_item();
        let errors = entries(&results, "errors");
        let tests = entries(&results, "tests");
        for error in &errors {
            self.add_row(root.as_ref(), "error", &text(error, "file"), error);
        }
        for kind in ["failure", "error", "skip"] {
            for test in tests.iter().filter(|test| text(test, "result") == kind) {
                let name = format!("{}#{}", text(test, "class"), text(test, "name"));
                self.add_row(root.as_ref(), kind, &name, test);
            }
        }
        let count = |kind: &str| {
            tests
                .iter()
                .filter(|test| text(test, "result") == kind)
                .count()
        };
        self.status.set_text(&format!(
            "{} runs, {} failures, {} errors, {} skips",
            tests.len(),
            count("failure"),
            count("error") + errors.len(),
            count("skip")
        ));
    }

    // A row naming what did not pass, which opens its Ruby line when
    // activated.
    fn add_row(
        &mut self,
        root: Option<&Gd<TreeItem>>,
        result: &str,
        name: &str,
        entry: &VarDictionary,
    ) {
        let Some(mut row) = self.rows.create_item_ex().parent(root).done() else {
            return;
        };
        row.set_text(0, result);
        row.set_text(1, name);
        let message = text(entry, "message");
        row.set_text(2, message.lines().next().unwrap_or_default());
        row.set_tooltip_text(2, &message);
        row.set_metadata(0, &entry.get_or_nil("file"));
        row.set_metadata(1, &entry.get_or_nil("line"));
    }

    fn open_activated(&mut self) {
        let Some(row) = self.rows.get_selected() else {
            return;
        };
        let file = row.get_metadata(0);
        let Ok(file) = file.try_to::<GString>() else {
            return;
        };
        // JSON reads every number as a float.
        let line = row
            .get_metadata(1)
            .try_to::<f64>()
            .map_or(1, |line| line as i32);
        let script = ResourceLoader::singleton()
            .load(&file)
            .and_then(|resource| resource.try_cast::<Script>().ok());
        if let Some(script) = script {
            EditorInterface::singleton()
                .edit_script_ex(&script)
                .line(line)
                .done();
        }
    }
}

/// What the run file holds for a run of `directory`, or of every test
/// directory, writing its results where the panel reads them.
pub fn run_args(directory: Option<&str>) -> Vec<String> {
    let mut args = vec!["--results".to_owned(), RESULTS_FILE.to_owned()];
    if let Some(directory) = directory {
        args.extend(["--dir".to_owned(), directory.to_owned()]);
    }
    args
}

fn write_run_file(args: &[String]) -> Result<(), godot::global::Error> {
    let directory = RUN_FILE.rsplit_once('/').map_or(RUN_FILE, |(dir, _)| dir);
    DirAccess::make_dir_recursive_absolute(directory);
    let mut file = FileAccess::open(RUN_FILE, godot::classes::file_access::ModeFlags::WRITE)
        .ok_or_else(FileAccess::get_open_error)?;
    let args: PackedStringArray = args.iter().map(GString::from).collect();
    file.store_string(&Json::stringify(&args.to_variant()));
    Ok(())
}

fn results() -> Option<VarDictionary> {
    if !FileAccess::file_exists(RESULTS_FILE) {
        return None;
    }
    Json::parse_string(&FileAccess::get_file_as_string(RESULTS_FILE))
        .try_to::<VarDictionary>()
        .ok()
}

fn entries(results: &VarDictionary, key: &str) -> Vec<VarDictionary> {
    results
        .get_or_nil(key)
        .try_to::<VarArray>()
        .map(|entries| {
            entries
                .iter_shared()
                .filter_map(|entry| entry.try_to::<VarDictionary>().ok())
                .collect()
        })
        .unwrap_or_default()
}

// A string an entry holds under `key`, or nothing when it holds none.
fn text(entry: &VarDictionary, key: &str) -> String {
    entry
        .get(key)
        .filter(|value| !value.is_nil())
        .map(|value| value.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{RESULTS_FILE, run_args};

    #[test]
    fn a_run_of_every_directory_names_only_its_results_file() {
        assert_eq!(run_args(None), ["--results", RESULTS_FILE]);
    }

    #[test]
    fn a_run_of_one_directory_names_it() {
        assert_eq!(
            run_args(Some("res://test")),
            ["--results", RESULTS_FILE, "--dir", "res://test"]
        );
    }
}
