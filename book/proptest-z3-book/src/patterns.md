# Common Patterns

This chapter presents constraint patterns that are useful when testing name resolution.

## Name-resolution boundary cases

A randomly generated valid program rarely hits situations where a resolver is likely to be wrong. Constrain the input to be valid with `bound`, then use `any` to ensure that a tricky situation occurs.

### A same-named definition outside the current statement

```rust,ignore
let x
let x = x       // The right-hand side should refer to x from the first line.
```

A resolver that registers the left-hand `x` before resolving the right-hand side may incorrectly bind the right-hand side to itself.

```rust,ignore
program().fill(names(), |binds| constraint!(
    bound(binds, VAR),
    any { for i in binds.uses(VAR) { defined_in_same_step(binds, i) } },
))
```

Because `bound` requires every reference to be visible, and the same step contains a definition with the same name, the right-hand side must resolve to the outer definition.

### Shadowing

```rust,ignore
let x
{
    let x
    print x     // Should refer to the inner x.
}
```

The outer `x` has the same name, so a resolver that does not respect lexical scopes may choose the outer definition.

```rust,ignore
/// Whether the definition resolved by reference `i` hides a same-named definition in an outer scope.
fn hides_outer(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .flat_map(|k| reaching_defs(binds, i).map(move |q| (k, q)))
        .map(|(k, q)| constraint!(
            resolves_to(binds, i, k),
            binds.encloses(q, i),
            !binds.encloses(k, q),
            binds.name[k] == binds.name[q],
        ))
        .collect();
    constraint!(any { for c in cases { c } })
}
```

Use a rule like this to ensure that a reference resolves to an inner definition while a same-named outer definition also exists.

## Deliberately generate an undefined reference

To exercise the error path, require exactly one invisible reference and mark it. See [Labels](./feature_guide/labels.md) for retrieving the marked hole and mapping it to a source span.

```rust,ignore
program().fill(names(), |binds| constraint!(
    exactly(1) {
        for i in binds.refs(VAR) {
            binds.mark(i, Bug::Unbound, !visible(binds, i))
        }
    },
))
```

## Deleted names

To test use-after-deletion, ensure a `Kill` and a later reference share a name, with no later definition restoring visibility. Since `Kill` crosses scope boundaries, the shape of the generated program matters: the deletion must occur before the reference in evaluation order.

```rust,ignore
constraint!(
    any {
        for i in binds.refs(VAR) {
            killed(binds, i)
        }
    },
)
```

## Captures

Use `captured` to select references that resolve across a boundary, such as a closure body. Ordinary lexical blocks do not count as boundaries. To test a closure that relays a capture through another closure, require a reference whose target is separated from it by two or more boundaries.

For the exact semantics of evaluation order, scopes, and deletion, see the [API Reference](./api.md).
