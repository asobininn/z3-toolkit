use z3::ast::Int;

use crate::__private::Arith;

/// Sum as an `Int`. `Bool` counts as 0/1.
///
/// ## Examples
/// ```
/// # use z3::ast::Bool; use z3_toolkit::prelude::*;
/// let picks: [Bool; 3] = fresh("p");
/// let count = sum(&picks);
/// ```
pub fn sum<I>(items: I) -> Int
where
    I: IntoIterator,
    I::Item: Arith,
{
    items.into_iter().map(|x| x.arith()).sum()
}

/// Inner product `Σ lhs[i] * rhs[i]` as an `Int`.
///
/// Each side takes Rust ints, `bool`, `Int` or `Bool` (owned or borrowed). `Bool` / `bool` count
/// as 0/1. Rust ints are lifted, and panic if they don't fit in `i64`.
///
/// - Lengths aren't checked: the longer side's extra elements are silently dropped.
/// - Keep one side concrete. Symbolic × symbolic is nonlinear and slow (or `Unknown`) for Z3.
/// - Empty input gives `0`.
///
/// ## Examples
/// ```
/// # use z3::{Optimize, ast::Bool}; use z3_toolkit::prelude::*;
/// let picks: [Bool; 3] = fresh("p");
/// let (weights, values) = ([4, 2, 7], [10, 3, 8]);
/// let opt = Optimize::new();
/// constraints! { opt; dot(weights, &picks) <= 9 };
/// opt.maximize(&dot(values, &picks));
/// # let m = opt.solve().unwrap();
/// # assert_eq!(m.get(&picks), Ok([true, true, false]));
/// ```
pub fn dot<L, R>(lhs: L, rhs: R) -> Int
where
    L: IntoIterator,
    L::Item: Arith,
    R: IntoIterator,
    R::Item: Arith,
{
    let (lhs, rhs): (Vec<L::Item>, Vec<R::Item>) =
        (lhs.into_iter().collect(), rhs.into_iter().collect());
    lhs.iter()
        .zip(&rhs)
        .map(|(x, y)| x.arith() * y.arith())
        .sum()
}

/// `|x|` via `ite`.
///
/// ## Examples
/// ```
/// # use z3_toolkit::prelude::*;
/// let [x, y] = ints("v");
/// let dist = abs(&x - &y);
/// ```
pub fn abs(x: impl Arith) -> Int {
    let x = x.arith();
    x.lt(0).ite(&-&x, &x)
}

/// Larger one via `ite`.
///
/// ## Examples
/// ```
/// # use z3_toolkit::prelude::*;
/// let [x] = ints("x");
/// let clamped = max(&x, 0);
/// ```
pub fn max(lhs: impl Arith, rhs: impl Arith) -> Int {
    let (lhs, rhs) = (lhs.arith(), rhs.arith());
    lhs.ge(&rhs).ite(&lhs, &rhs)
}

/// Smaller one via `ite`.
///
/// ## Examples
/// ```
/// # use z3_toolkit::prelude::*;
/// let [x] = ints("x");
/// let capped = min(&x, 100);
/// ```
pub fn min(lhs: impl Arith, rhs: impl Arith) -> Int {
    let (lhs, rhs) = (lhs.arith(), rhs.arith());
    lhs.le(&rhs).ite(&lhs, &rhs)
}
