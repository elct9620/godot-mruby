# Conventions

How code in godot-mruby is written, beside [`architecture.md`](architecture.md), which says how its parts fit. Every rule here applies to Rust, Ruby and GDScript alike.

## 1. Naming

### 1.1 Precedence

A name is settled by three steps, in order; the first that decides it ends the search.

1. Keep a name the language or another side already fixes, as 1.9 lists.
2. Give it the word class of what it names, as 1.2 to 1.4 lay out.
3. Read it where it is used, as 1.5 shows.

A convention kept in step 1 may look like an exception to step 2. It stays, because a reader of that language expects it.

### 1.2 Word Classes

What a name stands for decides its word class.

| What is named | Its name | As in |
| --- | --- | --- |
| A module or a type | a noun, or an adjective and a noun | `Snapshot`, `BoundMethod` |
| A method | a verb, then its object or a preposition | `collect_members`, `move_to` |
| An attribute | a noun | `name`, `digest` |
| An enum variant | a noun or an adjective | `Heading::Category`, `Run::Running` |
| A variable or field | a noun, or the state it records | `owner`, `expired` |

An attribute answers what a value holds and takes nothing, as `player.name` does. In Ruby, decide first whether a name is an attribute or a method. A boolean variable takes `is_` or `has_` where that reads clearer than its state.

### 1.3 Method Names

A method is named for what it does, and these rows say how each kind shows it.

| A method that | Its name | As in |
| --- | --- | --- |
| acts | a verb first, a state after it if any | `publish`, `release_queued` |
| works a thing out | a verb and the thing | `read_name`, `trace_ancestry` |
| finds a thing by a key | the noun, `_by_` and the key | `file_by_name` |
| asks a question | `is_` or `has_`; in Ruby, `?` | `is_hidden`, `empty?` |
| makes or converts a value | `new`, `with_`, `from_`, `into_`, `to_` | `with_hint`, `to_engine` |

A handle the call runs in, such as beni's `&Mrb`, is not an argument. A function given only that handle is an attribute, as beni's `name(self, mrb)` is.

### 1.4 Participles

A word ending in `-ed` or `-ing` says what was done to a thing. That rarely names an action, so it stands only in these places.

| Where | As in |
| --- | --- |
| An enum variant's state | `Run::Running`, `Game::Closed` |
| After a verb | `release_queued` |
| A variable's state | `expired` |
| A dictionary noun, in the sense it plays | `heading`, `warning` |
| A name step 1 keeps | `const_added`, `passed?` |

A dictionary noun counts only in its dictionary sense. `Failing` is a noun meaning a shortcoming, so it cannot name the runs that must fail.

### 1.5 Call-Site Reading

A name reads at the call site as a phrase, without a comment repeating it.

```ruby
player.move_to(position)      # subject, verb, preposition, object
index.file_by_name(names)     # a lookup reads as its key

source.line_at(offset)        # drift: a noun first, so no act is named
source.locate_line(offset)    # a verb names what the call works out
```

A preposition stands after a verb, never alone after a noun.

### 1.6 File Names

A file is named as the module or type it defines.

| File | Named after | As in |
| --- | --- | --- |
| A Rust module | the Godot extension point it implements, or what it makes or does | `language.rs`, `bridge.rs` |
| A Ruby file | the constant it defines, as the loader requires | `http_client.rb` → `HttpClient` |
| A GDScript or scene | the node it scripts or holds | `battle.tscn` |
| A test file | its subject, with `_test` | `loader_test.rb` |

A Rust module with parts of its own keeps them in a directory of its name.

### 1.7 Fixtures

A fixture is a thing prepared for a test, so it is a noun. The situation it sets up goes in its comment.

```ruby
# Declares one signal with two shapes, which the second declaration refuses.
class Gong < Godot::Node
  signal :rung, times: Integer
  signal :rung, times: Integer, loudly: Godot::Bool
end
```

A directory of fixtures is a noun too, named for what it holds.

### 1.8 Test Names

A test's name says what it checks, so a failure reads without opening the test.

| Test | Its name | As in |
| --- | --- | --- |
| A Rust test | the behaviour as a sentence | `a_realm_opened_again_gives_each_extension_its_data_afresh` |
| A Minitest test | `test_`, then the behaviour | `test_a_node_classs_object_refuses_to_be_copied` |

The `test_` prefix is Minitest's own, so step 1 keeps it.

### 1.9 Fixed Names

These names come from another side, and step 1 keeps each as that side spells it.

| Side | As in |
| --- | --- |
| Rust's API Guidelines | `new`, `into_`, a getter without `get_` |
| Rust's `log` crate, which the log follows | `Level::Warn`, `error!`, `warn!` |
| Godot's virtuals | `get_name`, `ready` |
| Ruby's hooks and predicates | `const_added`, `empty?` |
| Minitest | `passed?`, `test_` |
| beni | `const_defined_at` |
| GDScript's words in Ruby's DSL | `signal`, `icon` |
| Godot's global utility functions | `randf`, `is_same` |
| GUT's test helpers in Minitest | `autoqfree`, `wait_process_frames` |
| A Rust function behind Ruby's `__name__` | `name` |
