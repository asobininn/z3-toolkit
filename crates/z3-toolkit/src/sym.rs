use z3::ast::{Bool, Int};

/// Shapes with a value-level if-then-else: `Int`, `Bool`, `[T; N]`, `Vec<T>` and tuples up to 4.
pub trait Symbolic: Sized {
    /// `Vec`s of different length panic.
    fn ite(cond: &Bool, then: &Self, otherwise: &Self) -> Self;
}

impl Symbolic for Int {
    fn ite(cond: &Bool, then: &Self, otherwise: &Self) -> Self {
        cond.ite(then, otherwise)
    }
}

impl Symbolic for Bool {
    fn ite(cond: &Bool, then: &Self, otherwise: &Self) -> Self {
        cond.ite(then, otherwise)
    }
}

impl<T: Symbolic, const N: usize> Symbolic for [T; N] {
    fn ite(cond: &Bool, then: &Self, otherwise: &Self) -> Self {
        std::array::from_fn(|i| T::ite(cond, &then[i], &otherwise[i]))
    }
}

impl<T: Symbolic> Symbolic for Vec<T> {
    fn ite(cond: &Bool, then: &Self, otherwise: &Self) -> Self {
        assert_eq!(
            then.len(),
            otherwise.len(),
            "`ite` on Vecs of different lengths"
        );
        then.iter()
            .zip(otherwise)
            .map(|(t, o)| T::ite(cond, t, o))
            .collect()
    }
}

macro_rules! symbolic_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: Symbolic),+> Symbolic for ($($t,)+) {
            fn ite(cond: &Bool, then: &Self, otherwise: &Self) -> Self {
                ($($t::ite(cond, &then.$i, &otherwise.$i),)+)
            }
        }
    };
}
symbolic_tuple!(T1 0);
symbolic_tuple!(T1 0, T2 1);
symbolic_tuple!(T1 0, T2 1, T3 2);
symbolic_tuple!(T1 0, T2 1, T3 2, T4 3);

/// Values usable where a [`Symbolic`] is expected: the shape itself, or Rust ints / `bool` in it.
pub trait ToSymbolic {
    type Output: Symbolic;
    fn to_symbolic(&self) -> Self::Output;
}

impl ToSymbolic for Int {
    type Output = Int;
    fn to_symbolic(&self) -> Self::Output {
        self.clone()
    }
}

impl ToSymbolic for Bool {
    type Output = Bool;
    fn to_symbolic(&self) -> Self::Output {
        self.clone()
    }
}

impl ToSymbolic for bool {
    type Output = Bool;
    fn to_symbolic(&self) -> Self::Output {
        Bool::from_bool(*self)
    }
}

macro_rules! to_symbolic_int {
    ($($t:ty),*) => {$(
        impl ToSymbolic for $t {
            type Output = Int;
            fn to_symbolic(&self) -> Self::Output {
                Int::from_i64(i64::try_from(*self).expect("integer does not fit in i64"))
            }
        }
    )*};
}
to_symbolic_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl<T: ToSymbolic + ?Sized> ToSymbolic for &T {
    type Output = T::Output;
    fn to_symbolic(&self) -> Self::Output {
        (**self).to_symbolic()
    }
}

impl<T: ToSymbolic> ToSymbolic for [T] {
    type Output = Vec<T::Output>;
    fn to_symbolic(&self) -> Self::Output {
        self.iter().map(T::to_symbolic).collect()
    }
}

impl<T: ToSymbolic, const N: usize> ToSymbolic for [T; N] {
    type Output = [T::Output; N];
    fn to_symbolic(&self) -> Self::Output {
        std::array::from_fn(|i| self[i].to_symbolic())
    }
}

impl<T: ToSymbolic> ToSymbolic for Vec<T> {
    type Output = Vec<T::Output>;
    fn to_symbolic(&self) -> Self::Output {
        self.as_slice().to_symbolic()
    }
}

macro_rules! to_symbolic_tuple {
    ($($t:ident $i:tt),+) => {
        impl<$($t: ToSymbolic),+> ToSymbolic for ($($t,)+) {
            type Output = ($($t::Output,)+);
            fn to_symbolic(&self) -> Self::Output {
                ($(self.$i.to_symbolic(),)+)
            }
        }
    };
}
to_symbolic_tuple!(T1 0);
to_symbolic_tuple!(T1 0, T2 1);
to_symbolic_tuple!(T1 0, T2 1, T3 2);
to_symbolic_tuple!(T1 0, T2 1, T3 2, T4 3);

/// `if cond { then } else { otherwise }`, following the shape. Branches take z3 or Rust values.
///
/// ## Examples
/// ```
/// # use z3_toolkit::prelude::*;
/// let ([p], [x]) = (bools("p"), ints("x"));
/// let y = ite(&p, &x, &0);
/// ```
pub fn ite<T: Symbolic, L, R>(cond: &Bool, then: &L, otherwise: &R) -> T
where
    L: ToSymbolic<Output = T> + ?Sized,
    R: ToSymbolic<Output = T> + ?Sized,
{
    T::ite(cond, &then.to_symbolic(), &otherwise.to_symbolic())
}

/// Read / Write at a symbolic index. Out-of-range results are unspecified; constrain `idx` yourself.
pub trait SymbolicSlice {
    type Elem: Symbolic;
    type Owned;

    /// `self[idx]`. Panics on an empty slice.
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let [i] = ints("i");
    /// let s = Solver::new();
    /// constraints! { s; i in 0..3, [10, 20, 30].at(&i) == 20 };
    /// assert_eq!(s.solve().unwrap().get(&i), Ok(1));
    /// ```
    fn at<I: ToSymbolic<Output = Int> + ?Sized>(&self, idx: &I) -> Self::Elem;

    /// Copy of `self` with `self[idx] = value`. Index and value take z3 or Rust values.
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let [i] = ints("i");
    /// let s = Solver::new();
    /// constraints! { s; i in 0..3, [1, 2, 3].with(&i, &0) == [1, 0, 3] };
    /// assert_eq!(s.solve().unwrap().get(&i), Ok(1));
    /// ```
    fn with<I, V>(&self, idx: &I, value: &V) -> Self::Owned
    where
        I: ToSymbolic<Output = Int> + ?Sized,
        V: ToSymbolic<Output = Self::Elem> + ?Sized;
}

fn is_at(idx: &Int, i: usize) -> Bool {
    idx.eq(Int::from_u64(i as u64))
}

impl<E: ToSymbolic> SymbolicSlice for [E] {
    type Elem = E::Output;
    type Owned = Vec<E::Output>;

    fn at<I: ToSymbolic<Output = Int> + ?Sized>(&self, idx: &I) -> Self::Elem {
        let idx = idx.to_symbolic();
        let (last, init) = self.split_last().expect("`at` on an empty slice");
        init.iter()
            .enumerate()
            .rev()
            .fold(last.to_symbolic(), |acc, (i, e)| {
                Symbolic::ite(&is_at(&idx, i), &e.to_symbolic(), &acc)
            })
    }

    fn with<I, V>(&self, idx: &I, value: &V) -> Self::Owned
    where
        I: ToSymbolic<Output = Int> + ?Sized,
        V: ToSymbolic<Output = Self::Elem> + ?Sized,
    {
        let (idx, value) = (idx.to_symbolic(), value.to_symbolic());
        self.iter()
            .enumerate()
            .map(|(i, e)| Symbolic::ite(&is_at(&idx, i), &value, &e.to_symbolic()))
            .collect()
    }
}

impl<E: ToSymbolic, const N: usize> SymbolicSlice for [E; N] {
    type Elem = E::Output;
    type Owned = [E::Output; N];

    fn at<I: ToSymbolic<Output = Int> + ?Sized>(&self, idx: &I) -> Self::Elem {
        self.as_slice().at(idx)
    }

    fn with<I, V>(&self, idx: &I, value: &V) -> Self::Owned
    where
        I: ToSymbolic<Output = Int> + ?Sized,
        V: ToSymbolic<Output = Self::Elem> + ?Sized,
    {
        self.as_slice()
            .with(idx, value)
            .try_into()
            .unwrap_or_else(|_| unreachable!("with keeps the length"))
    }
}
