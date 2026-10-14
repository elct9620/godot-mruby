//! What the addon adds to the editor while it is open.

use godot::classes::{EditorInterface, EditorPlugin, IEditorPlugin};
use godot::prelude::*;

use crate::announcement;
use crate::export::RubyExportPlugin;
use crate::game;
use crate::highlighter::RubySyntaxHighlighter;
use crate::panel::RubyTestPanel;
use crate::realm::Files;
use crate::settings;

/// Hands the editor the export plugin, the test panel and the Ruby
/// highlighter for as long as the editor is open, and lists the game's files
/// again whenever the editor finds them changed, having it ask again of the
/// files sharing a name with one added or removed.
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
                .connect_other(
                    &*self,
                    |_| ask_again_for_name_sharers(&game::relist_files()),
                );
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

// Has the editor ask again for the class name of each file sharing a file
// name with one in `changes`, since a file arriving or leaving turns whether
// its name is shared; it asks only of the changed files by itself.
fn ask_again_for_name_sharers(changes: &[String]) {
    if changes.is_empty() {
        return;
    }
    let sharers = announcement::collect_name_sharers(
        &game::listing().paths(),
        changes,
        &settings::test_directories(),
    );
    if let Some(mut file_system) = EditorInterface::singleton().get_resource_filesystem() {
        for path in sharers {
            file_system.update_file(&path);
        }
    }
}
