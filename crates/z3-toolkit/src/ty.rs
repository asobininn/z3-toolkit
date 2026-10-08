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
pub fn ints<const N: usize>(prefix: &str) -> [Int; N] {
    std::array::from_fn(|_| Int::fresh_const(prefix))
}

/// todo
pub fn bools<const N: usize>(prefix: &str) -> [Bool; N] {
    std::array::from_fn(|_| Bool::fresh_const(prefix))
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
