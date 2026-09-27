# Addon

How a project comes to run Ruby: the addon under `addons/godot_mruby/` carries the extension's library and the `.gdextension` that names it, and the editor loads it with the project.

## Includes

- `godot/test/unit/addon/**/*_test.rb`
- `tasks/support/addon.rb`
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

## `RA-003` The package names each crate it changed, with the change

| Step | Statement |
| --- | --- |
| Given | a package built with a crate changed from its published version |
| When | its third-party notices are read |
| Then | the crate's section says it was changed and shows the source before and after |
