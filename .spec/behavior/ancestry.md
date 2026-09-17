# Ancestry

How a file's ancestry is read from the headers of the files its class inherits from, without running any of them, so a class extending an engine class through other files is known as a node script before it runs.

## Includes

- `rust/src/ancestry.rs`

## `RI-001` A superclass is found in the innermost namespace first

| Step | Statement |
| --- | --- |
| Given | a file whose class inside a namespace extends a name that a file in that namespace and a top-level file both spell |
| When | its ancestry is read |
| Then | the ancestry's nearest file is the one in that namespace |

## `RI-002` A superclass the innermost namespace lacks is found further out

| Step | Statement |
| --- | --- |
| Given | a file whose class inside a namespace extends a name only a top-level file spells |
| When | its ancestry is read |
| Then | the ancestry's nearest file is the top-level one |

## `RI-003` A superclass written inside a namespace is found in that namespace

| Step | Statement |
| --- | --- |
| Given | a top-level file whose class extends a constant path spelling a namespace and a name a file in it spells |
| When | its ancestry is read |
| Then | the ancestry's nearest file is that file |

## `RI-004` An ancestry ends at the engine class its farthest file extends

| Step | Statement |
| --- | --- |
| Given | a file whose class extends another file's class, which extends a `Godot::` class |
| When | its ancestry is read |
| Then | the ancestry carries that engine class |

## `RI-005` A superclass no file names breaks the ancestry

| Step | Statement |
| --- | --- |
| Given | a file whose class extends a name no file spells |
| When | its ancestry is read |
| Then | the ancestry is broken at that name |

## `RI-006` A superclass two files name breaks the ancestry

| Step | Statement |
| --- | --- |
| Given | a file whose class extends a name two files spell |
| When | its ancestry is read |
| Then | the ancestry is broken at that name |

## `RI-007` An ancestry that comes back to a file it passed is broken

| Step | Statement |
| --- | --- |
| Given | two files whose classes extend each other |
| When | the ancestry of one is read |
| Then | the ancestry is broken as a cycle |

## `RI-008` An ancestry that reaches no engine class is broken

| Step | Statement |
| --- | --- |
| Given | a file whose class extends another file's class, which writes no superclass |
| When | its ancestry is read |
| Then | the ancestry is broken for reaching no engine class |
