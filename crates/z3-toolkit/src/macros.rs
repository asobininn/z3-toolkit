// Thin wrappers that pass `$crate` to the proc macros, so the expansion resolves z3-toolkit
// even when the caller depends on it only indirectly (e.g. through a re-export in another crate).

/// Same syntax as [`constraints!`](crate::constraints!), but returns one ANDed `Bool` instead of asserting.
///
/// Labels are not allowed here.
///
/// ## Examples
/// ```
/// # use z3_toolkit::prelude::*;
/// let [x, y] = ints("v");
/// let c = constraint!(x < y, x in 0..10);
/// ```
#[macro_export]
macro_rules! constraint {
    ($($tokens:tt)*) => {
        $crate::__private::__constraint! { ($crate) $($tokens)* }
    };
}

/// Asserts constraints into a `Solver` / `Optimize`
///
/// ## Examples
/// ```
/// # use z3::Solver; use z3_toolkit::prelude::*;
/// let q = ints_n("q", 4);
/// let s = Solver::new();
/// constraints! { s;
///     for c in &q { c in 0..4 },
///     distinct(q.iter()),
///     for i in 0..4 { for j in i + 1..4 { abs(q[i] - q[j]) != j - i } }
/// }
/// # assert!(s.solve().is_ok());
/// ```
///
/// ## Items
/// - `x < y`, `p == q`, `!p`, `p && q`, `p || q`: only the top-level operators are translated
/// - `xs == [0, 1]`, `(x, p) != (1, true)`: arrays, `Vec`s and tuples compare element-wise
/// - `2 * x + 1`, `sum(xs, |v| e)`, `abs` / `max` / `min`: Rust ints are lifted, `Bool` is 0/1
/// - `x in 1..=3`, `x in [2, 5]`, `x in iter`
/// - `all { .. }`, `any { .. }`, `not { .. }`
/// - `{ p } implies { q }`
/// - `exactly(k) { .. }`, `at_most(k) { .. }`, `at_least(k) { .. }`
/// - `distinct(iter)`, `differs(&model, vars)`: an array literal `[x, y]` is borrowed, not moved
/// - `for` / `if`: plain Rust, unrolled at runtime
/// - `"label": item`: named in the unsat core. Not inside logic blocks (`all`, `implies`, ..).
/// - `"label" @ item`: read back with `model.labels(&expr)` when `item` is true. Labels only the next term: `"a" @ p || q` labels `p`.
///
/// `,` is optional after an item ending with `}`.
#[macro_export]
macro_rules! constraints {
    ($($tokens:tt)*) => {
        $crate::__private::__constraints! { ($crate) $($tokens)* }
    };
}

/// Arithmetic expression → `Int`, with the same lifting rules as [`constraints!`](crate::constraints!).
///
/// ## Examples
/// ```
/// # use z3::Optimize; use z3_toolkit::prelude::*;
/// let ([x], [p]) = (ints("x"), bools("p"));
/// let opt = Optimize::new();
/// opt.maximize(&lin!(3 * p - x));
/// ```
#[macro_export]
macro_rules! lin {
    ($($tokens:tt)*) => {
        $crate::__private::__lin! { ($crate) $($tokens)* }
    };
}
