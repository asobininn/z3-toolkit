use z3::ast::Int;

use crate::__private::Arith;

/// todo
pub fn sum<I>(items: I) -> Int
where
    I: IntoIterator,
    I::Item: Arith,
{
    items.into_iter().map(|x| x.arith()).sum()
}

/// todo
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
