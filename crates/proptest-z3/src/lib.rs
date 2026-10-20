mod binds;
mod frag;
mod label;
mod names;
pub mod rules;
mod solve;
mod strategy;

pub use binds::Binds;
pub use frag::{Frag, Hint, IntoFrag, Role, Skeleton, Space, Text};
pub use label::Label;
pub use names::{NameSpec, Names};
pub use solve::{Fill, FilledHole, Hole, Solved};
pub use strategy::{FillExt, Filled};

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
        Binds, Fill, FillExt, Filled, Frag, Hint, Hole, IntoFrag, NameSpec, Names, Role, Skeleton,
        Solved, Space, Text,
    };
    pub use z3::ast::{Bool, Int};
    pub use z3_toolkit::prelude::*;
}
