# Held objects

How a realm holds a Ruby object for something outside it, such as a node, and lets go of it: whatever holds the key never waits for the realm to give it up.

## Includes

- `rust/src/realm.rs`

## `RO-001` A released key no longer reaches its object

| Step | Statement |
| --- | --- |
| Given | an object the game's realm holds for a key |
| Given | the key, released |
| When | a method is sent to the key at the realm's next entry |
| Then | the send fails |

## `RO-002` Releasing a key never waits for the realm

| Step | Statement |
| --- | --- |
| Given | an object the game's realm holds for a key |
| Given | another thread inside the realm |
| When | the key is released |
| Then | the release returns while that thread is still inside |

## `RO-003` A frame lets go of released keys without any other entry

| Step | Statement |
| --- | --- |
| Given | an object the game's realm holds for a key |
| Given | the key, released |
| When | the frame's release runs |
| Then | the realm holds no object |

## `RO-004` An answer converts to what its caller takes as a method's argument does

| Step | Statement |
| --- | --- |
| Given | an object the game's realm holds for a key, whose method answers a Float |
| When | the method is sent to the key by a caller taking an integer |
| Then | the caller takes the Float's integer part |

## `RO-005` An answer its caller cannot take names the call and the reason

| Step | Statement |
| --- | --- |
| Given | an object the game's realm holds for a key, whose method answers a String |
| When | the method is sent to the key by a caller taking an integer |
| Then | the send fails with the method, the answer, and why the String is not an integer |
