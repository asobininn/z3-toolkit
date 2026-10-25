# Appendix: API reference

This lists the public API of proptest-z3 0.1.0, including signatures and semantics (especially the relation to scopes, steps and evaluation order).
It is intended as a supplement to the main text, and as context when having an LLM write tests.

## A.1 Before reading: two kinds of numbers

proptest-z3 has two kinds of numbers that point to holes. Mixing them up leads to wrong tests.

| | Type | Order | Where used |
| --- | --- | --- | --- |
| number in evaluation order | `usize` (variable names `i`, `k`, `j`) | evaluation order (A.2) | inside constraints (`Binds`, `rules`) |
| number in text order | `Hole(pub usize)` | the order they appear in the output text | after solving (`Solved`) |

- Inside a constraint, `i < k` means "`i` is evaluated before `k`". It is not before or after in the text.
- There is no public API for getting a `Hole` from inside a constraint. If you need the position of a hole, put a mark on it with `binds.mark(i, ..)` and receive it as a `Hole` with `s.marks()` after solving.

## A.2 Rules for steps, evaluation order and scopes

### Steps

- A step is "the unit at which definitions become visible", and corresponds to a statement of the language.
- Only three things create steps: `Skeleton::steps(..)` / `Frag::scope(..)` / `Frag::boundary(..)`. Each of them closes each element passed to it as one step.
- `frag!` / `Frag::concat` / `Frag::join` do not create steps. Joined fragments become one step together when they are closed outside.
- Holes inside `Frag::scope(..)` / `Frag::boundary(..)` belong to the inner steps, not to the step of the outer statement.

### Evaluation order

The evaluation order is determined by the following rules.

1. Steps are ordered in text order.
2. Within one step,
   - first: `Use` holes, and holes inside nested scopes contained in that step (in text order)
   - then: the step's own `Def` and `Kill` holes (in text order)

Consequences:

- The right-hand `x` (`Use`) of `let x = x` is evaluated before the left-hand `x` (`Def`) of the same statement, so it does not point to it.
- `del x` (`Kill`) is evaluated after the `Use`s of the same statement.
- When the `Def` of a statement and a scope contained in that statement are placed together, as in `fn f { .. call f .. }`, every hole inside the scope is evaluated before the definition of `f`. **Therefore, by default, recursive calls are not visible.** The same applies to boundaries (`Frag::boundary`).
- The contents of a boundary (the body of a closure) are treated as evaluated at the position where they are written (the same as being called there). A `Kill` inside a boundary also affects references after the boundary.

### Scopes

- `Frag::scope(steps)` creates one nested scope. Scopes are lexical (static) nesting only.
- A `Def` inside a scope is visible only from that scope and the scopes further inside it. It is not visible from outer or sibling scopes.
- An outer `Def` is visible from inside. If the same name is defined inside, that one takes precedence inside (shadowing).
- Scopes are not passed to z3. As Rust-side `bool`s (`Binds::encloses`), they are used to narrow down candidates when building constraints.

### Boundaries (closures and function bodies)

- `Frag::boundary(steps)` creates **a scope marked as a boundary**. Use it for the bodies of closures and functions.
- The visible range (`encloses`, `visible`, `target` and so on) is exactly the same as for `Frag::scope`. Boundaries do not change the result of name resolution.
- The only difference is that you can count how many boundaries a reference crosses to reach its definition (`Binds::boundaries_between`). This is used to decide "captures" (`captured`).
- Ordinary scopes (`Frag::scope`) are not counted when crossed. Blocks inside boundaries and boundaries inside blocks can both be nested freely.

### Whether each API crosses scopes

Legend:

- **lexical**: outer definitions are visible from inside. Definitions in inner and sibling scopes are not visible
- **whole**: does not look at scopes. All holes in the whole skeleton are targets
- **does not cross**: only within the same step (holes in the same step are always in the same scope)
- **—**: unrelated to scopes

| API | Scope | Notes |
| --- | --- | --- |
| `Binds::defs` / `uses` / `kills` / `refs` / `of` | whole | only filters by namespace and role |
| `Binds::same_step` | does not cross | a hole inside a nested scope and a hole of the outer statement containing that scope are in different steps |
| `Binds::encloses` | lexical | this itself is the scope check. Boundaries are treated as ordinary scopes too |
| `Binds::boundaries_between` | lexical | the number of boundaries that enclose `inner` but not `outer`. Ordinary scopes are not counted |
| `reaching_defs` | lexical | among `Def`s in the same namespace earlier in evaluation order, those in a scope enclosing `i` (visible across boundaries too) |
| `kills_between` | **whole** | `Kill` looks at neither scopes nor boundaries (see the note below) |
| `visible` | lexical + `Kill` is whole | a combination of `reaching_defs` and `kills_between` |
| `killed` | lexical + `Kill` is whole | same as above |
| `resolves_to` | lexical + `Kill` is whole | the same rule as `Solved::target`, written as a constraint |
| `captured` | lexical (counts boundaries) | there is at least one boundary between the definition it points to and the reference |
| `bound` / `unbound` | lexical + `Kill` is whole | applies `visible` to each element of `refs` (whole) |
| `defined_in_same_step` | does not cross | the order of evaluation does not matter |
| `distinct_defs` | **whole** | same-name definitions in inner scopes (shadowing) and same names across sibling scopes are also counted as "duplicates" |
| `Solved::target` | lexical + `Kill` is whole | applies the same rule as `visible` to the concrete names after solving |
| `Solved::events` | whole | the evaluation order of the whole skeleton |
| `Binds::mark` / `observe` / `require` | — | |
| `NameSpec` / `Names` | — | candidates are per namespace, not per scope |

**`Kill` crosses scopes.** If there is even one `Kill` of the same name in the same namespace between definition `k` and reference `i` (in evaluation order), `k` is not visible from `i`. Which scope that `Kill` is in does not matter.

- If you `del a` in an inner scope, the outer `a` also disappears. This is the same even when there is an `a` of the same name inside and `del a` points to that one.
- A `del a` inside a closed sibling scope also affects later references to `a`.
- If there is a `Def` of the same name after the `Kill`, that definition is newly visible. However, the range where it is visible is only inside the scope of that `Def`.

**`distinct_defs` crosses scopes.** `!distinct_defs(binds, S)` means "there is a pair of definitions with the same name somewhere", and is not limited to reassignment in the same scope. If you want to forbid or allow only redefinitions within the same scope, narrow down the pairs of definitions with `binds.encloses` and `binds.same_step` and write it yourself with `constraint!`.

### Example

The examples in this appendix use the same small language as the main text (`def x` is a definition, `set x y` uses `y` and defines `x`, `print x` is a use, `del x` is a deletion, `{ .. }` is a scope). The only namespace is `VAR`, and the candidates are `a`, `b`, `c`.

```text
0| def a
1| set b a
2| { def a print a }
3| del a
4| print b
```

| `Hole` in text order | Hole | Role | Scope | Evaluation order `i` |
| --- | --- | --- | --- | --- |
| `Hole(0)` | `a` on line 0 | `Def` | top | 0 |
| `Hole(1)` | `b` on line 1 | `Def` | top | 2 |
| `Hole(2)` | `a` on line 1 | `Use` | top | 1 |
| `Hole(3)` | first `a` on line 2 | `Def` | inner | 3 |
| `Hole(4)` | second `a` on line 2 | `Use` | inner | 4 |
| `Hole(5)` | `a` on line 3 | `Kill` | top | 5 |
| `Hole(6)` | `b` on line 4 | `Use` | top | 6 |

- On line 1, the `Use` `a` (`i = 1`) is evaluated before the `Def` `b` (`i = 2`).
- `reaching_defs(4)` is `{0, 2, 3}`. `Solved::target(Hole(4))` is the inner `Hole(3)` (the last one in evaluation order among the live definitions of the same name).
- `reaching_defs(5)` is `{0, 2}`. The inner `Hole(3)` does not enclose the top-level `Kill`, so it is not included. `target(Hole(5))` is `Hole(0)`.
- `print b` (`i = 6`) points to `Hole(1)`. The `Kill` in between has the name `a`, so it has no effect.
- `distinct_defs(binds, VAR)` is false, because `Hole(0)` and `Hole(3)` are both `a` (the scopes differ, but it counts over the whole).
- The result is the same even if line 3 is `{ del a }` (`Kill` deletes the outer `a` even when it is inside a scope).
- If line 2 is changed to `{ def a del a }`, the inner `del a` points to the inner `a`, but the outer `a` (`Hole(0)`) also disappears at the same time. Later references to `a` at the top level become deleted (`killed`).

### Example: boundaries

Let `[ .. ]` be a boundary (`Frag::boundary`) and `{ .. }` an ordinary scope.

```text
0| def a
1| [ def b [ print a print b ] { print b } ]
```

| `Hole` in text order | Hole | Role | Enclosed by (innermost first) | Evaluation order `i` |
| --- | --- | --- | --- | --- |
| `Hole(0)` | `a` on line 0 | `Def` | none | 0 |
| `Hole(1)` | `b` of `def b` | `Def` | boundary 1 | 1 |
| `Hole(2)` | `a` of `print a` | `Use` | boundary 2, boundary 1 | 2 |
| `Hole(3)` | `b` of `print b` inside boundary 2 | `Use` | boundary 2, boundary 1 | 3 |
| `Hole(4)` | `b` of `print b` inside `{ }` | `Use` | scope, boundary 1 | 4 |

- `Hole(2)` points to `Hole(0)`. `boundaries_between(0, 2) = 2` (boundary 1 and boundary 2), so `captured` is true. It reaches across two boundaries, which means the outer closure relays the capture to the inner one.
- `Hole(3)` points to `Hole(1)`. `boundaries_between(1, 3) = 1` (only boundary 2; boundary 1 also encloses the definition), so `captured` is true.
- `Hole(4)` also points to `Hole(1)`, but there is only an ordinary scope in between, so `boundaries_between(1, 4) = 0` and `captured` is false.

## A.3 Modules and prelude

`use proptest_z3::prelude::*;` brings in the following.

| Origin | Items |
| --- | --- |
| proptest-z3 | `frag!`, `Binds`, `Fill`, `FillExt`, `Filled`, `Frag`, `Given`, `Hint`, `Hole`, `IntoFrag`, `NameSpec`, `Names`, `Role`, `Skeleton`, `Solved`, `Space`, `Text`, `label_stats` |
| `proptest_z3::rules::*` | `visible`, `bound`, `unbound`, `defined_in_same_step`, `killed`, `distinct_defs`, `resolves_to`, `captured`, `reaching_defs`, `kills_between` |
| z3 | `z3::ast::{Bool, Int}` |
| z3-toolkit | `z3_toolkit::prelude::*` (`constraint!`, `Failure` and so on) |

Items not in the prelude, at the crate root: `FilledHole`, `Label`, `LabelStat`, `take_label_stats`, `take_rejections`.

```rust
use proptest::prelude::*;
use proptest_z3::prelude::*;
use proptest_z3::{take_label_stats, take_rejections}; // only when needed
```

## A.4 Building the shape

### `Space` (namespace)

`#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]`. Distinguished by the string inside (`Space`s with the same string are the same namespace).

| Signature | Contents |
| --- | --- |
| `const fn new(name: &'static str) -> Space` | define it as a `const`: `const VAR: Space = Space::new("var");` |
| `fn name(self) -> &'static str` | used in display and panic messages |
| `fn hole(self, role: Role) -> impl Strategy<Value = Frag>` | a `Frag` with one hole. The hint is decided by proptest |
| `fn def(self) -> impl Strategy<Value = Frag>` | `hole(Role::Def)` |
| `fn refer(self) -> impl Strategy<Value = Frag>` | `hole(Role::Use)` |
| `fn kill(self) -> impl Strategy<Value = Frag>` | `hole(Role::Kill)` |

Namespaces are independent of each other. The `a` of `VAR` and the `a` of `FUNC` are unrelated, and every rule only looks at holes in the same namespace.

### `Role` (role of a hole)

| Value | Meaning |
| --- | --- |
| `Role::Def` | defines a name |
| `Role::Use` | refers to a name |
| `Role::Kill` | refers to a name and deletes it at the same time. Afterwards, no definition of the same name is visible (A.2) |

- `fn is_ref(self) -> bool`: true for `Use` or `Kill`. These two are called **references**.
- `Kill` is also a reference, so imposing `bound` means "delete a live name".

### `Hint` (preferred name)

`pub struct Hint(pub u16)`. A fixed-point number from 0 to 1 representing "which of the candidates I want". Implements `Arbitrary`.

- `fn index(self, size: usize) -> usize`: `floor(size × h / 65536)`. Panics if `size == 0`.
- It is kept unless it conflicts with the constraints. Shrinking moves it towards 0, so names move towards the first candidate (the lexicographically smallest, as in A.5).
- Normally `Space::def()` and the like make it automatically, so you only use it directly when calling `Frag::hole` by hand.

### `Frag` (fragment under construction)

`#[derive(Debug, Clone, Default)]`. `Frag::default()` is the empty fragment.

| Signature | Contents | Steps and scopes |
| --- | --- | --- |
| `fn hole(space: Space, role: Role, hint: Hint) -> Frag` | one hole | does not create |
| `fn text(s: impl Into<String>) -> Frag` | text only | does not create |
| `fn concat(parts: impl IntoIterator<Item = Frag>) -> Frag` | joins in order | does not create |
| `fn join(sep: impl IntoFrag + Clone, parts: impl IntoIterator<Item = impl IntoFrag>) -> Frag` | joins with separators | does not create |
| `fn scope(steps: impl IntoIterator<Item = Frag>) -> Frag` | a nested scope. Closes each element as one step | creates steps and a scope |
| `fn boundary(steps: impl IntoIterator<Item = Frag>) -> Frag` | a nested scope marked as a boundary (closure, function body). The visible range is the same as `scope` | creates steps and a scope (boundary) |

- The return value of `Frag::scope` / `Frag::boundary` is still a `Frag`. Place it in `frag![..]` as part of the outer statement, and close it with the outer `Skeleton::steps` or `Frag::scope` / `Frag::boundary`.
- Adding separators (spaces, newlines) to the inner elements is up to the user: `Frag::scope(body.into_iter().map(|s| frag![s, " "]))`.
- The appearance, such as brackets, is also added by the user. For example, to make blocks and closures separately:

```rust
fn block(stmts: Vec<Frag>) -> Frag {
    frag!["{ ", Frag::scope(stmts.into_iter().map(|s| frag![s, " "])), "}"]
}

fn closure(stmts: Vec<Frag>) -> Frag {
    frag!["[ ", Frag::boundary(stmts.into_iter().map(|s| frag![s, " "])), "]"]
}
```

### `IntoFrag` and `frag!`

`frag![e1, e2, ..]` is `Frag::concat([e1.into_frag(), e2.into_frag(), ..])`. The trailing comma can be omitted.

Implementations of `IntoFrag`:

| Type | Conversion |
| --- | --- |
| `Frag`, `&Frag` | as is (`&Frag` is cloned) |
| `Vec<T: IntoFrag>` | joins the elements |
| `Option<T: IntoFrag>` | `None` is empty |
| `(T1, T2)`, `(T1, T2, T3)`, `(T1, T2, T3, T4)` | joins the elements |
| `&str`, `String`, `&String`, `char`, `bool` | text |
| integers (`i8` to `i128`, `isize`, `u8` to `u128`, `usize`), `f32`, `f64` | text from `to_string()` |
| `Text<T: Display>` | text from `to_string()` |

For user types, implement `IntoFrag` directly, or implement `Display` and wrap with `Text(x)`. Holes cannot be put into `Display` (the hole information is lost once it becomes a string).

### `Skeleton` (finished skeleton)

| Signature | Contents |
| --- | --- |
| `fn steps(steps: impl IntoIterator<Item = Frag>) -> Skeleton` | closes each element as one step and completes the skeleton. The only way to make a `Skeleton` |
| `fn holes(&self) -> usize` | the number of holes |
| `fn solve(&self, names: &Names, fill: Fill, rule: impl Fn(&Binds) -> Bool) -> Result<Solved, Failure>` | solves directly without going through proptest (A.10) |

`fill` can only be used on `Strategy<Value = Skeleton>`, so forgetting to close steps is a type error.

The typical form:

```rust
fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8)
        .prop_map(|stmts| Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"])))
}
```

## A.5 Name candidates

### `NameSpec` (strategy that makes candidates)

Implements `Strategy<Value = Names>`. `Clone`.

| Signature | Contents |
| --- | --- |
| `fn new() -> NameSpec` | empty |
| `fn space(self, space: Space, regex: &str, count: usize) -> NameSpec` | `count` mutually distinct names matching `regex`. Candidates are sorted **lexicographically** |
| `fn space_with(self, space: Space, names: BoxedStrategy<Vec<String>>) -> NameSpec` | passes a strategy of candidates directly. The order becomes the candidate numbers as is |

- The name candidates are also made by proptest, so shrinking also makes the names themselves shorter.
- If fewer than `count` strings match `regex`, the candidates cannot be made.
- The fewer the candidates, the more the same names are reused, making the inputs harder for name resolution. However, "mutually distinct" kinds of constraints become harder to solve (e.g. `distinct_defs` with 4 definitions and 3 candidates is unsat).

Conditions that panic: an invalid regular expression / registering the same `Space` twice / the candidates of `space_with` contain an empty string or duplicates.

### `Names` (concrete candidates)

| Signature | Contents |
| --- | --- |
| `fn default() -> Names` | empty |
| `fn with(self, space: Space, names: impl IntoIterator<Item = impl Into<String>>) -> Names` | sets candidates by hand (for unit tests that call `Skeleton::solve` directly) |
| `fn get(&self, space: Space) -> &[String]` | the list of candidates. Empty if not registered |

## A.6 Writing constraints: `Binds`

The rule `rule: Fn(&Binds) -> Bool` passed to `fill` / `solve` takes `Binds` and returns a z3 `Bool`. The rule is called every time something is solved (including each stage of shrinking), so do not give it side effects (records such as `mark` are rebuilt on each solve).

All numbers in `Binds` are in evaluation order (A.1).

### Public fields

| Field | Type | Contents |
| --- | --- | --- |
| `role` | `Vec<Role>` | the role of each hole |
| `space` | `Vec<Space>` | the namespace of each hole |
| `name` | `Vec<Int>` | the name that goes into each hole. A z3 integer variable whose value is the candidate number (`0..number of candidates`). The range constraint is added automatically |

Equality of names is written `binds.name[p] == binds.name[q]` (inside `constraint!`). The name strings are not visible from constraints.

### Methods

| Signature | Returns | Scope | Contents |
| --- | --- | --- | --- |
| `fn len(&self) -> usize` | | | the number of holes |
| `fn is_empty(&self) -> bool` | | | |
| `fn of(&self, space: Space, role: Role) -> impl Iterator<Item = usize>` | evaluation order | whole | holes with that namespace and role |
| `fn defs(&self, space: Space)` | same as above | whole | `of(space, Def)` |
| `fn uses(&self, space: Space)` | same as above | whole | `of(space, Use)` |
| `fn kills(&self, space: Space)` | same as above | whole | `of(space, Kill)` |
| `fn refs(&self, space: Space)` | same as above | whole | `Use` and `Kill` |
| `fn same_step(&self, p: usize, q: usize) -> bool` | Rust `bool` | does not cross | whether they are in the same step |
| `fn encloses(&self, outer: usize, inner: usize) -> bool` | Rust `bool` | lexical | whether the scope that `outer` belongs to encloses `inner`. Always true if `outer` is at the top level. True if in the same scope. Boundaries are treated as ordinary scopes too |
| `fn boundaries_between(&self, outer: usize, inner: usize) -> usize` | Rust `usize` | lexical | the number of boundaries (`Frag::boundary`) that enclose `inner` but not `outer`. Ordinary scopes are not counted. Used when `encloses(outer, inner)` is true (the number of boundaries crossed from definition `outer` to reference `inner`) |
| `fn mark<L: Label>(&self, i: usize, kind: L, broken: Bool) -> Bool` | returns `broken` as is | — | A.8 |
| `fn observe<L: Label>(&self, kind: L, cond: Bool)` | | — | A.8 |
| `fn require<L: Label>(&self, label: L, constraint: Bool)` | nothing | — | A.8 |

`same_step`, `encloses` and `boundaries_between` are information fixed at generation time, so they are Rust values rather than z3 expressions. Use them with `filter` or `if` to narrow down candidates.

## A.7 Writing constraints: rules (`proptest_z3::rules`)

All of them take `&Binds` and return a z3 `Bool`. `i` is a number in evaluation order.

| Signature | Scope | Holds when |
| --- | --- | --- |
| `visible(binds: &Binds, i: usize) -> Bool` | lexical + `Kill` is whole | reference `i` points to a definition that is visible and not deleted. That is, for some `k` in `reaching_defs(i)`, `name[k] == name[i]` and there is no `Kill` of the same name between `k` and `i` |
| `killed(binds: &Binds, i: usize) -> Bool` | same as above | the name of reference `i` was defined within the visible range but has been deleted. And `!visible(i)` |
| `bound(binds: &Binds, space: Space) -> Bool` | same as above | every reference in `space` is `visible` |
| `unbound(binds: &Binds, space: Space, n: usize) -> Bool` | same as above | exactly `n` of the references in `space` are `!visible` |
| `defined_in_same_step(binds: &Binds, i: usize) -> Bool` | does not cross | there is a `Def` with the same namespace and the same name in the same step as `i` (the order of evaluation does not matter) |
| `distinct_defs(binds: &Binds, space: Space) -> Bool` | **whole** | the names of the `Def`s in `space` are mutually distinct. True if there are 0 or 1 `Def`s |
| `resolves_to(binds: &Binds, i: usize, k: usize) -> Bool` | lexical + `Kill` is whole | reference `i` points to definition `k`. That is, `k` is in `reaching_defs(i)`, `name[k] == name[i]`, no candidate in `reaching_defs(i)` after `k` has the same name, and there is no `Kill` of the same name between `k` and `i`. A constant false if `k` is not in `reaching_defs(i)` |
| `captured(binds: &Binds, i: usize) -> Bool` | lexical (counts boundaries) | reference `i` points to a definition outside a boundary that encloses `i` (capture). That is, `resolves_to(i, k)` for some `k` in `reaching_defs(i)` satisfying `boundaries_between(k, i) > 0` |

Functions that are material for constraints (they return iterators, not z3 expressions):

| Signature | Scope | Contents |
| --- | --- | --- |
| `reaching_defs(binds: &Binds, i: usize) -> impl Iterator<Item = usize>` | lexical | candidate definitions that `i` may point to. `k` satisfying `k < i`, `Def`, the same namespace as `i`, and `encloses(k, i)` (evaluation order) |
| `kills_between(binds: &Binds, k: usize, i: usize) -> impl Iterator<Item = usize>` | **whole** | `j` satisfying `k < j < i` (strictly between, in evaluation order), `Kill`, and the same namespace as `i` |

`resolves_to(i, k)` is `Solved::target` turned into a constraint. After solving, `target(hole of i) == Some(hole of k)` only when `resolves_to(i, k)` is true. `visible(i)` is equivalent to "`resolves_to` for some element of `reaching_defs(i)`".

### The three states of a reference

Reference `i` is in exactly one of the following. `!visible` is the union of "deleted" and "undefined".

| State | Expression |
| --- | --- |
| visible | `visible(binds, i)` |
| deleted | `killed(binds, i)` |
| undefined (no visible definition anywhere) | `(!visible(binds, i)).given([!killed(binds, i)])` |

"Undefined" also includes the case where there are only definitions not visible from outside (inside a closed scope, inside a sibling scope). If you want to distinguish "points to a definition out of scope" from "there is no definition anywhere", narrow down `binds.defs(..)` with `encloses` and write it yourself with `constraint!`.

### Common forms

```rust
// positive case
bound(binds, VAR)

// positive case including redefinition (including same names across scopes)
constraint!(bound(binds, VAR), !distinct_defs(binds, VAR))

// negative case: exactly one reference is not visible. Mark that hole
constraint!(exactly(1) { for i in binds.refs(VAR) {
    binds.mark(i, Bug::Unbound, !visible(binds, i))
} })

// self reference (`let x = x`)
constraint!(all { !visible(binds, i), defined_in_same_step(binds, i) })

// definition k is used (for detecting unused definitions; add `!` for unused)
constraint!(any { for i in binds.refs(VAR) { resolves_to(binds, i, k) } })

// mark captured references (a classification, so not made a constraint)
for i in binds.refs(VAR) {
    binds.mark(i, Captured, captured(binds, i));
}

// at least one capture that crosses two or more boundaries (relayed by an outer closure)
fn relayed(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .filter(|&k| binds.boundaries_between(k, i) >= 2)
        .map(|k| resolves_to(binds, i, k))
        .collect();
    Bool::or(&cases)
}
constraint!(bound(binds, VAR), any { for i in binds.refs(VAR) { relayed(binds, i) } })
```

When you want to change the condition by the number of boundaries, do not use `captured`; like `relayed`, narrow down `reaching_defs` with `boundaries_between` and combine `resolves_to`.

## A.8 Marks, classification and named constraints (labels)

### `Label`

Every type that is `Copy + Debug + PartialEq + Send + Sync + 'static` is a `Label` (blanket implementation). Use your own enums or `&'static str`.

- The **name** of a label is the string itself for `&'static str`, and the `Debug` output otherwise. Statistics (`label_stats`) and the tally of rejections (`take_rejections`) are counted by this name. Two labels of different types are counted as the same if their names are the same.
- `marks::<L>()` / `labels::<L>()` filter by the type `L`.

### Recording (methods of `Binds`)

| Signature | Becomes a constraint? | Contents |
| --- | --- | --- |
| `mark(i, kind, broken: Bool) -> Bool` | only when the return value is put into a constraint | records that hole `i` is broken as `kind`. After solving, if `broken` is true, `(Hole, kind)` appears in `s.marks()` |
| `observe(kind, cond: Bool)` | no | only checks whether `cond` is true after solving. Classifies without changing the distribution of generation |
| `require(label, constraint: Bool)` | yes (mandatory) | a named constraint. When it cannot be solved, the name of `label` appears in the unsat core. It returns no value, so it cannot be written inside `constraint!`. Call it in the body of the rule |

- `mark` returns `broken` as is. It only becomes a constraint once the return value is placed inside `exactly(1) { .. }` or the like. If not placed, it is the same as an `observe` attached to a hole.
- The truth of `mark` is determined by evaluating `broken` in the model after solving. Multiple `mark`s can be true for one hole (when violation conditions overlap).
- The return value of `rule` and the range constraints on names have no names, so they do not appear in the unsat core. When a named constraint conflicts with an unnamed one, only the named side appears in the reason. Even if a reason has only one name, that constraint is not necessarily unsatisfiable on its own. Move every constraint whose rejection causes you want to investigate into a `require`, and return `Bool::from_bool(true)`.
- If you `require` a constraint and also include it in the return value of `rule`, the unnamed side alone can explain the conflict, so that label no longer appears in the reason.
- Calling `require` several times with the same label groups them into one named constraint (calling it per hole in a loop still shows the name only once in a reason).

```rust
fn rule(binds: &Binds) -> Bool {
    binds.require("self_ref", constraint!(any { for i in binds.uses(VAR) { defined_in_same_step(binds, i) } }));
    binds.require("all_bound", bound(binds, VAR));
    binds.require("no_redefinition", distinct_defs(binds, VAR));
    Bool::from_bool(true)
}
```

### `Given`

```rust
pub trait Given {
    fn given(self, guard: impl IntoIterator<Item = Bool>) -> Bool;
}
impl Given for Bool { .. } // self ∧ guard1 ∧ guard2 ∧ ..
```

A way of writing "ask only when the preconditions hold". When one violation always comes with another, excluding one of them with `given` makes both kinds generated independently.

```rust
exactly(1) { for i in binds.refs(VAR) { any {
    binds.mark(i, Bug::Killed, killed(binds, i)),
    binds.mark(i, Bug::SelfRef, constraint!(all {
        !visible(binds, i),
        defined_in_same_step(binds, i),
    })),
    binds.mark(i, Bug::Undefined, (!visible(binds, i)).given([
        !killed(binds, i),
        !defined_in_same_step(binds, i),
    ])),
} } }
```

### Statistics (per thread)

| Signature | Contents |
| --- | --- |
| `label_stats() -> Vec<LabelStat>` | the number of occurrences per label in the cases generated on this thread |
| `take_label_stats() -> Vec<LabelStat>` | takes the same as above and resets |
| `take_rejections() -> Vec<(Vec<String>, usize)>` | takes the reasons and counts of rejections and resets. A reason is the names of `require` that appeared in the unsat core (sorted). An empty set when only unnamed constraints conflict. Failures other than unsat are the display string of `Failure`. Only counted at generation time in `fill` |

```rust
pub struct LabelStat {
    pub label: String,   // the name of the label
    pub declared: usize, // the number of cases in which the label was recorded
    pub occurred: usize, // the number of cases in which the label was true at least once
}
```

- Only newly generated cases are counted; cases in the middle of shrinking are not counted.
- When the test thread ends, labels that never became true are warned about on standard error. They are shown even if the test passed.

```text
proptest-z3 warning: label `Delete` never occurred in 4 cases (test `each_kind_is_reported`); is it implied by another violation? consider `given`
```

- If you `mark` even kinds that you constrained not to occur, this warning appears. `mark` only the kinds you aim for.

## A.9 Passing to proptest

### `FillExt`

Implemented for every type that implements `Strategy<Value = Skeleton>`.

| Signature | Contents |
| --- | --- |
| `fn fill<R>(self, names: NameSpec, rule: R) -> Filled<Self, R>` | `fill_with(Fill::Greedy, ..)` |
| `fn fill_with<R>(self, fill: Fill, names: NameSpec, rule: R) -> Filled<Self, R>` | chooses how to solve |

`R: Fn(&Binds) -> Bool + 'static`. When using outside values, make it a `move` closure.

`Filled<S, R>` is a `Strategy<Value = Solved>`.

### `Fill`

| Value | Contents |
| --- | --- |
| `Fill::Greedy` (default) | solves with the hint of each hole as an assumption. On a conflict, drops the hint that is last in evaluation order within the unsat core and solves again. Uses a `QF_LIA` solver. Deterministic and fast |
| `Fill::Optimal` | maximizes the number of hints kept (`Optimize`). Faithful, but slow with many holes |

In `Greedy`, preferences are given up starting from holes later in evaluation order, so names are closer to the hints for holes evaluated earlier.

### Behavior of generation and shrinking

- Generation: makes a skeleton and candidates, and remakes them if they cannot be solved (unsat). One remake is one local reject in proptest, and the test is aborted when the limit (`max_local_rejects`) is exceeded.
- Shrinking: when a shrunk input cannot be solved, it goes back as "the failure cannot be reproduced in that direction" and searches another direction.
- If the solved result (the text and the marks and labels) is the same as one tested before, the test is not called. If the skeleton and candidates are the same, it is not solved again.
- Shrinking is not run to a fixed point. Lines unrelated to the counterexample may remain.

## A.10 Result: `Solved`

`Clone`. Contains no z3 types.

| Signature | Contents |
| --- | --- |
| `pub text: String` | the text with holes filled |
| `fn holes(&self) -> impl Iterator<Item = (Hole, &FilledHole)>` | all holes in text order |
| `fn hole(&self, hole: Hole) -> &FilledHole` | |
| `fn span(&self, hole: Hole) -> Range<usize>` | the position in `text` (bytes) |
| `fn name(&self, hole: Hole) -> &str` | the name that went in |
| `fn events(&self) -> &[Hole]` | holes arranged in evaluation order. `events()[i]` is the hole at evaluation order `i` |
| `fn target(&self, hole: Hole) -> Option<Hole>` | the definition hole that a reference points to. For `Kill`, the definition it deletes. `None` for definition holes and for references that are not visible |
| `fn marks<L: Label>(&self) -> Vec<(Hole, L)>` | the `mark`s that became true |
| `fn labels<L: Label>(&self) -> Vec<L>` | the `observe`s that became true |
| `fn has<L: Label>(&self, kind: L) -> bool` | whether `kind` of `observe` became true |
| `fn numbered(&self) -> String` | the text with line numbers (starting from 0) |

Rules of `target` (the same as A.2):

- The candidates are those in `reaching_defs` (lexical) with the same name and with no `Kill` of the same name (in any scope) in between.
- It returns the last one among them in evaluation order. Therefore, with shadowing it points to the inner definition, and with reassignment in the same scope it points to the last assignment.
- What a `Kill` actually deletes is every definition of the same name. `target` returns only one of them (the one chosen by the rule above).
- To write the same thing inside a constraint, use `resolves_to(binds, i, k)` (A.7). Boundaries do not change the result of `target`.

### `FilledHole`

```rust
pub struct FilledHole {
    pub space: Space,
    pub role: Role,
    pub name: String,
    pub span: Range<usize>, // the position in text (bytes)
}
```

### `Hole`

`pub struct Hole(pub usize)`. A number in text order. `Copy`, `Eq`, `Ord`, `Hash`.

### The `Debug` display

Used for displaying counterexamples. It shows the text (with line numbers), the marks that became true, and the labels of `observe` that became true.

```text
Solved{
  0| let b
  1| let a = a
 marks: Undefined @ 14..15 `a`
 labels: Reassigned
}
```

A mark has the form "label name @ byte range name (in backquotes)". The `marks:` and `labels:` lines are not shown if empty.

## A.11 Solving directly: `Skeleton::solve`

```rust
fn solve(&self, names: &Names, fill: Fill, rule: impl Fn(&Binds) -> Bool) -> Result<Solved, Failure>
```

Solves one skeleton without going through proptest. Used in unit tests and when checking generated inputs.

```rust
let skeleton = Skeleton::steps([frag!["def ", Frag::hole(VAR, Role::Def, Hint(0))]]);
let names = Names::default().with(VAR, ["a", "b"]);
let solved = skeleton.solve(&names, Fill::Greedy, |binds| bound(binds, VAR))?;
```

- If it cannot be solved, `Err(Failure::Unsat { core })` (`core` is the names of `require`); if it cannot be decided, `Err(Failure::Unknown { reason })`.
- It panics if the candidates of a namespace that has holes in the skeleton are empty (`no names for space ..`).
- It does not count label statistics (they are only counted at generation time in `fill`).

## A.12 What is used from z3-toolkit

Inside rules, z3-toolkit's `constraint!` is used (it comes in with the prelude). The forms used in proptest-z3's examples and tests are as follows. For the exact syntax, see the documentation of z3-toolkit.

| Form | Meaning |
| --- | --- |
| `constraint!(e1, e2, ..)` | all hold (conjunction) |
| `constraint!(x == y)` / `constraint!(x in 0..n)` | comparison / range |
| `all { .. }` / `any { .. }` | conjunction / disjunction |
| `exactly(n) { .. }` / `at_most(n) { .. }` | exactly n / at most n are true |
| `{ for i in iter { e1, e2 } }` | lists terms by iteration. If multiple terms are written in the body, all of them are counted |
| `if cond { e } else { e }` (Rust `bool`) | chooses terms by a condition fixed at generation time |
| `e1 \|\| e2`,`!e` | disjunction / negation |

z3 `Bool`s can also be built directly: `bound(binds, VAR) & bound(binds, FUNC)`, `Bool::from_bool(true)` (no constraint), `Bool::or(&[..])` (false if empty), `Bool::and(&[..])` (true if empty).

## A.13 Conditions that lead to rejection or panic

| Situation | Result |
| --- | --- |
| the rule is unsat (e.g. `exactly(1) { for i in binds.refs(..) .. }` on a skeleton with no references) | a local reject during generation. Treated as "passed" during shrinking |
| local rejects exceed `max_local_rejects` | the test is aborted |
| a namespace that has holes in the skeleton is not registered in `NameSpec` | panic (`no names for space ..`) |
| registering the same `Space` in `NameSpec` twice | panic |
| the candidates of `space_with` contain an empty string or duplicates | panic |
| the regular expression of `NameSpec::space` is invalid | panic |

When rejections are frequent, give names to the constraints with `require` and look at `take_rejections()`. Often the shape (strategy) cannot satisfy the conditions, so prepare shapes that can satisfy them on the strategy side (e.g. in a use-after-delete test, make sure to build a shape that places a use after `del`).

- A `proptest!` test panics when rejections exceed the limit, and cannot proceed to `take_rejections()` afterwards. Generate by calling `new_tree` directly on a `TestRunner`, and tally the results
- A set with only one name is almost always caused by the shape (it is rejected even if the other constraints are removed). A contradiction in the rule itself appears as a set that no shape can satisfy. The most frequent set is not necessarily the cause
- The sets returned by Z3 are not necessarily minimal. Remove suspicious constraints one at a time to confirm

```rust
use proptest::test_runner::TestRunner;
use proptest_z3::take_rejections;

#[test]
fn why_rejected() {
    let strategy = program().fill(names(), rule);
    let mut runner = TestRunner::default();
    for _ in 0..50 {
        if strategy.new_tree(&mut runner).is_err() {
            break; // rejections exceeded the limit
        }
    }
    for (core, count) in take_rejections() {
        eprintln!("{count:>5} {core:?}");
    }
}
```

## A.14 Checklist when writing tests

Points where an LLM tends to make mistakes when writing tests.

- Numbers inside constraints are in evaluation order; the `Hole` of `Solved` is in text order. A `Hole` cannot be made inside a constraint.
- The right-hand side of a statement does not see the definition on the left-hand side. The `Def` of a statement is not visible from the scopes contained in that statement either (by default, recursion is not possible).
- `Kill` deletes every definition of the same name across scopes.
- `distinct_defs` counts across scopes. `!distinct_defs` means "there is a definition of the same name somewhere", not necessarily a reassignment in the same scope.
- `Frag::boundary` does not change the visible range. Outer definitions are normally visible from inside a closure. Whether it is a capture is decided with `captured` / `boundaries_between`.
- `boundaries_between` does not count ordinary scopes (`Frag::scope`). Pointing to an outer definition from inside a block is not a capture.
- The contents of a boundary are treated as evaluated at the position where they are written. A `Kill` in a closure body also affects later references.
- To distinguish "undefined" from other violations, add preconditions with `given`. Otherwise, some kind may never be generated (you can notice it from the warning).
- If you forget to put the return value of `mark` into a constraint, it is only a record.
- `exactly(1)` in a negative case is unsat if the skeleton has no target hole at all. Make sure the target hole is built on the shape side.
- Write the checker for positive cases (the expected values of tests) independently of `rules`. If written with the same semantics, it shares the same mistakes.
- The closure of a rule is `'static`. Bring outside values in with `move`.
- `require` returns no value, so do not write it inside `constraint!`. Call it in the body of the rule, and do not also put the same constraint in the return value.
