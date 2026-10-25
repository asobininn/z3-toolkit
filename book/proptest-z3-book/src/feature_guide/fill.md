# fill and Solved

`fill` combines a skeleton Strategy, candidate names, and a rule to produce a Strategy for inputs with their holes filled. Generated values are `Solved`.

```rust
proptest! {
    #[test]
    fn defined_names_resolve(
        s in program().fill(names(), |binds| bound(binds, VAR))
    ) {
        prop_assert_eq!(undefined_refs(&s.text), vec![], "{:?}", s);
    }
}
```

## Choose a solving mode

Use `fill_with` to choose how holes are filled. By default, `fill` uses `Fill::Greedy`.

| Mode | Behavior |
| --- | --- |
| `Fill::Greedy` (default) | Treat hints as assumptions. If they conflict with constraints, discard the last conflicting hint and solve again. |
| `Fill::Optimal` | Maximize the number of hints that can be honored. |

Both modes produce the same result from the same skeleton and hints. Re-solving the same input during shrinking is deterministic.

- `Fill::Greedy` is fast. However, its choice of which hint to discard is not globally optimal, so it may discard more hints than `Fill::Optimal`.
- `Fill::Optimal` tries to honor hints as much as possible, but can be slower for skeletons with many holes.

Use `Fill::Greedy` by default, and try `Fill::Optimal` if the generated names are distributed in an undesirable way.

## Shrinking

When a test fails, proptest shrinks its input. proptest-z3 shrinks the structure, hints, and candidate names, and only tries inputs that satisfy the rule. If a shrunken input is unsatisfiable, that shrink direction is abandoned.

Shrinking does not necessarily reach a fixed point, so statements unrelated to the counterexample may remain in the final input.
