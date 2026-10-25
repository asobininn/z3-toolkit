# proptest-z3

[![crates.io](https://img.shields.io/crates/v/proptest-z3.svg)](https://crates.io/crates/proptest-z3)
[![docs.rs](https://docs.rs/proptest-z3/badge.svg)](https://docs.rs/proptest-z3/)
[![Book](https://img.shields.io/badge/book-English-blue.svg)](https://asobininn.github.io/z3-toolkit/)
[![Book(日本語版)](https://img.shields.io/badge/book-日本語-blue.svg)](https://asobininn.github.io/z3-toolkit/ja/)
[![z3-toolkit](https://img.shields.io/badge/with-z3--toolkit-orange.svg)](https://crates.io/crates/z3-toolkit)

proptest-z3 is an extension of [proptest](https://crates.io/crates/proptest) for generating test inputs whose elements depend on each other.
You define the structure of an input with ordinary proptest Strategies, and specify the relations between elements with [Z3](https://crates.io/crates/z3) constraints.
By flipping a constraint, you can generate inputs that contain a given number of violations, and you can also locate where the violations are. The relations between elements are kept during shrinking as well.

## Setup

```toml
[dev-dependencies]
proptest = "1"
proptest-z3 = "0.1"
```

The Z3 library is required. For how to install it and how to configure linking, see the [README of the z3 crate](https://github.com/prove-rs/z3.rs).

## Usage

### Building inputs with dependencies

As an example, we use a small language with one statement per line.

```text
let x        // define x
let x = y    // use y, then define x
print x      // use x
```

What we test is the name resolver of this language, `undefined_refs(src) -> Vec<usize>`. It returns the positions (in bytes) where undefined names are referred to.

Where a name goes, put a hole instead of an actual name. `def` is a definition and `refer` is a reference. Which name goes into which hole is decided by Z3, so you do not need to track names inside the generator.

```rust
use proptest::prelude::*;
use proptest_z3::prelude::*;

const VAR: Space = Space::new("var");

fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8)
        .prop_map(|stmts| Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"])))
}

fn stmt() -> impl Strategy<Value = Frag> {
    prop_oneof![
        VAR.def().prop_map(|x| frag!["let ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["let ", x, " = ", y]),
        VAR.refer().prop_map(|y| frag!["print ", y]),
    ]
}

// Name candidates: 3 mutually distinct names matching [a-c]
fn names() -> NameSpec {
    NameSpec::new().space(VAR, "[a-c]", 3)
}

proptest! {
    // Every reference points to a visible name
    #[test]
    fn defined_names_resolve(s in program().fill(names(), |binds| bound(binds, VAR))) {
        prop_assert_eq!(undefined_refs(&s.text), vec![], "{:?}", s);
    }
}
```

### Generating negative inputs by changing the constraint

All you change is the constraint passed to `fill`.

```rust
// positive case
program().fill(names(), |binds| bound(binds, VAR))

// negative case: exactly one reference points to a name that is not visible
program().fill(names(), |binds| constraint!(
    exactly(1) { for i in binds.refs(VAR) {
        binds.mark(i, Undefined, !visible(binds, i))
    } },
))
```

Holes marked with `mark` can be located after the constraints are solved. This lets you check that the name resolver reports the violation at the right place.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
struct Undefined;

proptest! {
    #[test]
    fn undefined_name_is_reported_where_it_is(s in program().fill(names(), |binds| constraint!(
        exactly(1) { for i in binds.refs(VAR) {
            binds.mark(i, Undefined, !visible(binds, i))
        } },
    ))) {
        let [(hole, Undefined)] = s.marks::<Undefined>()[..] else {panic!("{s:?}")};
        prop_assert_eq!(undefined_refs(&s.text), vec![s.span(hole).start], "{:?}", s);
    }
}
```

If the name resolver has a bug that defines the left-hand side before checking the right-hand side in `let x = y`, it fails as follows.

```text
running 2 tests
test undefined_name_is_reported_where_it_is ... FAILED
test defined_names_resolve ... ok

minimal failing input: s = Solved{
  0| let b
  1| let a = a
 marks: Undefined @ 14..15 `a`
}
```

Even when the positive tests pass, bugs that accept inputs which should be errors can remain. By also generating negative inputs, you can detect bugs like these.

### Building different kinds of breakage

You can also decide per namespace whether to break it, and build violations separately by kind. In the example below, redefinition of variables is allowed, and exactly one of the references has one of the violations "use after deletion", "self reference" or "undefined reference".

```rust
program().fill(names(), |binds| {
    constraint!(
        // define variables of the same name again
        !&distinct_defs(binds, VAR),
        // every reference to a function is valid
        bound(binds, FUNC),
        // exactly one of the references to variables is broken in one of three ways
        exactly(1) { for i in binds.refs(VAR) { any {
            binds.mark(i, Broken::Delete, killed(binds, i)),
            binds.mark(i, Broken::SelfRef, constraint!(all {
                !visible(binds, i),
                defined_in_same_step(binds, i),
            })),
            binds.mark(i, Broken::Undefined, (!visible(binds, i)).given([
                !killed(binds, i),
                !defined_in_same_step(binds, i),
            ])),
        } } },
    )
})
```

When violation conditions overlap, or one violation always comes with another, some kinds may not be generated as intended, for example when `given` is removed in the example above. In that case, even if the test passes, the labels that never appeared are warned about.

```text
proptest-z3 warning: label `Delete` never occurred in 4 cases (test `each_kind_is_reported`); is it implied by another violation? consider `given`
```

By stating the preconditions explicitly with `given`, the violations can be made independent of each other.

You can also do the following. See the [Book](..) for details.

- nested scopes with blocks
- getting which definition each reference points to
- tallying the causes when generation is mostly rejected

## Scope

proptest-z3 is a crate for small test inputs that contain definitions and references. A "name" in this crate broadly means anything that is created and then pointed to from another place, such as variable names, table names and row keys.

- The only relation it can handle is equality between names. Conditions on types or values can be added with `constraint!`, but there is no feature yet for letting Z3 choose the values themselves, such as integers.
- Scopes are limited to lexical nesting, such as blocks.
- The structure of an input is decided by proptest, so generation is rejected when the structure cannot satisfy the constraints. When rejections are frequent, provide structures that can satisfy the conditions on the Strategy side.
- The more holes there are, the longer it takes to solve the constraints.
