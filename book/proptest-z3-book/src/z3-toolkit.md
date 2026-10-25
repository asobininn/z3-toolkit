# Writing Constraints with z3-toolkit

proptest-z3 constraints are written with z3-toolkit's `constraint!` macro. This chapter covers only the parts used by proptest-z3. `proptest_z3::prelude::*` re-exports the z3-toolkit prelude, so you do not need to add a separate import.

## `constraint!` builds one `z3::ast::Bool`

`constraint!(...)` turns the conditions inside it into a single Z3 `Bool`. Since a rule passed to `fill` must return a `Bool`, the body of a rule will generally use `constraint!(...)`.

```rust
program().fill(names(), |binds| constraint!(
    bound(binds, FUNC),
    !distinct_defs(binds, VAR),
))
```

## Writing conditions

| Syntax | Meaning |
| --- | --- |
| `all { p, q, .. }` | All conditions hold |
| `any { p, q, .. }` | At least one condition holds |
| `exactly(n) { .. }` | Exactly `n` conditions hold |
| `at_most(n) { .. }` / `at_least(n) { .. }` | At most / at least `n` conditions hold |
| `!p`, `p && q`, `p \|\| q` | Not, and, or |
| `{ p } implies { q }` | If `p`, then `q` |
| `distinct(xs)` | All values are distinct |

## Expanding with `for` and `if`

Inside `constraint!`, `for` and `if` can expand into per-item conditions. The following example means that exactly one reference to a `VAR` is not visible.

```rust
constraint!(
    exactly(1) {
        for i in binds.refs(VAR) {
            !visible(binds, i)
        }
    },
)
```

A `for` body can contain multiple conditions; each one contributes separately to the count. An `if` whose condition is a Rust `bool` is evaluated while building the constraint, so it can be used to select which terms to include.

For the complete syntax and more examples, see the [z3-toolkit documentation](https://github.com/asobininn/z3-toolkit).
