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

pub fn abs(x: impl Arith) -> Int {
    let x = x.arith();
    x.lt(0).ite(&-&x, &x)
}

pub fn max(lhs: impl Arith, rhs: impl Arith) -> Int {
    let (lhs, rhs) = (lhs.arith(), rhs.arith());
    lhs.ge(&rhs).ite(&lhs, &rhs)
}

pub fn min(lhs: impl Arith, rhs: impl Arith) -> Int {
    let (lhs, rhs) = (lhs.arith(), rhs.arith());
    lhs.le(&rhs).ite(&lhs, &rhs)
}
