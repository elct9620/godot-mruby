use std::sync::Mutex;

use godot::classes::{
    FileAccess, IResourceFormatLoader, ResourceFormatLoader, ResourceLoader, Script,
};
use godot::global::Error;
use godot::prelude::*;

use crate::script::RubyScript;

/// Loads a `.rb` file as a `RubyScript`, keeping its source without running it.
#[derive(GodotClass)]
#[class(base = ResourceFormatLoader, init, tool)]
pub struct ResourceFormatLoaderRubyScript {
    base: Base<ResourceFormatLoader>,
}

static REGISTERED: Mutex<Option<InstanceId>> = Mutex::new(None);

pub fn register() {
    let loader = ResourceFormatLoaderRubyScript::new_gd();
    ResourceLoader::singleton().add_resource_format_loader(&loader);
    *REGISTERED.lock().unwrap() = Some(loader.instance_id());
}

pub fn unregister() {
    if let Some(id) = REGISTERED.lock().unwrap().take() {
        let loader = Gd::<ResourceFormatLoaderRubyScript>::from_instance_id(id);
        ResourceLoader::singleton().remove_resource_format_loader(&loader);
    }
}

#[godot_api]
impl IResourceFormatLoader for ResourceFormatLoaderRubyScript {
    fn get_recognized_extensions(&self) -> PackedStringArray {
        PackedStringArray::from(&[GString::from("rb")])
    }

    fn handles_type(&self, type_: StringName) -> bool {
        type_ == Script::class_id().to_string_name()
            || type_ == RubyScript::class_id().to_string_name()
    }

    fn get_resource_type(&self, path: GString) -> GString {
        if path.get_extension() == "rb" {
            RubyScript::class_id().to_gstring()
        } else {
            GString::new()
        }
    }

    // A loader reports failure by returning the error code in place of the resource.
    fn load(
        &self,
        path: GString,
        _original_path: GString,
        _use_sub_threads: bool,
        _cache_mode: i32,
    ) -> Variant {
        let source = FileAccess::get_file_as_string(&path);
        let error = FileAccess::get_open_error();
        if error != Error::OK {
            return error.to_variant();
        }

        RubyScript::from_source(source).to_variant()
    }
}
