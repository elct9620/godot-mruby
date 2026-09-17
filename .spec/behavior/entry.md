# Entry

How a thread goes into the game's realm: one thread at a time is inside, and a thread already inside goes on, since Godot calls back into scripts while Ruby it started is still running.

## Includes

- `rust/src/realm.rs`

## `RE-001` A thread inside the realm enters it again

| Step | Statement |
| --- | --- |
| Given | a thread inside the game's realm |
| When | the thread enters the realm again |
| Then | the inner entry runs and returns to the outer one |

## `RE-002` Another thread waits until the outermost entry returns

| Step | Statement |
| --- | --- |
| Given | a thread inside the game's realm, entered twice |
| Given | the inner entry, returned |
| When | another thread enters the realm |
| Then | it runs only once the outer entry has returned |

## `RE-003` An entry made while the realm opens fails

| Step | Statement |
| --- | --- |
| Given | the game's realm, opening at its first entry |
| When | what it opens with enters the realm |
| Then | that entry fails |

## `RE-004` Closing the realm from inside it leaves it open

| Step | Statement |
| --- | --- |
| Given | a thread inside the game's realm |
| When | the thread closes the realm |
| Then | the entry it is inside still reaches the realm |

## `RE-005` An entry after the realm panicked while opening opens it again

| Step | Statement |
| --- | --- |
| Given | the game's realm, which panicked while opening at an entry |
| When | the realm is entered again |
| Then | the realm opens |
