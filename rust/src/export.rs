//! What the editor exports of a project: every file Godot would ship, but for
//! the tests, which stay in the project they test.

use godot::classes::{EditorExportPlugin, EditorPlugin, IEditorExportPlugin, IEditorPlugin};
use godot::prelude::*;

use crate::settings;

const RUNNER_SCENE: &str = "res://addons/godot_mruby/runner.tscn";

/// Hands the editor the export plugin for as long as the editor is open.
#[derive(GodotClass)]
#[class(tool, init, base = EditorPlugin)]
pub struct RubyEditorPlugin {
    export: Option<Gd<RubyExportPlugin>>,
    base: Base<EditorPlugin>,
}

#[godot_api]
impl IEditorPlugin for RubyEditorPlugin {
    fn enter_tree(&mut self) {
        let export = RubyExportPlugin::new_gd();
        self.base_mut().add_export_plugin(&export);
        self.export = Some(export);
    }

    fn exit_tree(&mut self) {
        if let Some(export) = self.export.take() {
            self.base_mut().remove_export_plugin(&export);
        }
    }
}

/// Leaves out of an export the files under the project's test directories
/// and the runner scene, whatever the export preset includes.
#[derive(GodotClass)]
#[class(tool, init, base = EditorExportPlugin)]
pub struct RubyExportPlugin {
    base: Base<EditorExportPlugin>,
}

#[godot_api]
impl IEditorExportPlugin for RubyExportPlugin {
    fn get_name(&self) -> GString {
        "godot_mruby".into()
    }

    fn export_file(&mut self, path: GString, _type: GString, _features: PackedStringArray) {
        let path = path.to_string();
        if path == RUNNER_SCENE
            || settings::is_in_test_directory(&path, &settings::test_directories())
        {
            self.base_mut().skip();
        }
    }

    // Godot calls these only for a plugin that begins customizing, which
    // this one never does.
    fn customize_resource(
        &mut self,
        _resource: Gd<Resource>,
        _path: GString,
    ) -> Option<Gd<Resource>> {
        None
    }

    fn customize_scene(&mut self, _scene: Gd<Node>, _path: GString) -> Option<Gd<Node>> {
        None
    }

    fn get_customization_configuration_hash(&self) -> u64 {
        0
    }
}
