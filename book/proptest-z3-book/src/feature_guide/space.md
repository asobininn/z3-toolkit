# Spaces and Holes

A **hole** is a blank that will be filled with a name later. Z3 decides which name goes into each hole based on the constraints. This section explains namespaces (`Space`), which define the groups of names a hole can draw from, and the information carried by each hole.

## Define namespaces

A namespace is an independent set of names. Create a separate one for each category of name that is resolved independently, such as variables and functions.

```rust
const VAR: Space = Space::new("var");
const FUNC: Space = Space::new("func");
```

`Space::new()` takes an `&'static str`. This string is used as-is in diagnostic output, including counterexamples.

The same string name in different namespaces has no relationship to the other.

## Hole roles

Each hole has a namespace and a **role** (`Role`). There are three roles:

| Role | How to create it | Meaning |
| --- | --- | --- |
| `Role::Def` | `VAR.def()` | Define a name |
| `Role::Use` | `VAR.refer()` | Reference a name |
| `Role::Kill` | `VAR.kill()` | Delete a name; it is no longer visible afterward |

A `Use` and a `Kill` hole are collectively called a **reference**, because both point to an existing name. The [Binds and Rules](./binds.md) section explains how to handle references together in constraints.

`def()`, `refer()`, and `kill()` each return a Strategy for a `Frag` containing one hole. To select the role dynamically, use `hole(role)`.

```rust
VAR.hole(Role::Kill) // Same as VAR.kill()
```

## Example: deletion and functions

Extend the language from Getting Started with `del x` and function declarations/calls such as `fn f { ... }` and `call f`. A `Kill` hole represents the name in `del x`. A function declaration introduces a `Def`, and a call introduces a `Use`. This lets the same visibility rules express variable references, function calls, and use-after-deletion errors.
