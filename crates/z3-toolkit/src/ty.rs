use z3::{
    Model,
    ast::{Bool, Int},
};

/// Linking Rust and Z3 types
pub trait Z3Type: Sized {
    type Repr;

    /// Extract values from the model
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

/// todo
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

/// todo
pub fn fresh<T: Fresh>(prefix: &str) -> T {
    T::fresh(prefix)
}

/// todo
pub fn fresh_n<T: Fresh>(prefix: &str, len: usize) -> Vec<T> {
    std::iter::repeat_with(|| T::fresh(prefix))
        .take(len)
        .collect()
}

/// todo
pub fn ints<const N: usize>(prefix: &str) -> [Int; N] {
    fresh(prefix)
}

/// todo
pub fn bools<const N: usize>(prefix: &str) -> [Bool; N] {
    fresh(prefix)
}

/// todo
pub fn ints_n(prefix: &str, len: usize) -> Vec<Int> {
    fresh_n(prefix, len)
}

/// todo
pub fn bools_n(prefix: &str, len: usize) -> Vec<Bool> {
    fresh_n(prefix, len)
}

/// Extract values from the model
pub trait ModelExt {
    fn get<T: Z3Type>(&self, repr: &T::Repr) -> Option<T>;
}

impl ModelExt for Model {
    fn get<T: Z3Type>(&self, repr: &T::Repr) -> Option<T> {
        T::eval(self, repr)
    }
}
