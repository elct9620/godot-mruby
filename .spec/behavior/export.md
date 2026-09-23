# Export

How a game ships without its tests: each layer keeps test code out of an exported game on its own, so a game whose export let a test file through still cannot run it.

## Includes

- `tasks/support/exported_game.rb`

## `RX-001` An exported game's class index leaves out a test directory

| Step | Statement |
| --- | --- |
| Given | an exported game |
| Given | a pack loaded into it that carries a file under a test directory |
| When | Ruby in the game names that file's constant |
| Then | Ruby raises `NameError` |
