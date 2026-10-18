use godot::init::InitStage;
use godot::prelude::*;

mod ancestry;
mod announcement;
mod bridge;
mod compiler;
#[cfg(feature = "editor")]
mod completion;
#[cfg(feature = "editor")]
mod documentation;
#[cfg(feature = "editor")]
mod export;
mod game;
mod header;
#[cfg(feature = "editor")]
mod highlighter;
mod hint;
mod instance;
mod language;
#[cfg(feature = "editor")]
mod lexer;
mod loader;
mod log;
#[cfg(feature = "editor")]
mod lookup;
#[cfg(feature = "editor")]
mod minitest;
#[cfg(feature = "editor")]
mod panel;
#[cfg(feature = "editor")]
mod plugin;
mod realm;
#[cfg(feature = "editor")]
mod runner;
mod saver;
mod script;
mod settings;
mod snapshot;
#[cfg(feature = "editor")]
mod template;
#[cfg(feature = "editor")]
mod validation;

struct GodotMruby;

// How long a thread waits for the game's realm before the log tells of it:
// longer than Ruby holds the realm for a frame's work, short enough that a
// game stuck on two threads waiting for each other is told of at once.
const REALM_PATIENCE: std::time::Duration = std::time::Duration::from_secs(5);

#[gdextension]
unsafe impl ExtensionLibrary for GodotMruby {
    // Scene is the stage a script language has to be registered by.
    fn on_stage_init(stage: InitStage) {
        if stage == InitStage::Scene {
            settings::register();
            language::register();
            loader::register();
            saver::register();
            realm::watch_waits(realm::Waits {
                patience: REALM_PATIENCE,
                log: Box::new(log::GodotLog),
                thread_name: log::name_thread,
            });
            realm::prepare(|| {
                realm::Realm::open(game::RealmFiles, log::GodotLog, |realm| {
                    realm.install::<bridge::Godot>()
                })
            });
        }
    }

    // Lets go of the reference-counted objects Ruby let go of, and the Ruby
    // objects of objects freed since the last frame, even when no Ruby runs
    // to take them, and runs again the files of scripts
    // reloaded since: each class takes back its exports, then gives the
    // objects it made before the ones it exports anew. In the editor, runs
    // the files of the scripts it made placeholders of, and tells the
    // placeholders what the files run declared.
    fn on_main_loop_frame() {
        bridge::release_collected();
        realm::release_queued();
        let reruns = realm::rerun_queued(&realm::Rerun {
            withdraw: c"__withdraw__",
            prune: c"__prune__",
            adopt: c"__adopt__",
        });
        script::remake_instances(&reruns.remake_paths);
        script::run_placed(reruns.has_run);
    }

    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Scene {
            saver::unregister();
            loader::unregister();
            language::unregister();
            realm::close();
            bridge::release_collected();
        }
    }
}
