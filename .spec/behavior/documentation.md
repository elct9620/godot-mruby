# Documentation

How the editor's help documents a Ruby class: the language says it documents its scripts, and a script's page is read from its comments as RDoc reads them, the last block of comment lines above the class or a method, signal or export, with no statement between, and named as the editor lists the class.

### What it does not see

- A comment above an outer namespace documents that namespace, not the class inside it, as in RDoc.

## Includes

- `rust/src/documentation.rs`
- `tasks/support/documentation.rb`

## `RZ-001` The class comment's first paragraph is its brief and the rest its description

| Step | Statement |
| --- | --- |
| Given | a class whose comment has two paragraphs |
| When | its documentation is read |
| Then | the first paragraph is its brief description and the second its description |

## `RZ-002` A method takes the last comment block above it with no statement between

| Step | Statement |
| --- | --- |
| Given | methods with a comment a blank line above, two comment blocks above, a statement between, and `:nodoc:` on the `def` line |
| When | the class's documentation is read |
| Then | each takes its nearest block, the one after a statement takes none, and the `:nodoc:` one is left out |

## `RZ-003` A signal takes the comment above its call

| Step | Statement |
| --- | --- |
| Given | a `signal` call with a comment above it |
| When | the class's documentation is read |
| Then | the signal is documented by that comment |

## `RZ-004` A magic comment is no documentation

| Step | Statement |
| --- | --- |
| Given | a class whose only comment above it is `# frozen_string_literal: true` |
| When | its documentation is read |
| Then | it has none |

## `RZ-005` The editor's help shows a Ruby class's comment on its page

| Step | Statement |
| --- | --- |
| Given | a node script whose class has a comment, handed to the editor's help as the script editor does on saving |
| When | its class's page is opened |
| Then | the page shows the comment |

## `RZ-006` The editor's help shows a Ruby method's comment on its class's page

| Step | Statement |
| --- | --- |
| Given | a node script whose method has a comment, handed to the editor's help as the script editor does on saving |
| When | its class's page is opened |
| Then | the page shows the method's comment |

## `RZ-007` The editor's help documents a Ruby script its scan finds

| Attribute | Value |
| --- | --- |
| unverifiable | a headless editor's scan registers no script's documentation, a GDScript's included |

| Step | Statement |
| --- | --- |
| Given | a Ruby node script with comments, in a project the editor opens |
| When | the editor's scan has found the script and its class's page is opened |
| Then | the page shows the comments |
