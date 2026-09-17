# Announcement

How a node script is announced to the editor, which lists it as a class like a GDScript's `class_name`, read from headers and ancestries without running any file.

## Includes

- `rust/src/announcement.rs`

## `RN-001` A node script is announced by its class's name without its namespaces

| Step | Statement |
| --- | --- |
| Given | a node script whose class sits inside a namespace |
| When | it is announced |
| Then | the announcement's name is the class's own name |

## `RN-002` A node script inheriting from no announced file is announced with its engine class as base

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends an engine node class |
| When | it is announced |
| Then | the announcement's base is that engine class |

## `RN-003` A node script extending an announced class is announced with that class as base

| Step | Statement |
| --- | --- |
| Given | a node script whose class extends another node script's class |
| When | it is announced |
| Then | the announcement's base is the other class's name |

## `RN-004` A file under a test directory is not announced

| Step | Statement |
| --- | --- |
| Given | a node script under a test directory |
| When | it is announced |
| Then | there is no announcement |

## `RN-005` A library file is not announced

| Step | Statement |
| --- | --- |
| Given | a file whose class extends no engine node class |
| When | it is announced |
| Then | there is no announcement |

## `RN-006` Node scripts sharing a name are not announced

| Step | Statement |
| --- | --- |
| Given | two node scripts in different namespaces whose classes share a name |
| When | one of them is announced |
| Then | there is no announcement, naming the other file |

## `RN-007` A base sharing its name is passed over for the next

| Step | Statement |
| --- | --- |
| Given | a node script extending another node script's class, whose name a third node script shares |
| When | the first is announced |
| Then | the announcement's base is the engine class |

## `RN-008` An announcement carries whether the class is a tool

| Step | Statement |
| --- | --- |
| Given | a node script whose class body calls `tool` |
| When | it is announced |
| Then | the announcement says the class is a tool |

## `RN-009` An announcement carries whether the class is abstract

| Step | Statement |
| --- | --- |
| Given | a node script whose class body calls `abstract` |
| When | it is announced |
| Then | the announcement says the class is abstract |

## `RN-010` An announcement carries the class's icon

| Step | Statement |
| --- | --- |
| Given | a node script whose class body calls `icon` with a path |
| When | it is announced |
| Then | the announcement carries that path |
