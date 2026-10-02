//! What the addon adds to the editor while it is open.

use godot::classes::{EditorInterface, EditorPlugin, IEditorPlugin};
use godot::prelude::*;

use crate::export::RubyExportPlugin;
use crate::game;
use crate::highlighter::RubySyntaxHighlighter;
use crate::panel::RubyTestPanel;

/// Hands the editor the export plugin, the test panel and the Ruby
/// highlighter for as long as the editor is open, and lists the game's files again whenever the editor
/// finds them changed.
#[derive(GodotClass)]
#[class(tool, init, base = EditorPlugin)]
pub struct RubyEditorPlugin {
    export: Option<Gd<RubyExportPlugin>>,
    panel: Option<Gd<RubyTestPanel>>,
    highlighter: Option<Gd<RubySyntaxHighlighter>>,
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
        if let Some(mut script_editor) = EditorInterface::singleton().get_script_editor() {
            let highlighter = RubySyntaxHighlighter::new_gd();
            script_editor.register_syntax_highlighter(&highlighter);
            self.highlighter = Some(highlighter);
        }
        if let Some(file_system) = EditorInterface::singleton().get_resource_filesystem() {
            file_system
                .signals()
                .filesystem_changed()
                .connect_other(&*self, |_| game::relist_files());
        }
    }

    fn exit_tree(&mut self) {
        if let Some(export) = self.export.take() {
            self.base_mut().remove_export_plugin(&export);
        }
        if let Some(mut panel) = self.panel.take() {
            self.base_mut().remove_dock(&panel);
            panel.queue_free();
        }
        if let (Some(highlighter), Some(mut script_editor)) = (
            self.highlighter.take(),
            EditorInterface::singleton().get_script_editor(),
        ) {
            script_editor.unregister_syntax_highlighter(&highlighter);
        }
    }
}
