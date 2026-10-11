//! An ergonomic constraint DSL for the [`z3`] crate.
//!
//! It keeps z3's own `Int`, `Bool`, `Solver` and `Optimize`, and adds one-line variable
//! declarations, linear helpers, solution enumeration, and typed readback around them.
//! Only `bool` / `i64` (Z3 `Bool` / `Int`) are supported for now.
//!
//! ## Example
//!
#![doc = concat!("```\n", include_str!("../examples/choose_member.rs"), "```")]
//!
//! ## More examples
#![doc = include_str!("../examples.md")]

extern crate self as z3_toolkit;

mod engine;
mod error;
mod label;
mod lin;
mod sym;
mod ty;

#[doc(hidden)]
pub mod __private;

pub use engine::{Engine, Solutions};
pub use error::{Failure, ReadError};
pub use label::Labeled;
pub use lin::{abs, dot, max, min, sum};
pub use sym::{Symbolic, SymbolicSlice, ToSymbolic, ite};
pub use ty::{
    Fresh, FromValue, ModelExt, ReadAs, Readback, bools, bools_n, differs, fresh, fresh_n, ints,
    ints_n,
};
pub use z3_toolkit_derive::{constraint, constraints, lin};

pub mod prelude {
    pub use crate::{
        Engine, Failure, Fresh, ModelExt, ReadError, Readback, Solutions, Symbolic, SymbolicSlice,
        ToSymbolic, abs, bools, bools_n, constraint, constraints, differs, dot, fresh, fresh_n,
        ints, ints_n, ite, lin, max, min, sum,
    };
}
