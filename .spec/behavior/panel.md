# Test panel

How the editor runs a project's Ruby tests: the test panel plays the test runner's own scene on the test directory chosen, lists what did not pass from the run's results, and opens the Ruby line a failure names.

## Includes

- `tasks/support/panel.rb`
- `rust/src/panel.rs`

## `RP-001` The panel lists what did not pass in the directory it ran

| Step | Statement |
| --- | --- |
| Given | the editor's test panel with a test directory chosen whose one test fails |
| When | its Run is pressed and the runner scene it plays stops |
| Then | the panel lists the failure by class and name with its message, and counts the run |

## `RP-002` Activating a listed failure opens its Ruby line

| Step | Statement |
| --- | --- |
| Given | a failure the test panel lists |
| When | its row is activated |
| Then | the editor switches to the script editor, showing the test file at the line the failure names |

## `RP-003` The panel runs the test directory chosen, or every one

| Attribute | Value |
| --- | --- |
| refines | `RP-001` |

| Step | Statement |
| --- | --- |
| Given | the panel's choice of a test directory, or of none |
| When | the panel writes the run file |
| Then | the run names that directory, or no directory so every test directory runs |
