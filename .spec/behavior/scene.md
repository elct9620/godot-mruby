# Scene

How a test reaches the scene a game runs in: the runner resumes the run once per frame, so a test waits for frames in straight-line code and sees what the engine did in them.

## Includes

- `godot/test/unit/scene/**/*.rb`

## `RW-001` A test waiting for process frames continues after that many frames

| Step | Statement |
| --- | --- |
| Given | a test that reads the engine's process frame count |
| When | the test waits for 3 process frames |
| Then | the count has grown by 3 |
