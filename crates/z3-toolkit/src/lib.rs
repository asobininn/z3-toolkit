extern crate self as z3_toolkit;

mod engine;
mod lin;
mod ty;

#[doc(hidden)]
pub mod __private;

pub use engine::{Engine, Failure};
pub use lin::{dot, sum};
pub use ty::{ModelExt, Z3Type, bools, ints};
pub use z3_toolkit_derive::{constraint, constraints, lin};

pub mod prelude {
    pub use crate::{
        Engine, Failure, ModelExt, Z3Type, bools, constraint, constraints, dot, ints, lin, sum,
    };
}
