# Math

How Ruby computes in a realm: Ruby's own `Math` behaves as Ruby's does, so a Ruby developer writes the math they know.

## Includes

- `godot/test/math/**/*.rb`

## `RC-001` Ruby has Math

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby calls `Math.sin` and `Math.sqrt` |
| Then | each answers its Float result |

## `RC-002` A value outside a function's domain raises Math::DomainError

| Step | Statement |
| --- | --- |
| Given | a realm |
| When | Ruby calls `Math.sqrt` with a negative number |
| Then | it raises `Math::DomainError` |
