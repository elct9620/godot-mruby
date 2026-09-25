use std::sync::Mutex;

use godot::classes::file_access::ModeFlags;
use godot::classes::{
    EditorInterface, Engine, FileAccess, IResourceFormatSaver, Resource, ResourceFormatSaver,
    ResourceSaver,
};
use godot::global::Error;
use godot::obj::Singleton;
use godot::prelude::*;

use crate::game::GameFiles;
use crate::realm::Files;
use crate::script::RubyScript;
use crate::{language, template};

// The editor setting that has a script saved in the editor reload, which
// GDScript's saver follows too.
const RELOAD_ON_SAVE: &str = "text_editor/behavior/files/auto_reload_and_parse_scripts_on_save";

/// Saves a `RubyScript` as a `.rb` file holding its source.
#[derive(GodotClass)]
#[class(base = ResourceFormatSaver, init, tool)]
pub struct ResourceFormatSaverRubyScript {
    base: Base<ResourceFormatSaver>,
}

static SAVER: Mutex<Option<InstanceId>> = Mutex::new(None);

pub fn register() {
    let saver = ResourceFormatSaverRubyScript::new_gd();
    ResourceSaver::singleton().add_resource_format_saver(&saver);
    *SAVER.lock().unwrap() = Some(saver.instance_id());
}

pub fn unregister() {
    if let Some(id) = SAVER.lock().unwrap().take() {
        let saver = Gd::<ResourceFormatSaverRubyScript>::from_instance_id(id);
        ResourceSaver::singleton().remove_resource_format_saver(&saver);
    }
}

#[godot_api]
impl IResourceFormatSaver for ResourceFormatSaverRubyScript {
    // The file takes the source as Godot holds it, and a script saved in the
    // editor reloads, as a GDScript does.
    fn save(&mut self, resource: Option<Gd<Resource>>, path: GString, _flags: u32) -> Error {
        let Some(mut script) = resource.and_then(|resource| resource.try_cast::<RubyScript>().ok())
        else {
            return Error::ERR_INVALID_PARAMETER;
        };
        if script.bind().take_template_mark() {
            nest_template(&mut script, &path.to_string());
        }
        let Some(mut file) = FileAccess::open(&path, ModeFlags::WRITE) else {
            return FileAccess::get_open_error();
        };
        file.store_string(&script.get_source_code());
        let error = file.get_error();
        if error != Error::OK && error != Error::ERR_FILE_EOF {
            return Error::ERR_CANT_CREATE;
        }
        drop(file);
        if is_reloaded_on_save() {
            script.reload();
        }
        Error::OK
    }

    fn recognize(&self, resource: Option<Gd<Resource>>) -> bool {
        resource.is_some_and(|resource| resource.try_cast::<RubyScript>().is_ok())
    }

    fn get_recognized_extensions(&self, resource: Option<Gd<Resource>>) -> PackedStringArray {
        if self.recognize(resource) {
            PackedStringArray::from(&[GString::from("rb")])
        } else {
            PackedStringArray::new()
        }
    }
}

// Puts a template made for a new file inside the namespaces its path
// spells, the first place the path is known.
fn nest_template(script: &mut Gd<RubyScript>, path: &str) {
    let constant = GameFiles.roots().name_of(path);
    let nested = template::source_in_namespaces(
        &script.get_source_code().to_string(),
        &constant,
        &language::indentation(),
    );
    script.set_source_code(&nested);
}

// Whether a script saved now reloads: only in the editor, as it sets.
fn is_reloaded_on_save() -> bool {
    Engine::singleton().is_editor_hint()
        && EditorInterface::singleton()
            .get_editor_settings()
            .is_some_and(|settings| settings.get_setting(RELOAD_ON_SAVE).booleanize())
}
