//! An extension of [proptest] for generating test inputs whose elements depend on each other.
//!
//! You define the structure of an input with ordinary proptest Strategies, and put **holes** where
//! names go. Which name goes into which hole is decided by Z3 constraints.
//! By flipping a constraint, you can generate inputs that contain a given number of violations,
//! and you can also locate where the violations are.
//!
//! The [Book] / [Book(日本語版)] explains how to use it step by step, using a small
//! language as an example.
//!
//! # Examples
//!
//! Testing the name resolver `undefined_refs` of a small language with one statement per line
//! (`let x` / `let x = y` / `print x`). It returns the positions where undefined names are referred to.
//!
//! ```
//! # use std::collections::HashSet;
//! use proptest::prelude::*;
//! use proptest_z3::prelude::*;
//!
//! const VAR: Space = Space::new("var");
//!
//! // Put holes where names go, and make each statement one step
//! fn program() -> impl Strategy<Value = Skeleton> {
//!     let stmt = prop_oneof![
//!         VAR.def().prop_map(|x| frag!["let ", x]),
//!         (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["let ", x, " = ", y]),
//!         VAR.refer().prop_map(|y| frag!["print ", y]),
//!     ];
//!     prop::collection::vec(stmt, 1..8)
//!         .prop_map(|stmts| Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"])))
//! }
//!
//! #[derive(Debug, Clone, Copy, PartialEq)]
//! struct Undefined;
//!
//! proptest! {
//!     // In an input with exactly one reference that is not visible, is its position reported?
//!     #[test]
//!     fn undefined_name_is_reported(s in program().fill(
//!         NameSpec::new().space(VAR, "[a-c]", 3),
//!         |binds| constraint!(exactly(1) { for i in binds.refs(VAR) {
//!             binds.mark(i, Undefined, !visible(binds, i))
//!         } }),
//!     )) {
//!         let [(hole, Undefined)] = s.marks::<Undefined>()[..] else { panic!("{s:?}") };
//!         prop_assert_eq!(undefined_refs(&s.text), vec![s.span(hole).start], "{:?}", s);
//!     }
//! }
//! # fn undefined_refs(source: &str) -> Vec<usize> {
//! #     let offset = |word: &str| word.as_ptr() as usize - source.as_ptr() as usize;
//! #     let mut defined = HashSet::new();
//! #     let mut errors = vec![];
//! #     for line in source.lines() {
//! #         match line.split_whitespace().collect::<Vec<_>>()[..] {
//! #             ["let", x] => { defined.insert(x); }
//! #             ["let", x, "=", y] => {
//! #                 if !defined.contains(y) { errors.push(offset(y)); }
//! #                 defined.insert(x);
//! #             }
//! #             ["print", y] => if !defined.contains(y) { errors.push(offset(y)); },
//! #             ref other => panic!("unexpected line: {other:?}"),
//! #         }
//! #     }
//! #     errors
//! # }
//! ```
//!
//! The Z3 library is required. See the README of the z3 crate for how to install it.
//!
//! [proptest]: https://crates.io/crates/proptest
//! [Book]: https://asobininn.github.io/z3-toolkit/book/proptest-z3-book
//! [Book(日本語版)]: https://asobininn.github.io/z3-toolkit/book/proptest-z3-book-ja
mod binds;
mod frag;
mod label;
mod names;
pub mod rules;
mod solve;
mod strategy;

pub use binds::Binds;
pub use frag::{Frag, Hint, IntoFrag, Role, Skeleton, Space, Text};
pub use label::{Given, Label, LabelStat, label_stats, take_label_stats, take_rejections};
pub use names::{NameSpec, Names};
pub use solve::{Fill, FilledHole, Hole, Solved};
pub use strategy::{FillExt, Filled};

/// Joins the given values in order into a single [`Frag`].
///
/// `frag![e1, e2, ..]` is the same as `Frag::concat([e1.into_frag(), e2.into_frag(), ..])`.
/// Any value implementing [`IntoFrag`] can be passed (strings, numbers, `Frag`, `Vec`, `Option`,
/// tuples of up to 4 elements, [`Text`]). A trailing comma is allowed.
///
/// It does not create a step. The joined fragment becomes a single step when it is later passed
/// to [`Skeleton::steps`] or [`Frag::scope`].
///
/// # Examples
///
/// ```
/// # use proptest::prelude::*; use proptest_z3::prelude::*;
/// const VAR: Space = Space::new("var");
///
/// let assign = (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["let ", x, " = ", y]);
/// ```
#[macro_export]
macro_rules! frag {
    ($($e:expr),* $(,)?) => {
        $crate::Frag::concat([$($crate::IntoFrag::into_frag($e)),*])
    };
}

pub mod prelude {
    pub use crate::frag;
    pub use crate::rules::*;
    pub use crate::{
        Binds, Fill, FillExt, Filled, Frag, Given, Hint, Hole, IntoFrag, NameSpec, Names, Role,
        Skeleton, Solved, Space, Text, label_stats,
    };
    pub use z3::ast::{Bool, Int};
    pub use z3_toolkit::prelude::*;
}
