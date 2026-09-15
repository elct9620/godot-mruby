# Loader

How a file's constants reach the Ruby that uses them without `require`: a realm's class index names every file after its path, and the loader runs the file that names a constant at the constant's first use.

## Includes

- `tasks/support/loader.rb`
- `godot/test/loader/**/*.rb`

## `RL-001` Two files naming one constant are warned about

| Step | Statement |
| --- | --- |
| Given | two files whose paths spell one constant, apart from an underscore |
| When | a realm's class index takes them in |
| Then | the log carries a warning that neither of them loads by name |

## `RL-002` One name at two namespace levels is warned about

| Step | Statement |
| --- | --- |
| Given | a file whose name another file inside a namespace below it shares |
| When | a realm's class index takes them in |
| Then | the log carries a warning that the outer one hides the inner one once it has loaded |

## `RL-003` A file naming a constant the realm already has is warned about

| Step | Statement |
| --- | --- |
| Given | a file whose path spells a constant mruby already defines |
| When | a realm's class index takes it in |
| Then | the log carries a warning that it never loads by name |

## `RL-004` A constant a file names loads at its first use

| Step | Statement |
| --- | --- |
| Given | a file under `res://` defining the constant its path spells |
| When | Ruby first uses that constant |
| Then | it gets what the file defined |

## `RL-005` A namespace's own file runs before a file inside it

| Step | Statement |
| --- | --- |
| Given | a file defining a namespace, and a file inside its directory that reopens it to define a class |
| When | Ruby first uses that class |
| Then | the class reaches what the namespace's own file defined |

## `RL-006` A directory with no file of its own is an empty module

| Step | Statement |
| --- | --- |
| Given | a directory holding a file but no file of the directory's own name |
| When | Ruby first uses the constant the directory spells |
| Then | it gets a module holding the constants of the files inside it |

## `RL-007` A directory's module answers only to the name Zeitwerk gives it

| Step | Statement |
| --- | --- |
| Given | a directory with no file of its own name, whose name Zeitwerk camelizes one way |
| When | Ruby uses the constant spelled another way |
| Then | it raises `NameError` naming the spelling the directory has |

## `RL-008` A name used inside a namespace finds that namespace's file first

| Step | Statement |
| --- | --- |
| Given | a file inside a namespace's directory, and code inside that namespace using its name alone |
| When | the code runs |
| Then | it gets the constant of the file inside the namespace |

## `RL-009` A name no file spells raises the NameError core Ruby raises

| Step | Statement |
| --- | --- |
| Given | a constant no file under `res://` spells |
| When | Ruby uses it |
| Then | it raises `NameError` with core Ruby's message, naming the constant |

## `RL-010` A qualified name its namespace lacks is looked for outward

| Step | Statement |
| --- | --- |
| Given | a file in a namespace's directory, and a namespace inside it whose directory has no file of that name |
| When | Ruby uses the name qualified by the inner namespace |
| Then | it gets the constant of the file in the outer namespace |

## `RL-011` A node script's namespace file runs before it

| Step | Statement |
| --- | --- |
| Given | a node script inside a namespace's directory that reopens the namespace, and the namespace's own file |
| When | the scene runs |
| Then | the script reaches what the namespace's own file defined |

## `RL-012` A file under a test directory loads by name

| Step | Statement |
| --- | --- |
| Given | a file under a test directory that is not a test file |
| When | a test uses the constant its path spells from `res://` |
| Then | it gets what the file defined |

## `RL-013` A test class loaded by name during a run does not run

| Step | Statement |
| --- | --- |
| Given | a file defining a test class, which a test loads by name |
| When | the run goes on |
| Then | the loaded class's tests do not run |

## `RL-014` A constant two files spell does not load by name

| Step | Statement |
| --- | --- |
| Given | two files whose paths spell one constant, apart from an underscore |
| When | Ruby uses that constant |
| Then | it raises `NameError` |

## `RL-015` A constant needed while its own file runs raises NameError naming the cycle

| Step | Statement |
| --- | --- |
| Given | two files, each needing the other's constant before defining its own |
| When | Ruby first uses one of the constants |
| Then | it raises `NameError` naming the files in the order they needed each other |

## `RL-016` A file that runs without defining its constant raises NameError

| Step | Statement |
| --- | --- |
| Given | a file that runs cleanly but defines something other than the constant its path spells |
| When | Ruby first uses that constant |
| Then | it raises `NameError` naming the file that did not define it |

## `RL-017` A file that raises takes the constants it created with it

| Step | Statement |
| --- | --- |
| Given | a file that defines its constant and another beside it, then raises |
| When | Ruby uses its constant and the exception is rescued |
| Then | neither constant is defined |

## `RL-018` A file loaded while a failing file ran keeps its constants

| Step | Statement |
| --- | --- |
| Given | a file that loads another file by name, which runs cleanly, and then raises |
| When | Ruby uses the failing file's constant and the exception is rescued |
| Then | the constant of the file it loaded is still defined |

## `RL-019` A file that raised runs again at the next use of its constant

| Step | Statement |
| --- | --- |
| Given | a file that raised the first time its constant was used |
| When | Ruby uses the constant again |
| Then | the file runs again and raises what it raised before |

## `RL-020` A file loaded by name that does not parse raises SyntaxError at its line

| Step | Statement |
| --- | --- |
| Given | a file whose source does not parse |
| When | Ruby first uses the constant its path spells |
| Then | it raises `SyntaxError` naming the file and the line |

## `RL-021` An exception in a file loaded by name is reported at that file's line

| Step | Statement |
| --- | --- |
| Given | a test using a constant whose file raises |
| When | the runner scene runs headless on that directory |
| Then | the log carries the test's error at the line of the file that raised |

## `RL-022` A directory's module made while a failing file ran stays

| Step | Statement |
| --- | --- |
| Given | a file that uses a class inside a directory with no file of its own name, then raises |
| When | Ruby uses the failing file's constant and the exception is rescued |
| Then | the directory's module is still defined |

## `RL-023` The hook that tells the loader of new constants stays private

| Step | Statement |
| --- | --- |
| Given | a realm whose loader learns of each constant a running file creates |
| When | Ruby calls `const_added` on a module from outside it |
| Then | it raises `NoMethodError`, as core Ruby's private hook does |
