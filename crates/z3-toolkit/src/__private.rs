//! Runtime pieces used by macro-generated code. Not plubic API

pub use z3::Model;
pub use z3::ast::{Ast, Bool, Int};

/// Turns a value leaf into a Z3 value by borrowing
pub trait Operand {
    type Sort;
    fn operand(&self) -> Self::Sort;
}

impl Operand for Int {
    type Sort = Int;
    fn operand(&self) -> Self::Sort {
        self.clone()
    }
}

impl Operand for Bool {
    type Sort = Bool;
    fn operand(&self) -> Self::Sort {
        self.clone()
    }
}

impl Operand for i64 {
    type Sort = Int;
    fn operand(&self) -> Self::Sort {
        Int::from_i64(*self)
    }
}

impl Operand for bool {
    type Sort = Bool;
    fn operand(&self) -> Self::Sort {
        Bool::from_bool(*self)
    }
}

macro_rules! operand_int {
    ($($t:ty),*) => {$(
        impl Operand for $t {
            type Sort = Int;
            fn operand(&self) -> Self::Sort {
                Int::from_i64(i64::try_from(*self).expect("integer does not fit in i64"))
            }
        }
    )*};
}
operand_int!(i8, i16, i32, isize, u8, u16, u32, u64, usize);

pub fn int_lit(value: i64) -> Int {
    Int::from_i64(value)
}

impl<T: Operand + ?Sized> Operand for &T {
    type Sort = T::Sort;
    fn operand(&self) -> Self::Sort {
        (**self).operand()
    }
}

/// Treat `Bool` implicitly as `Int` values of 0/1
pub trait Arith {
    fn arith(&self) -> Int;
}

impl Arith for Int {
    fn arith(&self) -> Int {
        self.clone()
    }
}

impl Arith for Bool {
    fn arith(&self) -> Int {
        self.ite(&Int::from_i64(1), &Int::from_i64(0))
    }
}

impl Arith for bool {
    fn arith(&self) -> Int {
        Int::from_i64(i64::from(*self))
    }
}

macro_rules! arith_int {
    ($($t:ty),*) => {$(
        impl Arith for $t {
            fn arith(&self) -> Int {
                Int::from_i64(i64::try_from(*self).expect("integer does not fit in i64"))
            }
        }
    )*};
}
arith_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl<T: Arith + ?Sized> Arith for &T {
    fn arith(&self) -> Int {
        (**self).arith()
    }
}

/// Expansion target of `sum(iter, |x| e)`
pub fn sum_by<I, F>(iter: I, f: F) -> Int
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Int,
{
    iter.into_iter().map(f).sum()
}

/// Expantion target of `e in <iter>`
pub fn member<I>(lhs: &Int, items: I) -> Bool
where
    I: IntoIterator,
    I::Item: Arith,
{
    or(items.into_iter().map(|v| lhs.eq(v.arith())).collect())
}

pub trait ToBool {
    fn to_bool(&self) -> Bool;
}

impl ToBool for Bool {
    fn to_bool(&self) -> Bool {
        self.clone()
    }
}

impl ToBool for bool {
    fn to_bool(&self) -> Bool {
        Bool::from_bool(*self)
    }
}

impl<T: ToBool + ?Sized> ToBool for &T {
    fn to_bool(&self) -> Bool {
        (**self).to_bool()
    }
}

pub fn and(items: Vec<Bool>) -> Bool {
    Bool::and(&items)
}

pub fn or(items: Vec<Bool>) -> Bool {
    Bool::or(&items)
}

pub fn not(b: &Bool) -> Bool {
    b.not()
}

pub fn implies(p: &Bool, q: &Bool) -> Bool {
    p.implies(q)
}

pub fn exactly(items: &[Bool], k: i32) -> Bool {
    let pairs: Vec<(&Bool, i32)> = items.iter().map(|b| (b, 1)).collect();
    Bool::pb_eq(&pairs, k)
}

pub fn at_most(items: &[Bool], k: i32) -> Bool {
    let pairs: Vec<(&Bool, i32)> = items.iter().map(|b| (b, 1)).collect();
    Bool::pb_le(&pairs, k)
}

pub fn at_least(items: &[Bool], k: i32) -> Bool {
    let pairs: Vec<(&Bool, i32)> = items.iter().map(|b| (b, 1)).collect();
    Bool::pb_ge(&pairs, k)
}

pub fn distinct<'a, A: Ast + 'a>(items: impl IntoIterator<Item = &'a A>) -> Bool {
    let v: Vec<&A> = items.into_iter().collect();
    A::distinct(&v)
}
