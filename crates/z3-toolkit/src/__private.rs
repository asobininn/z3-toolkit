//! Runtime pieces used by macro-generated code. Not public API

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

/// Expantion target of `sum(iter, |x| e)`
pub fn sum_by<I, F>(iter: I, f: F) -> Int
where
    I: IntoIterator,
    F: FnMut(I::Item) -> Int,
{
    iter.into_iter().map(f).sum()
}

/// Expansion target of `e in <iter>`
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

/// Expansion target of `==` / `!=` without arithmetic: compares shapes element-wise.
pub trait ShapeEq<Rhs: ?Sized> {
    fn shape_eq(&self, rhs: &Rhs) -> Bool;
}

pub fn shape_eq<L, R>(lhs: &L, rhs: &R) -> Bool
where
    L: ShapeEq<R> + ?Sized,
    R: ?Sized,
{
    lhs.shape_eq(rhs)
}

pub fn shape_ne<L, R>(lhs: &L, rhs: &R) -> Bool
where
    L: ShapeEq<R> + ?Sized,
    R: ?Sized,
{
    lhs.shape_eq(rhs).not()
}

impl<R: Operand<Sort = Int> + ?Sized> ShapeEq<R> for Int {
    fn shape_eq(&self, rhs: &R) -> Bool {
        self.eq(rhs.operand())
    }
}

impl<R: Operand<Sort = Bool> + ?Sized> ShapeEq<R> for Bool {
    fn shape_eq(&self, rhs: &R) -> Bool {
        self.eq(rhs.operand())
    }
}

impl<R: Operand<Sort = Bool> + ?Sized> ShapeEq<R> for bool {
    fn shape_eq(&self, rhs: &R) -> Bool {
        self.operand().eq(rhs.operand())
    }
}

macro_rules! shape_eq_int {
    ($($t:ty),*) => {$(
        impl<R: Operand<Sort = Int> + ?Sized> ShapeEq<R> for $t {
            fn shape_eq(&self, rhs: &R) -> Bool {
                self.operand().eq(rhs.operand())
            }
        }
    )*};
}
shape_eq_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl<T: ShapeEq<R> + ?Sized, R: ?Sized> ShapeEq<R> for &T {
    fn shape_eq(&self, rhs: &R) -> Bool {
        (**self).shape_eq(rhs)
    }
}

fn zip_eq<L: ShapeEq<R>, R>(lhs: &[L], rhs: &[R]) -> Bool {
    if lhs.len() != rhs.len() {
        return Bool::from_bool(false);
    }
    and(lhs.iter().zip(rhs).map(|(l, r)| l.shape_eq(r)).collect())
}

impl<L: ShapeEq<R>, R, const N: usize> ShapeEq<[R; N]> for [L; N] {
    fn shape_eq(&self, rhs: &[R; N]) -> Bool {
        zip_eq(self, rhs)
    }
}

impl<L: ShapeEq<R>, R, const N: usize> ShapeEq<[R]> for [L; N] {
    fn shape_eq(&self, rhs: &[R]) -> Bool {
        zip_eq(self, rhs)
    }
}

impl<L: ShapeEq<R>, R, const N: usize> ShapeEq<Vec<R>> for [L; N] {
    fn shape_eq(&self, rhs: &Vec<R>) -> Bool {
        zip_eq(self, rhs)
    }
}

impl<L: ShapeEq<R>, R> ShapeEq<[R]> for [L] {
    fn shape_eq(&self, rhs: &[R]) -> Bool {
        zip_eq(self, rhs)
    }
}

impl<L: ShapeEq<R>, R, const N: usize> ShapeEq<[R; N]> for [L] {
    fn shape_eq(&self, rhs: &[R; N]) -> Bool {
        zip_eq(self, rhs)
    }
}

impl<L: ShapeEq<R>, R> ShapeEq<Vec<R>> for [L] {
    fn shape_eq(&self, rhs: &Vec<R>) -> Bool {
        zip_eq(self, rhs)
    }
}

impl<L, Rhs: ?Sized> ShapeEq<Rhs> for Vec<L>
where
    [L]: ShapeEq<Rhs>,
{
    fn shape_eq(&self, rhs: &Rhs) -> Bool {
        self.as_slice().shape_eq(rhs)
    }
}

macro_rules! shape_eq_tuple {
    ($($l:ident $r:ident $i:tt),+) => {
        impl<$($l: ShapeEq<$r>,  $r),+> ShapeEq<($($r,)+)> for ($($l,)+) {
            fn shape_eq(&self, rhs: &($($r,)+)) -> Bool {
                and(::std::vec![$(self.$i.shape_eq(&rhs.$i)),+])
            }
        }
    };
}
shape_eq_tuple!(L1 R1 0);
shape_eq_tuple!(L1 R1 0, L2 R2 1);
shape_eq_tuple!(L1 R1 0, L2 R2 1, L3 R3 2);
shape_eq_tuple!(L1 R1 0, L2 R2 1, L3 R3 2, L4 R4 3);
