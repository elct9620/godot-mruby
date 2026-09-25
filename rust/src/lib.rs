use godot::init::InitStage;
use godot::prelude::*;

mod ancestry;
mod announcement;
mod bridge;
mod compiler;
mod export;
mod game;
mod header;
mod hint;
mod instance;
mod language;
mod loader;
mod log;
mod minitest;
mod panel;
mod plugin;
mod realm;
mod runner;
mod saver;
mod script;
mod settings;
mod snapshot;
mod template;
mod validation;

struct GodotMruby;

#[gdextension]
unsafe impl ExtensionLibrary for GodotMruby {
    // Scene is the stage a script language has to be registered by.
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Scene {
            settings::register();
            language::register();
            loader::register();
            saver::register();
            realm::prepare(|| {
                realm::Realm::open(game::RealmFiles, log::GodotLog, |realm| {
                    realm.install::<bridge::Godot>()
                })
            });
        }
    }

    // Lets go of the Ruby objects of nodes freed since the last frame, even
    // when no Ruby runs to take them, and runs again the files of scripts
    // reloaded since: each class takes back its exports, then gives the
    // objects it made before the ones it exports anew. In the editor, runs
    // the files of the scripts it made placeholders of, and tells the
    // placeholders what the files run declared.
    fn on_main_loop_frame() {
        realm::release_queued();
        let run_again = realm::rerun_queued(&realm::Rerun {
            withdraw: c"__withdraw__",
            adopt: c"__adopt__",
        });
        script::run_placed(run_again);
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Scene {
            saver::unregister();
            loader::unregister();
            language::unregister();
            realm::close();
        }
    }
}
