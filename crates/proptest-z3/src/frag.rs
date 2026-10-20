use std::fmt::Display;

use proptest::prelude::*;

pub type Frac16 = u16;

/// A namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Space(&'static str);

impl Space {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub fn name(self) -> &'static str {
        self.0
    }

    /// A strategy for a single hole hole with `role`.
    pub fn hole(self, role: Role) -> impl Strategy<Value = Frag> {
        any::<Hint>().prop_map(move |hint| Frag::hole(self, role, hint))
    }

    /// A hole that creates a name.
    pub fn def(self) -> impl Strategy<Value = Frag> {
        self.hole(Role::Def)
    }

    /// A hole that refers to a name.
    pub fn refer(self) -> impl Strategy<Value = Frag> {
        self.hole(Role::Use)
    }
}

/// The role of a hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Def,
    Use,
}

impl Role {
    pub fn is_ref(self) -> bool {
        matches!(self, Role::Use)
    }
}

/// The preferred name for a hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Hint(pub Frac16);

impl Hint {
    pub fn index(self, size: usize) -> usize {
        assert!(size > 0, "no candidates");
        // h `div` 65536
        ((size as u64 * self.0 as u64) >> Frac16::BITS) as usize
    }
}

impl Arbitrary for Hint {
    type Parameters = ();
    type Strategy = proptest::strategy::Map<proptest::num::u16::Any, fn(Frac16) -> Hint>;

    fn arbitrary_with(_: ()) -> Self::Strategy {
        any::<Frac16>().prop_map(Hint)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Piece {
    Text(String),
    Hole(usize),
}

#[derive(Debug, Clone)]
pub(crate) struct HoleInfo {
    pub space: Space,
    pub role: Role,
    pub hint: Hint,
}

// A fragment with holes, under construction.
#[derive(Debug, Clone, Default)]
pub struct Frag {
    pub(crate) pieces: Vec<Piece>,
    pub(crate) holes: Vec<HoleInfo>,
    pub(crate) order: Vec<usize>,
}

impl Frag {
    /// A fragment consisting of a single hole.
    pub fn hole(space: Space, role: Role, hint: Hint) -> Self {
        Self {
            pieces: vec![Piece::Hole(0)],
            holes: vec![HoleInfo { space, role, hint }],
            order: vec![0],
            ..Default::default()
        }
    }

    /// A fragment consisting only of text.
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            pieces: vec![Piece::Text(s.into())],
            ..Default::default()
        }
    }

    /// Concatenates fragments in order.
    pub fn concat(parts: impl IntoIterator<Item = Frag>) -> Self {
        let mut out = Frag::default();
        for part in parts {
            let hole_base = out.holes.len();
            out.pieces.extend(part.pieces.into_iter().map(|p| match p {
                Piece::Hole(h) => Piece::Hole(hole_base + h),
                text => text,
            }));
            out.holes.extend(part.holes);
            out.order
                .extend(part.order.into_iter().map(|h| hole_base + h));
        }
        out
    }

    pub fn join(
        sep: impl IntoFrag + Clone,
        parts: impl IntoIterator<Item = impl IntoFrag>,
    ) -> Self {
        let mut out = vec![];
        for (i, part) in parts.into_iter().enumerate() {
            if i > 0 {
                out.push(sep.clone().into_frag());
            }
            out.push(part.into_frag());
        }
        Frag::concat(out)
    }

    fn closed(steps: impl IntoIterator<Item = Frag>) -> Self {
        Frag::concat(steps.into_iter().map(Frag::close))
    }

    fn close(mut self) -> Self {
        let holes = &self.holes;
        self.order.sort_by_key(|&h| holes[h].role != Role::Use);
        self
    }
}

/// Things that can be placed in `frag!`.
pub trait IntoFrag {
    fn into_frag(self) -> Frag;
}

impl IntoFrag for Frag {
    fn into_frag(self) -> Frag {
        self
    }
}
impl IntoFrag for &Frag {
    fn into_frag(self) -> Frag {
        self.clone()
    }
}
impl<T: IntoFrag> IntoFrag for Vec<T> {
    fn into_frag(self) -> Frag {
        Frag::concat(self.into_iter().map(IntoFrag::into_frag))
    }
}
impl<T: IntoFrag> IntoFrag for Option<T> {
    fn into_frag(self) -> Frag {
        self.map(IntoFrag::into_frag).unwrap_or_default()
    }
}

/// A wrapper for placing a `Display` value in `frag!` as text.
#[derive(Debug, Clone, Copy)]
pub struct Text<T>(pub T);

impl<T: Display> IntoFrag for Text<T> {
    fn into_frag(self) -> Frag {
        Frag::text(self.0.to_string())
    }
}

macro_rules! text_impl {
    ($($t:ty),*) => {$(
        impl IntoFrag for $t {
            fn into_frag(self) -> Frag {
                Frag::text(self.to_string())
            }
        }
    )*};
}
text_impl!(&str, String, &String, char, bool);
text_impl!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

macro_rules! tuple_impl {
    ($($t:ident $i:tt),+) => {
        impl<$($t: IntoFrag),+> IntoFrag for ($($t,)+) {
            fn into_frag(self) -> Frag {
                Frag::concat([$(self.$i.into_frag()),+])
            }
        }
    };
}
tuple_impl!(T1 0, T2 1);
tuple_impl!(T1 0, T2 1, T3 2);
tuple_impl!(T1 0, T2 1, T3 2, T4 3);

/// A skeleton with holes, ready to be solved.
#[derive(Debug, Clone)]
pub struct Skeleton(pub(crate) Frag);

impl Skeleton {
    pub fn steps(steps: impl IntoIterator<Item = Frag>) -> Self {
        Skeleton(Frag::closed(steps))
    }

    pub fn holes(&self) -> usize {
        self.0.holes.len()
    }
}
