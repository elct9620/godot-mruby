# Addon

How a project comes to run Ruby: the addon under `addons/godot_mruby/` carries the extension's library and the `.gdextension` that names it, and the editor loads it with the project.

## Includes

- `tasks/support/godot.rb`

## `RA-001` The editor loads the addon at startup

| Step | Statement |
| --- | --- |
| Given | a project whose extension list names the addon |
| When | the editor starts headless and quits |
| Then | Godot's output shows the extension initialized, with no error |
