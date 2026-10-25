# Frag and Skeleton

Arrange holes and text to define the structure of an input. A structure under construction is called a `Frag` (fragment); the completed structure is a `Skeleton`.

- `Frag`: A sequence of text and holes. Fragments can be joined with `frag!` or `Frag::concat`.
- `Skeleton`: A sequence of `Frag` values divided into **steps** by `Skeleton::steps`.

## Join pieces with `frag!`

`frag!` concatenates its arguments in order into one `Frag`.

```rust
(VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!("let ", x, " = ", y))
```

The following values can be passed to `frag!`:

| Value | Example |
| --- | --- |
| String | `"let "` |
| Number | `42` |
| `Frag` containing holes | `x`, `frag![..]` |
| `Vec<Frag>` | Concatenates elements in order |
| `Option<Frag>` | Adds nothing when `None` |
| Tuple (up to four elements) | Concatenates elements in order |
| `Text(x)` | A user-defined type implementing `Display` |

Wrap a user-defined type in `Text` to insert its `Display` output as text.

```rust
#[derive(Debug, Clone, Copy)]
enum Op { Add, Sub }

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Op::Add => "+",
            Op::Sub => "-",
        })
    }
}

(VAR.refer(), prop_oneof![Just(Text(Op::Add)), Just(Text(Op::Sub))])
    .prop_map(|(x, op)| frag!("let a = ", x, " ", op, " b"))
```

## Steps

A step is a unit at which definitions become visible; it corresponds roughly to a statement in a language.

Only `Skeleton::steps`, `Frag::scope`, and `Frag::boundary` create steps. Each element passed to these functions is closed as a separate step. `frag!`, `Frag::concat`, and `Frag::join` do not create steps. Concatenated fragments become one step when the surrounding construct closes them.

This distinction matters for statements such as `let x = x`: the right-hand-side reference is evaluated before the definition on the left becomes visible.

## Scopes

`Frag::scope(steps)` creates a nested lexical scope. Each element in `steps` becomes a step in that scope. Definitions inside the scope are visible there and in nested scopes, but not outside it or in sibling scopes. Definitions from outer scopes remain visible inside, unless shadowed by a same-named inner definition.

The scope's delimiters and whitespace are ordinary text and must be supplied by the caller.

## Boundaries

`Frag::boundary(steps)` creates a scope marked as a boundary, suitable for a closure or function body. Name visibility is the same as for `Frag::scope`; the difference is that the number of boundaries between a reference and its target definition can be counted to detect captures.

Ordinary scopes do not count as boundaries. Blocks and boundaries can be nested freely.
