# Header

How a file's header is read from its source without running it: the class the file's path names, the superclass written on it, and the names of the methods it defines.

## Includes

- `rust/src/header.rs`

## `RH-001` A class written inside its path's namespaces is the file's class

| Step | Statement |
| --- | --- |
| Given | a file whose class is written inside `module` statements spelling its path's namespaces |
| When | its header is read |
| Then | the header carries the superclass written on that class |

## `RH-002` A class written with its whole constant path is the file's class

| Step | Statement |
| --- | --- |
| Given | a file whose class statement spells the constant its path names in full |
| When | its header is read |
| Then | the header carries the superclass written on that class |

## `RH-003` A class whose name only matches its path apart from underscores and case is the file's class

| Step | Statement |
| --- | --- |
| Given | a file `http_client.rb` defining `HTTPClient` |
| When | its header is read |
| Then | the header carries the superclass written on that class |

## `RH-004` A method of a class nested in the file's class is not the file's

| Step | Statement |
| --- | --- |
| Given | a file whose class holds another class defining a method |
| When | its header is read |
| Then | the header does not carry that method |

## `RH-005` A file that does not parse still has a header

| Step | Statement |
| --- | --- |
| Given | a file whose class and its methods come before a syntax error |
| When | its header is read |
| Then | the header carries the class's superclass and methods |

## `RH-006` A module file has no superclass

| Step | Statement |
| --- | --- |
| Given | a file whose path names a module |
| When | its header is read |
| Then | the header carries no superclass |
