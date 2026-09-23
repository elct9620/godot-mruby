# Export

How a game ships without its tests: each layer keeps test code out of an exported game on its own, so a game whose export let a test file through still cannot run it.

## Includes

- `rust/src/settings.rs`
- `tasks/support/export.rb`

## `RX-001` An exported game's class index leaves out a test directory

| Step | Statement |
| --- | --- |
| Given | an exported game |
| Given | a pack loaded into it that carries a file under a test directory |
| When | Ruby in the game names that file's constant |
| Then | Ruby raises `NameError` |

## `RX-002` An export leaves out the files under a test directory

| Step | Statement |
| --- | --- |
| Given | a project with a file under a test directory |
| When | the project is exported |
| Then | the exported game has no such file |

## `RX-003` An export leaves out the runner scene

| Step | Statement |
| --- | --- |
| Given | a project with the addon's runner scene |
| When | the project is exported |
| Then | the exported game has no runner scene |

## `RX-004` A test directory holds its files however its setting ends

| Step | Statement |
| --- | --- |
| Given | a test directory set with a trailing slash |
| When | a file under it is asked about |
| Then | the file is under a test directory |

## `RX-005` A directory whose name begins with a test directory's is not one

| Step | Statement |
| --- | --- |
| Given | a test directory `res://test` |
| When | a file under `res://testing` is asked about |
| Then | the file is not under a test directory |

## `RX-006` The test runner refuses to run in an exported game

| Step | Statement |
| --- | --- |
| Given | an exported game |
| When | a test runner node enters its scene tree |
| Then | the log carries an error saying the runner does not run in an exported game |
