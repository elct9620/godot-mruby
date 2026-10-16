use std::sync::Mutex;

use godot::classes::{
    FileAccess, IResourceFormatLoader, ResourceFormatLoader, ResourceLoader, Script,
};
use godot::global::Error;
use godot::prelude::*;

use crate::realm;
use crate::script::RubyScript;

/// Loads a `.rb` file as a `RubyScript`, keeping its source without running it.
#[derive(GodotClass)]
#[class(base = ResourceFormatLoader, init, tool)]
pub struct ResourceFormatLoaderRubyScript {
    base: Base<ResourceFormatLoader>,
}

static LOADER: Mutex<Option<InstanceId>> = Mutex::new(None);

pub fn register() {
    let loader = ResourceFormatLoaderRubyScript::new_gd();
    ResourceLoader::singleton().add_resource_format_loader(&loader);
    *LOADER.lock().unwrap() = Some(loader.instance_id());
}

pub fn unregister() {
    if let Some(id) = LOADER.lock().unwrap().take() {
        let loader = Gd::<ResourceFormatLoaderRubyScript>::from_instance_id(id);
        ResourceLoader::singleton().remove_resource_format_loader(&loader);
    }
}

#[godot_api]
impl IResourceFormatLoader for ResourceFormatLoaderRubyScript {
    fn get_recognized_extensions(&self) -> PackedStringArray {
        PackedStringArray::from(&[GString::from("rb")])
    }

    // An inner script's path is its file's followed by the names of its
    // class inside the file's class, which no extension check recognizes.
    fn recognize_path(&self, path: GString, _type: StringName) -> bool {
        let path = path.to_string();
        realm::split_inner_path(&path).0.ends_with(".rb")
    }

    fn handles_type(&self, type_: StringName) -> bool {
        type_ == Script::class_id().to_string_name()
            || type_ == RubyScript::class_id().to_string_name()
    }

    fn get_resource_type(&self, path: GString) -> GString {
        if self.recognize_path(path, StringName::default()) {
            RubyScript::class_id().to_gstring()
        } else {
            GString::new()
        }
    }

    // A loader reports failure by returning the error code in place of the resource.
    fn load(
        &self,
        path: GString,
        original_path: GString,
        _use_sub_threads: bool,
        _cache_mode: i32,
    ) -> Variant {
        let script_path = path.to_string();
        if let (file, names) = realm::split_inner_path(&script_path)
            && !names.is_empty()
        {
            return load_inner(file, &names)
                .map_or_else(|error| error.to_variant(), |script| script.to_variant());
        }
        let source = FileAccess::get_file_as_string(&path);
        let error = FileAccess::get_open_error();
        if error != Error::OK {
            return error.to_variant();
        }

        // An exported game loads a remapped file under the name it was asked
        // for, and the path Godot gives the script is that name.
        let name = if original_path.is_empty() {
            &path
        } else {
            &original_path
        };
        RubyScript::from_source(&name.to_string(), source).to_variant()
    }
}

// The inner script the script of the file at `file` keeps for the class
// `names` spells inside the file's class, or why there is none.
fn load_inner(file: &str, names: &[&str]) -> Result<Gd<RubyScript>, Error> {
    let script = ResourceLoader::singleton()
        .load(file)
        .and_then(|resource| resource.try_cast::<RubyScript>().ok())
        .ok_or(Error::ERR_FILE_NOT_FOUND)?;
    let names: Vec<String> = names.iter().map(|&name| name.to_owned()).collect();
    Ok(script.bind().inner_script_by_names(&names))
}
