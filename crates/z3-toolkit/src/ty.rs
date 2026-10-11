use std::fmt::Display;

use z3::{
    Model,
    ast::{Ast, Bool, Int},
};

use crate::{Labeled, ReadError};

/// Z3 shapes [`ModelExt::get`] reads back: `Int` → `i64`, `Bool` → `bool`,
/// and arrays, `Vec`, slices and tuples (up to 4) of them.
pub trait Readback {
    type Value;

    /// Unconstrained variables get a defailt.
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError>;
}

fn eval<A: Ast + Display>(model: &Model, ast: &A) -> Result<A, ReadError> {
    model.eval(ast, true).ok_or_else(|| ReadError::NoValue {
        expr: ast.to_string(),
    })
}

fn not_literal(expr: &impl Display, value: &impl Display) -> ReadError {
    ReadError::NotLiteral {
        expr: expr.to_string(),
        value: value.to_string(),
    }
}

impl Readback for Int {
    type Value = i64;
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
        let v = eval(model, self)?;
        v.as_i64().ok_or_else(|| not_literal(self, &v))
    }
}

impl Readback for Bool {
    type Value = bool;
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
        let v = eval(model, self)?;
        v.as_bool().ok_or_else(|| not_literal(self, &v))
    }
}

impl<R: Readback + ?Sized> Readback for &R {
    type Value = R::Value;
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
        (**self).read(model)
    }
}

impl<R: Readback> Readback for [R] {
    type Value = Vec<R::Value>;
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
        self.iter().map(|r| r.read(model)).collect()
    }
}

impl<R: Readback> Readback for Vec<R> {
    type Value = Vec<R::Value>;
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
        self.as_slice().read(model)
    }
}

impl<R: Readback, const N: usize> Readback for [R; N] {
    type Value = [R::Value; N];
    fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
        self.as_slice()
            .read(model)?
            .try_into()
            .map(Ok)
            .unwrap_or_else(|_| unreachable!("read keeps the length"))
    }
}

macro_rules! readback_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: Readback),+> Readback for ($($t,)+) {
            type Value = ($($t::Value,)+);
            fn read(&self, model: &Model) -> Result<Self::Value, ReadError> {
                Ok(($(self.$i.read(model)?,)+))
            }
        }
    };
}
readback_tuple!(T1 0);
readback_tuple!(T1 0, T2 1);
readback_tuple!(T1 0, T2 1, T3 2);
readback_tuple!(T1 0, T2 1, T3 2, T4 3);

/// Shapes [`fresh`] can create: `Int`, `Bool`, `[T; N]`, and tuples up to 4.
pub trait Fresh: Sized {
    fn fresh(prefix: &str) -> Self;
}

impl Fresh for Int {
    fn fresh(prefix: &str) -> Self {
        Int::fresh_const(prefix)
    }
}

impl Fresh for Bool {
    fn fresh(prefix: &str) -> Self {
        Bool::fresh_const(prefix)
    }
}

impl<T: Fresh, const N: usize> Fresh for [T; N] {
    fn fresh(prefix: &str) -> Self {
        std::array::from_fn(|_| T::fresh(prefix))
    }
}

macro_rules! fresh_tuple {
    ($($t:ident),+) => {
        impl<$($t: Fresh),+> Fresh for ($($t,)+) {
            fn fresh(prefix: &str) -> Self {
                ($($t::fresh(prefix),)+)
            }
        }
    };
}
fresh_tuple!(T1);
fresh_tuple!(T1, T2);
fresh_tuple!(T1, T2, T3);
fresh_tuple!(T1, T2, T3, T4);

/// Fresh constants shaped by `T`. Names not collide, even with the same prefix.
///
/// ## Examples
/// ```
/// # use z3::ast::{Bool, Int}; use z3_toolkit::fresh;
/// let (grid, flag): ([[Int; 3]; 3], Bool) = fresh("g");
/// ```
pub fn fresh<T: Fresh>(prefix: &str) -> T {
    T::fresh(prefix)
}

/// [`fresh`] with a runtime length.
///
/// ## Examples
/// ```
/// # use z3::ast::Int; use z3_toolkit::fresh_n;
/// let rows: Vec<[Int; 2]> = fresh_n("r", 5);
/// ```
pub fn fresh_n<T: Fresh>(prefix: &str, len: usize) -> Vec<T> {
    std::iter::repeat_with(|| T::fresh(prefix))
        .take(len)
        .collect()
}

/// `N` fresh `Int`s. Destructure to name them.
///
/// ## Examples
/// ```
/// # use z3_toolkit::ints;
/// let [x, y, z] = ints("v");
/// ```
pub fn ints<const N: usize>(prefix: &str) -> [Int; N] {
    fresh(prefix)
}

/// `N` fresh `Bool`s. Destructure to name them.
///
/// ## Examples
/// ```
/// # use z3_toolkit::bools;
/// let [p, q] = bools("p");
/// ```
pub fn bools<const N: usize>(prefix: &str) -> [Bool; N] {
    fresh(prefix)
}

/// [`ints`] with a runtime length.
///
/// ## Examples
/// ```
/// # use z3_toolkit::ints_n;
/// let q = ints_n("q", 5);
/// ```
pub fn ints_n(prefix: &str, len: usize) -> Vec<Int> {
    fresh_n(prefix, len)
}

/// [`bools`] with a runtime length.
///
/// ## Examples
/// ```
/// # use z3_toolkit::bools_n;
/// let picks = bools_n("pick", 5);
/// ```
pub fn bools_n(prefix: &str, len: usize) -> Vec<Bool> {
    fresh_n(prefix, len)
}

/// Typed readback from a `Model`.
pub trait ModelExt {
    /// The value of `repr`, typed by its shape.
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let xs = ints_n("x", 2);
    /// let s = Solver::new();
    /// constraints! { s; xs[0] == 1, xs[1] == 2 };
    /// let m = s.solve().unwrap();
    /// assert_eq!(m.get(&xs), Ok(vec![1, 2]));
    /// ```
    fn get<R: Readback + ?Sized>(&self, repr: &R) -> Result<R::Value, ReadError>;

    /// [`get`](ModelExt::get) then convert element-wise (see [`FromValue`]).
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let [i] = ints("i");
    /// let s = Solver::new();
    /// constraints! { s; i == 3 };
    /// let m = s.solve().unwrap();
    /// assert_eq!(m.get_as::<usize>(&i), Ok(3usize));
    /// ```
    fn get_as<T>(&self, repr: &(impl ReadAs<T> + ?Sized)) -> Result<T, ReadError>;

    /// `"name" @ e` labels in `expr` whose `e` is true, in order of first appearance.
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let [x] = ints("x");
    /// let c = constraint!(any { "small" @ x < 3, "big" @ x > 7 });
    /// let s = Solver::new();
    /// constraints! {s; c, x == 9 };
    /// assert_eq!(s.solve().unwrap().labels(&c), Ok(vec!["big"]));
    /// ```
    fn labels<L: Labeled + ?Sized>(&self, expr: &L) -> Result<Vec<&'static str>, ReadError>;
}

impl ModelExt for Model {
    fn get<R: Readback + ?Sized>(&self, repr: &R) -> Result<R::Value, ReadError> {
        repr.read(self)
    }

    fn get_as<T>(&self, repr: &(impl ReadAs<T> + ?Sized)) -> Result<T, ReadError> {
        repr.read_as(self)
    }

    fn labels<L: Labeled + ?Sized>(&self, expr: &L) -> Result<Vec<&'static str>, ReadError> {
        crate::label::true_tags(self, &expr.roots())
    }
}

/// Shapes whose value converts into `T`. Used by [`ModelExt::get_as`].
pub trait ReadAs<T> {
    fn read_as(&self, model: &Model) -> Result<T, ReadError>;
}

impl<T, R> ReadAs<T> for R
where
    R: Readback + ?Sized,
    T: FromValue<R::Value>,
{
    fn read_as(&self, model: &Model) -> Result<T, ReadError> {
        T::from_value(self.read(model)?)
    }
}

/// Element-wise conversion of a readback value, e.g. `[i64; 3]` → `[usize; 3]`.
pub trait FromValue<V>: Sized {
    fn from_value(value: V) -> Result<Self, ReadError>;
}

impl FromValue<bool> for bool {
    fn from_value(value: bool) -> Result<Self, ReadError> {
        Ok(value)
    }
}

macro_rules! from_value_int {
    ($($t:ty),+) => {$(
        impl FromValue<i64> for $t {
            fn from_value(value: i64) -> Result<Self, ReadError> {
                <$t>::try_from(value).map_err(|_| ReadError::Convert {
                    value: value.to_string(),
                    target: stringify!($t),
                })
            }
        }
    )+};
}
from_value_int!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

impl<V, T: FromValue<V>> FromValue<Vec<V>> for Vec<T> {
    fn from_value(value: Vec<V>) -> Result<Self, ReadError> {
        value.into_iter().map(T::from_value).collect()
    }
}

impl<V, T: FromValue<V>, const N: usize> FromValue<[V; N]> for [T; N] {
    fn from_value(value: [V; N]) -> Result<Self, ReadError> {
        Vec::from_value(Vec::from(value))?
            .try_into()
            .map(Ok)
            .unwrap_or_else(|_| unreachable!("from_value keeps the length"))
    }
}

macro_rules! from_value_tuple {
    ($($t:ident $v:ident $i:tt),+) => {
        impl<$($v, $t: FromValue<$v>),+> FromValue<($($v,)+)> for ($($t,)+) {
            fn from_value(value: ($($v,)+)) -> Result<Self, ReadError> {
                Ok(($($t::from_value(value.$i)?,)+))
            }
        }
    };
}
from_value_tuple!(T1 V1 0);
from_value_tuple!(T1 V1 0, T2 V2 1);
from_value_tuple!(T1 V1 0, T2 V2 1, T3 V3 2);
from_value_tuple!(T1 V1 0, T2 V2 1, T3 V3 2, T4 V4 3);

/// "At least one of `vars` takes a different value than in `model`."
///
/// Asserting it blocks that solution, so looping finds every solution (projected onto `vars`).
/// Inside `constraints!`, an array literal `[x, y]` is borrowed, so the vars aren't moved.
///
/// ## Examples
/// ```
/// # use z3::Solver; use z3_toolkit::prelude::*;
/// let [x, y] = ints("v");
/// let s = Solver::new();
/// constraints! { s; x in 0..3, y in 0..3, x < y };
/// let mut count = 0;
/// while let Ok(model) = s.solve() {
///     count += 1;
///     constraints! { s; differs(&model, [x, y]) }
/// }
/// assert_eq!(count, 3);
/// ```
pub fn differs<'a, A: Ast + 'a>(model: &Model, vars: impl IntoIterator<Item = &'a A>) -> Bool {
    let ne: Vec<Bool> = vars
        .into_iter()
        .map(|v| v.ne(model.eval(v, true).expect("model can evalute every var")))
        .collect();
    Bool::or(&ne)
}
