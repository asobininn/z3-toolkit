# Binds and Rules

Relationships between names are expressed as **rules**. A rule is a function that takes `&Binds` and returns a Z3 `Bool`; it is passed to `fill`.

```rust
program().fill(names(), |binds| bound(binds, VAR))
```

This section describes `Binds`, the building blocks for rules, and predicates for common relationships.

## Bind: hole indices

`Binds` lists the holes in a skeleton in [evaluation order](./frag.md#steps). Within a rule, a hole is referred to by its evaluation-order index `i`. A smaller index means earlier evaluation, so “a hole before `i`” can be expressed by comparing indices.

Hole indices can be selected by namespace and role. Each method returns indices in evaluation order.

| Syntax | Holes returned |
| --- | --- |
| `binds.defs(S)` | `Def` holes in namespace `S` |
| `binds.uses(S)` | `Use` holes in namespace `S` |
| `binds.kills(S)` | `Kill` holes in namespace `S` |
| `binds.refs(S)` | References (`Use` and `Kill`) in namespace `S` |
| `binds.of(S, role)` | Holes in namespace `S` with the given role |

`binds.name[i]` is the Z3 `Int` representing the name assigned to hole `i`. Its value is a candidate index, so `binds.name[p] == binds.name[q]` means that holes `p` and `q` contain the same name.

```rust
// All VAR definitions must have distinct names.
fn no_redefinition(binds: &Binds) -> Bool {
    constraint!(
        distinct(binds.defs(VAR).map(|i| &binds.name[i])),
    )
}
```

## Information known before solving

Some relationships are properties of the generated structure and are already known before Z3 solves the name assignments. Methods such as `same_step`, `encloses`, and `boundaries_between` return Rust values (`bool` or `usize`), not Z3 expressions. Use them to filter candidates or branch while building a constraint.

## Common predicates

The `rules` module provides helpers for common name-resolution relationships:

- `visible(binds, i)`: the reference at `i` resolves to a visible, non-deleted definition.
- `killed(binds, i)`: the referenced name has a visible definition, but it has been deleted.
- `bound(binds, space)`: every reference in the namespace resolves to a visible definition.
- `unbound(binds, space, n)`: exactly `n` references in the namespace are not visible.
- `defined_in_same_step(binds, i)`: a definition with the same namespace and name exists in the same step, regardless of evaluation order.
- `distinct_defs(binds, space)`: all definitions in the namespace have different names, across the entire skeleton.
- `resolves_to(binds, i, k)`: reference `i` resolves specifically to definition `k`.
- `captured(binds, i)`: reference `i` resolves to a definition outside at least one boundary enclosing the reference.

See the [API Reference](../api.md) for exact semantics, including how lexical scopes and `Kill` interact.
