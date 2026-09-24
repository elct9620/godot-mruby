# Reloading

How a changed source takes effect while the game runs: what Godot reads of a script follows the source it now holds, and a reloaded file runs again in the same realm, so the objects already made keep their state and take the new methods.

## Includes

- `godot/test/unit/reload/**/*_test.rb`

## `RF-001` A node script reports the engine class the file it extends now reaches

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another file's class, asked for its instance base type once |
| Given | the other file's source Godot holds, changed to extend another engine node class |
| When | Godot asks the script for its instance base type |
| Then | the script answers the engine class the changed source extends |
