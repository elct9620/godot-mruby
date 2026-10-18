use std::sync::Mutex;

use godot::classes::file_access::ModeFlags;
use godot::classes::{
    FileAccess, IResourceFormatSaver, Resource, ResourceFormatSaver, ResourceSaver,
};
use godot::global::Error;
use godot::obj::Singleton;
use godot::prelude::*;

use crate::script::RubyScript;

#[cfg(feature = "editor")]
mod editor;

// Only the editor makes templates or reloads what it saves, so a library
// built without it writes the source as it is.
#[cfg(not(feature = "editor"))]
mod editor {
    use godot::prelude::*;

    use crate::script::RubyScript;

    pub fn nest_template(_: &mut Gd<RubyScript>, _: &str) {}

    pub fn reload_saved(_: &mut Gd<RubyScript>) {}
}

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
        editor::nest_template(&mut script, &path.to_string());
        let Some(mut file) = FileAccess::open(&path, ModeFlags::WRITE) else {
            return FileAccess::get_open_error();
        };
        file.store_string(&script.get_source_code());
        let error = file.get_error();
        if error != Error::OK && error != Error::ERR_FILE_EOF {
            return Error::ERR_CANT_CREATE;
        }
        drop(file);
        editor::reload_saved(&mut script);
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
