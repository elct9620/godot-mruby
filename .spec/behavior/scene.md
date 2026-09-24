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

## `RW-002` A node added with `add_child_autofree` goes under the test root

| Step | Statement |
| --- | --- |
| Given | a test and a node that has no parent |
| When | the test adds the node with `add_child_autofree` |
| Then | the node's parent is the test's test root |

## `RW-003` A node given to `autofree` is freed once the test's teardown has run

| Step | Statement |
| --- | --- |
| Given | a test that gives a node outside the tree to `autofree` |
| When | the test's teardown has run |
| Then | the node has been freed |

## `RW-004` A node under the test root is freed with it once the test's teardown has run

| Step | Statement |
| --- | --- |
| Given | a test that adds a node under its test root with `add_child` |
| When | the test's teardown has run |
| Then | the node has been freed |

## `RW-005` A node the test freed itself is left to it

| Step | Statement |
| --- | --- |
| Given | a test that gives a node to `autofree`, then frees the node |
| When | the test's teardown has run |
| Then | the test passes |
