use godot::init::InitStage;
use godot::prelude::*;

mod ancestry;
mod announcement;
mod bridge;
mod compiler;
mod game;
mod instance;
mod language;
mod loader;
mod log;
mod minitest;
mod parser;
mod realm;
mod runner;
mod script;
mod settings;
mod snapshot;

struct GodotMruby;

#[gdextension]
unsafe impl ExtensionLibrary for GodotMruby {
    // Scene is the stage a script language has to be registered by.
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Scene {
            settings::register();
            language::register();
            loader::register();
            realm::prepare(|| {
                realm::Realm::open(game::GameFiles, log::GodotLog, |realm| {
                    realm.install::<bridge::Godot>()
                })
            });
        }
    }

    // Lets go of the Ruby objects of nodes freed since the last frame, even
    // when no Ruby runs to take them.
    fn on_main_loop_frame() {
        realm::release_queued();
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Scene {
            loader::unregister();
            language::unregister();
            realm::close();
        }
    }
}
