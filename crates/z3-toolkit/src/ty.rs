use z3::{
    Model,
    ast::{Ast, Bool, Int},
};

/// Rust type ↔ Z3 type: `bool` ↔ `Bool`, `i64` ↔ `Int`, nested in `Vec` / `[T; N]`.
pub trait Z3Type: Sized {
    type Repr;

    /// Extract values from the model. Unconstrained variables get a default.
    fn eval(model: &Model, repr: &Self::Repr) -> Option<Self>;
}

impl Z3Type for bool {
    type Repr = Bool;
    fn eval(model: &Model, repr: &Self::Repr) -> Option<Self> {
        model.eval(repr, true)?.as_bool()
    }
}

impl Z3Type for i64 {
    type Repr = Int;
    fn eval(model: &Model, repr: &Self::Repr) -> Option<Self> {
        model.eval(repr, true)?.as_i64()
    }
}

impl<T: Z3Type> Z3Type for Vec<T> {
    type Repr = Vec<T::Repr>;
    fn eval(model: &Model, repr: &Self::Repr) -> Option<Self> {
        repr.iter().map(|r| T::eval(model, r)).collect()
    }
}

impl<T: Z3Type, const N: usize> Z3Type for [T; N] {
    type Repr = [T::Repr; N];
    fn eval(model: &Model, repr: &Self::Repr) -> Option<Self> {
        let v: Option<Vec<T>> = repr.iter().map(|r| T::eval(model, r)).collect();
        v?.try_into().ok()
    }
}

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
    /// `None` if a value doesn't fit in `T` (e.g. outside `i64`).
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let xs = ints_n("x", 2);
    /// let s = Solver::new();
    /// constraints! { s; xs[0] == 1, xs[1] == 2 };
    /// let m = s.solve().unwrap();
    /// assert_eq!(m.get::<Vec<i64>>(&xs), Some(vec![1, 2]));
    /// ```
    fn get<T: Z3Type>(&self, repr: &T::Repr) -> Option<T>;
}

impl ModelExt for Model {
    fn get<T: Z3Type>(&self, repr: &T::Repr) -> Option<T> {
        T::eval(self, repr)
    }
}

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
