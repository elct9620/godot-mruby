use godot::prelude::*;

struct GodotMruby;

#[gdextension]
unsafe impl ExtensionLibrary for GodotMruby {}
