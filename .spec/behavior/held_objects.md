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
