//! What a Ruby script keeps and answers only for the editor: the
//! placeholders of nodes it makes no instance for, the template mark of a
//! new file, and the class's page in the help.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use godot::classes::IScriptExtension;
use godot::meta::conv::RawPtr;
use godot::prelude::*;
use godot::sys::{self, GodotFfi};

use super::{RubyScript, describe_member, no_instance};
use crate::game;
use crate::log::GodotLog;
use crate::realm;
use crate::settings;
use crate::snapshot::{self, Property};
use crate::{announcement, documentation};

/// What a script keeps for the editor.
#[derive(Default)]
pub struct EditorState {
    // The placeholders Godot made of the script in the editor and has not
    // erased, told again what the class exports whenever that changes.
    placeholders: Mutex<Vec<Placeholder>>,
    // What they were last told, as `Exports::digest` has it: Godot asks for
    // the exports again for every property of a scene it saves, and telling
    // a placeholder makes the editor list its properties anew.
    last_exports_digest: Mutex<Option<(u32, u32)>>,
    // Whether the script is a template made for a new file, which learns its
    // path only as it is first saved. Taken without binding the script
    // mutably, since another thread may be reading it then.
    is_from_template: AtomicBool,
}

/// What a placeholder is told of the class: what it exports, and the value
/// each was declared with.
struct Exports {
    properties: Array<AnyDictionary>,
    values: VarDictionary,
}

impl Exports {
    // A digest of what it tells, which differs whenever that does.
    fn digest(&self) -> (u32, u32) {
        (
            self.properties.to_variant().hash_u32(),
            self.values.to_variant().hash_u32(),
        )
    }

    // Tells `placeholder`, which Godot answers by asking the script about the
    // class again.
    fn tell(&self, placeholder: sys::GDExtensionScriptInstancePtr) {
        // SAFETY: the placeholder is one the engine made for this script and
        // has not erased, and both values outlive the call.
        unsafe {
            sys::interface_fn!(placeholder_script_instance_update)(
                placeholder,
                self.properties.sys(),
                self.values.sys(),
            );
        }
    }
}

/// A placeholder Godot made of a script, which only Godot frees.
struct Placeholder(sys::GDExtensionScriptInstancePtr);

// SAFETY: the pointer is only handed back to Godot, which keeps the
// placeholder alive until it tells the script it erased it.
unsafe impl Send for Placeholder {}

// The scripts holding placeholders, which are told what their classes
// declare whenever a file runs in the editor's realm.
static PLACEHOLDER_SCRIPTS: Mutex<Vec<InstanceId>> = Mutex::new(Vec::new());
// The paths of scripts Godot made placeholders of since the last frame,
// whose files the editor's realm runs then.
static PLACEHOLDER_PATHS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Runs in the game's realm the file of each script Godot made a
/// placeholder of since the last frame, unless it is one the editor leaves
/// out, then tells every placeholder what its class declares now, as it
/// does once files have `run_again`: the editor learns a class's hints,
/// headings and the members only running defines as it would a GDScript's.
/// Placeholders exist only in the editor, and the extension calls this
/// every frame.
pub fn run_placed(run_again: bool) {
    let paths = std::mem::take(
        &mut *PLACEHOLDER_PATHS
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
    if paths.is_empty() {
        if run_again {
            tell_placed();
        }
        return;
    }
    let test_directories = settings::test_directories();
    for path in paths {
        if game::is_left_out_in_editor(&path, &test_directories) {
            continue;
        }
        if let Err(error) = realm::enter(|realm| realm.run(&path)) {
            error.write(&GodotLog);
        }
    }
    tell_placed();
}

// Tells every script holding placeholders to tell them what its class
// declares, which each does only when that changed.
fn tell_placed() {
    let placed = PLACEHOLDER_SCRIPTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    for id in placed {
        if let Ok(mut script) = Gd::<RubyScript>::try_from_instance_id(id) {
            update_exports(&mut script.bind_mut());
        }
    }
}

impl RubyScript {
    /// The script a template made for a new file, holding `source`.
    pub fn from_template(source: GString) -> Gd<Self> {
        let script = Self::from_source("", source);
        script
            .bind()
            .editor_state
            .is_from_template
            .store(true, Ordering::Relaxed);
        script
    }

    /// Whether the script is a template made for a new file, which it stops
    /// being once asked.
    pub fn take_template_mark(&self) -> bool {
        self.editor_state
            .is_from_template
            .swap(false, Ordering::Relaxed)
    }

    // Keeps the script among those holding placeholders, and its file, if it
    // has one, for the editor's realm to run at the next frame.
    fn place(&self) {
        let id = self.base().instance_id();
        let mut placed = PLACEHOLDER_SCRIPTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !placed.contains(&id) {
            placed.push(id);
        }
        let path = self.base().get_path().to_string();
        let mut unrun = PLACEHOLDER_PATHS
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !path.is_empty() && !unrun.contains(&path) {
            unrun.push(path);
        }
    }

    fn placeholders(&self) -> MutexGuard<'_, Vec<Placeholder>> {
        self.editor_state
            .placeholders
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    // The properties the class exported, its ancestors' included, nearest
    // first: what each file declared as it ran, or what its header writes
    // while it has not run.
    fn properties(&self) -> Vec<Property> {
        self.lineage().collect_properties(&snapshot::latest())
    }

    // What the class exports and the value each was declared with, which is
    // all the editor has to show a placeholder while no object of the class
    // exists.
    fn exports(&self) -> Exports {
        let mut values = VarDictionary::new();
        for property in self.properties() {
            values.set(&StringName::from(&property.name), &property.default_value());
        }
        Exports {
            properties: self.members().iter().map(describe_member).collect(),
            values,
        }
    }

    // The name the script's page goes by in the editor's help and the page
    // it inherits from: the names the editor lists the class and its base
    // by, or for a script it does not announce, its path quoted, as a
    // GDScript without `class_name` is named, and its engine class.
    fn read_doc_names(&self) -> (String, String) {
        let path = self.base().get_path().to_string();
        match announcement::announcement_by_path(&path) {
            Ok(announcement) => (announcement.name, announcement.base),
            Err(_) => (
                format!("\"{}\"", path.trim_start_matches("res://")),
                self.ancestry()
                    .map(|ancestry| ancestry.engine_class().to_owned())
                    .unwrap_or_default(),
            ),
        }
    }
}

// The editor makes no instance, so Godot asks for its own placeholder
// and shows the node what that carries: the header's answer at first,
// then what the class declares once the editor's realm has run the file.
pub fn create_placeholder(script: &RubyScript, for_object: Gd<Object>) -> RawPtr<*mut c_void> {
    let Some(language) = script.get_language() else {
        return no_instance();
    };
    // SAFETY: the interface is initialized while the extension runs, and
    // Godot frees the placeholder with the object it was made for.
    let placeholder = unsafe {
        sys::interface_fn!(placeholder_script_instance_create)(
            language.obj_sys(),
            script.to_gd().obj_sys(),
            for_object.obj_sys(),
        )
    };
    if script.header.is_parsed() {
        script.exports().tell(placeholder);
    }
    script.placeholders().push(Placeholder(placeholder));
    script.place();
    // SAFETY: the pointer is the placeholder Godot just made.
    unsafe { RawPtr::new(placeholder.cast::<c_void>()) }
}

pub fn erase_placeholder(script: &RubyScript, placeholder: RawPtr<*mut c_void>) {
    let erased = placeholder.ptr().cast();
    let mut placeholders = script.placeholders();
    placeholders.retain(|kept| kept.0 != erased);
    if placeholders.is_empty() {
        let id = script.base().instance_id();
        PLACEHOLDER_SCRIPTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|placed| *placed != id);
    }
}

// Godot asks the script about the class as each placeholder is told, so
// the script is let go of while they are. A source that does not parse
// tells them nothing, as a GDScript's does not: telling a placeholder
// drops what it kept of the scene beyond what the class declares.
pub fn update_exports(script: &mut RubyScript) {
    if !script.header.is_parsed() {
        return;
    }
    let exported = script.exports();
    let digest = Some(exported.digest());
    let mut last_exports_digest = script
        .editor_state
        .last_exports_digest
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if *last_exports_digest == digest {
        return;
    }
    *last_exports_digest = digest;
    drop(last_exports_digest);
    let placeholders: Vec<_> = script.placeholders().iter().map(|kept| kept.0).collect();
    let _released = script.base_mut();
    for placeholder in placeholders {
        exported.tell(placeholder);
    }
}

pub fn read_doc_class_name(script: &RubyScript) -> StringName {
    StringName::from(&script.read_doc_names().0)
}

// The class's page in the editor's help, from what the source's comments
// say of the class and its members, as a GDScript's `##` comments give
// its page.
pub fn describe_documentation(script: &RubyScript) -> Array<AnyDictionary> {
    let path = script.base().get_path().to_string();
    let documentation =
        documentation::read_documentation(&script.source.to_string(), &script.header);
    let (name, inherits) = script.read_doc_names();
    let describe = |docs: &[documentation::MemberDoc]| -> Array<AnyDictionary> {
        docs.iter().map(describe_member_doc).collect()
    };
    let class = vdict! {
        "name" => name,
        "inherits" => inherits,
        "brief_description" => documentation.brief,
        "description" => documentation.description,
        "methods" => &describe(&documentation.methods),
        "properties" => &describe(&documentation.properties),
        "signals" => &describe(&documentation.signals),
        "is_script_doc" => true,
        "script_path" => path,
    };
    std::iter::once(class.upcast_any_dictionary()).collect()
}

// A member of a class's page in the editor's help.
fn describe_member_doc(doc: &documentation::MemberDoc) -> AnyDictionary {
    vdict! {
        "name" => doc.name.as_str(),
        "description" => doc.description.as_str(),
    }
    .upcast_any_dictionary()
}
