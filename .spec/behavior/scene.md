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

## `RW-006` A test waiting for physics frames continues inside the last of them

| Step | Statement |
| --- | --- |
| Given | a test that reads the engine's physics frame count |
| When | the test waits for 3 physics frames |
| Then | the count has grown by 3 and the engine is in a physics frame |

## `RW-007` A test begins where a process frame begins after one that waited into a physics frame

| Step | Statement |
| --- | --- |
| Given | a test that has waited for a physics frame |
| When | the next test begins |
| Then | it begins where a process frame begins, outside any physics frame |

## `RW-008` A test waiting for seconds continues once that much physics time has passed

| Step | Statement |
| --- | --- |
| Given | a test that reads the engine's physics frame count |
| When | the test waits for 0.1 seconds |
| Then | the physics frames begun since, at the physics delta each, add up to 0.1 seconds within one frame's delta |

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
