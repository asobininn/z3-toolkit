extern crate self as z3_toolkit;

mod engine;
mod lin;
mod ty;

#[doc(hidden)]
pub mod __private;

pub use engine::{Engine, Failure, Solutions};
pub use lin::{abs, dot, max, min, sum};
pub use ty::{Fresh, ModelExt, Z3Type, bools, bools_n, differs, fresh, fresh_n, ints, ints_n};
pub use z3_toolkit_derive::{constraint, constraints, lin};

pub mod prelude {
    pub use crate::{
        Engine, Failure, Fresh, ModelExt, Solutions, Z3Type, abs, bools, bools_n, constraint,
        constraints, differs, dot, fresh, fresh_n, ints, ints_n, lin, max, min, sum,
    };
}
