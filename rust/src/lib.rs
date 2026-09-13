use godot::init::InitStage;
use godot::prelude::*;

mod instance;
mod interpreter;
mod language;
mod loader;
mod runner;
mod script;
mod settings;

struct GodotMruby;

#[gdextension]
unsafe impl ExtensionLibrary for GodotMruby {
    // Scene is the stage a script language has to be registered by.
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Scene {
            settings::register();
            language::register();
            loader::register();
        }
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Scene {
            loader::unregister();
            language::unregister();
            interpreter::close();
        }
    }
}
