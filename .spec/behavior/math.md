# Math

How Ruby computes in a realm: Ruby's own `Math`, `rand` and `srand` behave as Ruby's do, so a Ruby developer writes the math they know, and the engine's game math, random numbers and error reports that Ruby lacks are module functions of `Godot`, behaving as the engine's own, so neither side's behaviour is changed by the other.

## Includes

- `godot/test/unit/math/**/*.rb`
- `tasks/support/godot.rb`

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

## `RC-003` Godot's game math is a module function of Godot

| Step | Statement |
| --- | --- |
| Given | the engine's interpolation, stepping, wrapping and clamping functions |
| When | Ruby calls each under `Godot` |
| Then | each answers what the engine's function answers |

## `RC-004` Godot's random numbers come from the engine's generator

| Step | Statement |
| --- | --- |
| Given | the engine's generator seeded with a number through `Godot.seed` |
| When | Ruby draws a number through `Godot.randf` |
| Then | it gets the number the engine's generator gives GDScript after the same seed |

## `RC-005` Ruby's own random numbers are a stream of their own

| Step | Statement |
| --- | --- |
| Given | Ruby's generator seeded with `srand` |
| When | the engine's generator is seeded through `Godot.seed` before Ruby draws with `rand` |
| Then | Ruby's `rand` draws what it would have drawn had the engine's not been seeded |

## `RC-006` Godot.push_error and Godot.push_warning report to the engine

| Step | Statement |
| --- | --- |
| Given | a node script calling `Godot.push_error` and `Godot.push_warning` with a message |
| When | the scene runs |
| Then | the log carries each message as an error and as a warning |

## `RC-007` Godot.is_same compares as the engine does

| Step | Statement |
| --- | --- |
| Given | an engine object, and two equal values of a value type |
| When | Ruby compares the object with itself, and the values with each other, through `Godot.is_same` |
| Then | each answers true, as the engine does |
