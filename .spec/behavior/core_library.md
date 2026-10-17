# Core library

What Ruby's own classes answer in a realm beyond mruby's core: the extensions a Ruby developer reaches for first, each answering a need GDScript meets with a method of its own, so a game neither keeps stand-ins nor writes around them. What would reach the host, such as files, time or the process, is left to Godot's API.

## Includes

- `godot/test/unit/core_library/**/*_test.rb`

## `RJ-001` A string answers Ruby's string methods

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby asks a String `start_with?`, `end_with?`, `strip` or `chars` |
| Then | each answers as Ruby's does, as GDScript's `begins_with`, `ends_with` and `strip_edges` answer |

## `RJ-002` An array answers Ruby's array methods

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby asks an Array `uniq`, `compact`, `flatten`, `dig` or `fetch` |
| Then | each answers as Ruby's does |

## `RJ-003` A collection answers Ruby's enumerable methods

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby asks a collection `sum`, `min_by`, `sort_by`, `group_by`, `each_slice` or `count` |
| Then | each answers as Ruby's does, as GDScript's `count`, `min` and `sort_custom` answer |

## `RJ-004` An iterator without a block answers an enumerator

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby calls `each_with_index` or `times` without a block and maps what it answers |
| Then | each answers as Ruby's does |

## `RJ-005` A hash answers Ruby's hash methods

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby asks a Hash `fetch`, with or without a default, `dig` or `transform_values` |
| Then | each answers as Ruby's does, as GDScript's `Dictionary.get` with a default answers |

## `RJ-006` A comparable value answers `clamp`

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby asks an Integer or a Float `clamp` |
| Then | it answers as Ruby's does, as GDScript's `clamp` answers |

## `RJ-007` A range answers Ruby's range methods

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby asks a Range `first` or `last` with a count, or `size` |
| Then | each answers as Ruby's does |

## `RJ-008` Kernel converts text as Ruby does

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby converts text with `Integer` or `Float` |
| Then | each answers the number, or raises `ArgumentError` for text naming none, as GDScript's `int` and `float` convert |

## `RJ-009` `Struct` makes a class of named members

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby makes a class with `Struct.new` and an object of it |
| Then | the object answers each member it was made with |

## `RJ-010` `Data` makes a class of values never changed

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby makes a class with `Data.define` and a value of it by its members' names |
| Then | the value answers each member, has no writer for one, and answers them as a Hash |

## `RJ-011` `Set` holds each element once

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby makes a `Set` of elements, some given twice |
| Then | it holds each once and answers whether it includes one |
