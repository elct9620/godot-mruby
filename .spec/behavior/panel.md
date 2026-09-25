# Test panel

How the editor runs a project's Ruby tests: the test panel plays the test runner's own scene on the test directory chosen, lists what did not pass from the run's results, and opens the Ruby line a failure names.

## Includes

- `tasks/support/panel.rb`

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
