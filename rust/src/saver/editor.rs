//! What saving a Ruby script does only in the editor: a new file's template
//! takes its path's namespaces, and the saved script reloads as the editor
//! sets.

use godot::classes::{EditorInterface, Engine};
use godot::obj::Singleton;
use godot::prelude::*;

use crate::game::GameFiles;
use crate::realm::Files;
use crate::script::RubyScript;
use crate::{language, template};

// The editor setting that has a script saved in the editor reload, which
// GDScript's saver follows too.
const RELOAD_ON_SAVE: &str = "text_editor/behavior/files/auto_reload_and_parse_scripts_on_save";

// Puts a template made for a new file inside the namespaces its path
// spells, the first place the path is known; any other script stays as it
// is.
pub fn nest_template(script: &mut Gd<RubyScript>, path: &str) {
    if !script.bind().take_template_mark() {
        return;
    }
    let constant = GameFiles.roots().name_by_path(path);
    let nested = template::nest_in_namespaces(
        &script.get_source_code().to_string(),
        &constant,
        &language::indentation(),
    );
    script.set_source_code(&nested);
}

// Reloads a script saved in the editor, as it sets.
pub fn reload_saved(script: &mut Gd<RubyScript>) {
    let is_reloaded = Engine::singleton().is_editor_hint()
        && EditorInterface::singleton()
            .get_editor_settings()
            .is_some_and(|settings| settings.get_setting(RELOAD_ON_SAVE).booleanize());
    if is_reloaded {
        script.reload();
    }
}
