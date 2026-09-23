# Addon

How a project comes to run Ruby: the addon under `addons/godot_mruby/` carries the extension's library and the `.gdextension` that names it, and the editor loads it with the project.

## Includes

- `godot/test/unit/addon/**/*_test.rb`
- `tasks/support/godot.rb`

## `RA-001` The editor loads the addon at startup

| Step | Statement |
| --- | --- |
| Given | a project whose extension list names the addon |
| When | the editor starts headless and quits |
| Then | the log shows the extension initialized, with no error |

## `RA-002` The extension's settings are shown without Advanced Settings

| Step | Statement |
| --- | --- |
| Given | a project with the addon |
| When | Godot lists the project's settings |
| Then | each setting under `mruby/` is one the Project Settings dialog shows without its Advanced Settings toggle |
