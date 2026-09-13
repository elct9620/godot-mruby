# Loading by name

How a file's constants reach the Ruby that uses them without `require`: a realm's class index names every file after its path, and the first use of a constant runs the file that names it.

## Includes

- `tasks/support/loading.rb`
- `godot/test/**/*.rb`

## `RL-001` Two files naming one constant are warned about

| Step | Statement |
| --- | --- |
| Given | two files whose paths spell one constant, apart from an underscore |
| When | a realm's class index takes them in |
| Then | Godot's log warns that neither of them loads by name |

## `RL-002` One name at two namespace levels is warned about

| Step | Statement |
| --- | --- |
| Given | a file whose name another file inside a namespace below it shares |
| When | a realm's class index takes them in |
| Then | Godot's log warns that the outer one hides the inner one once it has loaded |

## `RL-003` A file naming a constant the realm already has is warned about

| Step | Statement |
| --- | --- |
| Given | a file whose path spells a constant mruby already defines |
| When | a realm's class index takes it in |
| Then | Godot's log warns that it never loads by name |
