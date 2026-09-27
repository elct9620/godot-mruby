# Threads

How Godot's calls on the language, a script or the loader meet when they come from several threads at once, as the editor's scan, loading threads and any thread printing an error make them: a call that changes the object waits for the calls other threads have under way.

## Includes

- `rust/tests/threads.rs`

## `RB-001` A mutable call waits for another thread's call

| Step | Statement |
| --- | --- |
| Given | a thread that took the first shared bind of an object and let it go while another thread still holds one |
| When | the first thread binds the object mutably |
| Then | it waits until the other thread lets go, instead of failing |
