# Labels

Rules can attach **labels** to generated inputs. Labels serve three purposes:

- Record which hole was broken, and how (`mark`).
- Classify generated inputs without adding a constraint (`observe`).
- Name constraints so that rejection causes can be investigated (`require`).

Any type implementing `Copy + Debug + PartialEq` can be used as a label.

```rust
/// Kinds of violations introduced by a rule.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Bug {
    Unbound, // Reference to an undefined name
    Killed,  // Reference to a deleted name
}

/// Classifications of generated inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Redefined, // A name is defined more than once
}
```

## `mark`: record a broken location

`binds.mark(i, kind, cond)` returns `cond` unchanged and records that hole `i` is broken in way `kind` whenever `cond` holds.

```rust
|binds| constraint!(
    exactly(1) {
        for i in binds.refs(VAR) {
            binds.mark(i, Bug::Unbound, !visible(binds, i))
        }
    },
)
```

After solving, `s.marks::<Bug>()` returns a sequence of `(hole, label)` pairs. If exactly one hole is marked, a helper can make retrieving it convenient:

```rust
fn the_mark(s: &Solved) -> (Hole, Bug) {
    let [(hole, bug)] = s.marks::<Bug>()[..] else {
        panic!("expected exactly one mark");
    };
    (hole, bug)
}
```

`mark` is a record, not a constraint by itself. Its returned `Bool` must be included in the constraint (for example, inside `exactly(1)`) to enforce the condition.

## `observe`: classify without constraining

`binds.observe(kind, cond)` records a classification when `cond` is true, but does not constrain which inputs can be generated. Use it to inspect the distribution of generated cases without changing that distribution.

## `require`: name a constraint

`binds.require(label, constraint)` makes the condition mandatory and associates it with a name. If the combined constraints are unsatisfiable, the label can appear in Z3's unsat core, helping identify why generation was rejected.

Use `require` to split a complicated rule into named parts when diagnosing repeated rejections.

## Statistics and warnings

proptest-z3 tracks label statistics per thread. At the end of a test thread, labels that were declared but never occurred produce a warning, even if the test passes. This can reveal a supposedly covered violation that was never actually generated.

If one violation necessarily implies another, use `given` to state the precondition for classifying each one. This lets the two categories be generated and counted independently.
