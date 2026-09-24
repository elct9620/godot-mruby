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

## `RW-009` `wait_until` answers true once its block answers true

| Step | Statement |
| --- | --- |
| Given | a block that answers `true` the third time it is called |
| When | the test waits until it with a `max_time` of 1 second |
| Then | the wait answers `true` after the third call |

## `RW-010` `wait_until` answers false once `max_time` passes with no answer of true

| Step | Statement |
| --- | --- |
| Given | a block that always answers a truthy value other than `true` |
| When | the test waits until it with a `max_time` of 0.1 seconds |
| Then | the wait answers `false` once 0.1 seconds of physics time have passed |

## `RW-011` `wait_until` given a time between calls calls its block no more often

| Step | Statement |
| --- | --- |
| Given | a block that always answers `false` |
| When | the test waits until it with a `max_time` of 0.1 seconds and 0.05 seconds between calls |
| Then | the block has been called at most twice |

## `RW-012` `wait_until` without a block raises ArgumentError

| Step | Statement |
| --- | --- |
| Given | a test |
| When | the test calls `wait_until` with no block |
| Then | it raises `ArgumentError` |

## `RW-013` `wait_for_signal` answers true once the signal is emitted

| Step | Statement |
| --- | --- |
| Given | a timer that times out after 0.05 seconds |
| When | the test waits for its `timeout` signal with a `max_time` of 1 second |
| Then | the wait answers `true` before 1 second of physics time has passed |

## `RW-014` `wait_for_signal` answers false once `max_time` passes without the signal

| Step | Statement |
| --- | --- |
| Given | a node that is never renamed |
| When | the test waits for its `renamed` signal with a `max_time` of 0.05 seconds |
| Then | the wait answers `false` |

## `RW-015` `wait_for_signal` leaves the signal with the connections it had

| Step | Statement |
| --- | --- |
| Given | a signal and the connections it has |
| When | a wait for the signal has returned |
| Then | the signal has the same connections |

## `RW-016` A signal a watched object emitted passes `assert_signal_emitted`

| Step | Statement |
| --- | --- |
| Given | a test that watches a node's signals |
| When | the node emits `property_list_changed` |
| Then | `assert_signal_emitted` passes, given the node and the name or the signal |

## `RW-017` A signal a watched object did not emit fails `assert_signal_emitted`

| Step | Statement |
| --- | --- |
| Given | a test that watches a node's signals |
| When | the test asserts `property_list_changed` was emitted, which the node never emitted |
| Then | the assertion fails |

## `RW-018` An object nobody watched fails `assert_signal_emitted`

| Step | Statement |
| --- | --- |
| Given | a node that emitted `property_list_changed` without being watched |
| When | the test asserts the signal was emitted |
| Then | the assertion fails |

## `RW-019` A signal a watched object does not have fails `assert_signal_emitted`

| Step | Statement |
| --- | --- |
| Given | a test that watches a node's signals |
| When | the test asserts a signal the node does not have was emitted |
| Then | the assertion fails |

## `RW-020` A watched object has the connections it had once the test's teardown begins

| Step | Statement |
| --- | --- |
| Given | a test that watches a node's signals |
| When | the test's teardown begins |
| Then | the node's signals have the connections they had before it was watched |

## `RW-021` `simulate` processes a node and its children as many times as asked

| Step | Statement |
| --- | --- |
| Given | a node script's node outside the tree, with another under it |
| When | the test simulates the first for 3 frames of 0.5 seconds |
| Then | each node's `_process` and `_physics_process` were given 0.5 three times |

## `RW-022` `simulate` checking processing leaves a node that is not processing

| Step | Statement |
| --- | --- |
| Given | a node script's node outside the tree, so not processing |
| When | the test simulates it for a frame, checking processing |
| Then | neither its `_process` nor its `_physics_process` was called |

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
