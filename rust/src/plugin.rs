//! What the addon adds to the editor while it is open.

use godot::classes::{EditorPlugin, IEditorPlugin};
use godot::prelude::*;

use crate::export::RubyExportPlugin;
use crate::panel::RubyTestPanel;

/// Hands the editor the export plugin and the test panel for as long as the
/// editor is open.
#[derive(GodotClass)]
#[class(tool, init, base = EditorPlugin)]
pub struct RubyEditorPlugin {
    export: Option<Gd<RubyExportPlugin>>,
    panel: Option<Gd<RubyTestPanel>>,
    base: Base<EditorPlugin>,
}

#[godot_api]
impl IEditorPlugin for RubyEditorPlugin {
    fn enter_tree(&mut self) {
        let export = RubyExportPlugin::new_gd();
        self.base_mut().add_export_plugin(&export);
        self.export = Some(export);
        let panel = RubyTestPanel::new();
        self.base_mut().add_dock(&panel);
        self.panel = Some(panel);
    }

    fn exit_tree(&mut self) {
        if let Some(export) = self.export.take() {
            self.base_mut().remove_export_plugin(&export);
        }
        if let Some(mut panel) = self.panel.take() {
            self.base_mut().remove_dock(&panel);
            panel.queue_free();
        }
    }
}
