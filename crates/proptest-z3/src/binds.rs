use std::cell::RefCell;

use z3::ast::{Bool, Int};
use z3_toolkit::prelude::*;

use crate::{
    Label, Role, Space,
    frag::Frag,
    label::{Records, Tag},
};

/// The holes of a skeleton in **evaluation order**, passed to a rule.
///
/// Inside a rule (`Fn(&Binds) -> Bool`), a hole is referred to by its position `i` (`usize`) in
/// evaluation order. A smaller position is evaluated earlier, so "holes before `i`" can be written
/// as a comparison. See [`rules`](crate::rules) for how the evaluation order is determined.
///
/// - These positions are different from the [`Hole`](crate::Hole)s (text order) of a solved result.
///   [`Solved::events`](crate::Solved::events) maps between them (`events()[i]` is the hole at
///   position `i` in evaluation order)
/// - A rule cannot produce a [`Hole`](crate::Hole). To locate a hole, mark it with [`Binds::mark`]
/// - Positions are obtained with [`defs`](Binds::defs) / [`uses`](Binds::uses) /
///   [`kills`](Binds::kills) / [`refs`](Binds::refs) / [`of`](Binds::of), and names are
///   represented by [`name`](Binds::name)
/// - Steps and scopes are fixed when the structure is generated, so [`same_step`](Binds::same_step)
///   / [`encloses`](Binds::encloses) / [`boundaries_between`](Binds::boundaries_between) return
///   Rust values rather than Z3 expressions
///
/// A rule is called every time a skeleton is solved, including each shrinking step. Records such as
/// [`mark`](Binds::mark) are rebuilt on each call, so keep rules free of side effects.
pub struct Binds {
    /// Roles, in evaluation order.
    pub role: Vec<Role>,
    /// The namespace of each hole, in evaluation order.
    pub space: Vec<Space>,
    /// Z3 integer variables for the names filled into the holes, in evaluation order.
    pub name: Vec<Int>,
    pub(crate) hole: Vec<usize>,
    step: Vec<usize>,
    /// Scopes enclosing this one, innermost first. Empty at the top level (outside any scope).
    scopes: Vec<Vec<usize>>,
    /// Whether each scope is a boundary.
    boundary: Vec<bool>,
    pub(crate) records: RefCell<Records>,
}

impl Binds {
    pub(crate) fn new(frag: &Frag) -> Self {
        let n = frag.order.len();
        let info = |i: usize| &frag.holes[frag.order[i]];
        let path =
            |s: Option<usize>| std::iter::successors(s, |&cur| frag.scope_parent[cur]).collect();
        Self {
            role: (0..n).map(|i| info(i).role).collect(),
            space: (0..n).map(|i| info(i).space).collect(),
            name: ints_n("name", n),
            hole: frag.order.clone(),
            step: (0..n)
                .map(|i| info(i).step.expect("closed by Skeleton::steps"))
                .collect(),
            scopes: (0..n).map(|i| path(info(i).scope)).collect(),
            boundary: frag.scope_boundary.clone(),
            records: RefCell::default(),
        }
    }

    /// Returns the number of holes.
    pub fn len(&self) -> usize {
        self.role.len()
    }

    /// Returns `true` if there are no holes.
    pub fn is_empty(&self) -> bool {
        self.role.is_empty()
    }

    /// Returns the positions of the holes in `space` with role `role`, in evaluation order.
    ///
    /// Scopes do not matter; the whole skeleton is searched.
    pub fn of(&self, space: Space, role: Role) -> impl Iterator<Item = usize> + '_ {
        (0..self.len()).filter(move |&i| self.space[i] == space && self.role[i] == role)
    }

    /// Returns the positions of the definitions (`Def`) in `space`, in evaluation order.
    /// Same as `of(space, Role::Def)`.
    pub fn defs(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        self.of(space, Role::Def)
    }

    /// Returns the positions of the `Use`s in `space`, in evaluation order.
    /// Same as `of(space, Role::Use)`.
    pub fn uses(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        self.of(space, Role::Use)
    }

    /// Returns the positions of the kills (`Kill`) in `space`, in evaluation order.
    /// Same as `of(space, Role::Kill)`.
    pub fn kills(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        self.of(space, Role::Kill)
    }

    /// Returns the positions of the references (`Use` and `Kill`) in `space`, in evaluation order.
    pub fn refs(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        (0..self.len()).filter(move |&i| self.space[i] == space && self.role[i].is_ref())
    }

    /// Returns whether holes `p` and `q` belong to the same step.
    pub fn same_step(&self, p: usize, q: usize) -> bool {
        self.step[p] == self.step[q]
    }

    /// Returns whether the scope of hole `outer` encloses hole `inner`,
    ///
    /// In other words, whether a name defined at `outer` is within sight of `inner` (evaluation
    /// order and kills are not considered).
    ///
    /// - Always `true` if `outer` is at the top level (outside any scope)
    /// - `true` if `outer` and `inner` are in the same scope
    pub fn encloses(&self, outer: usize, inner: usize) -> bool {
        match self.scopes[outer].first() {
            Some(s) => self.scopes[inner].contains(s),
            None => true,
        }
    }

    /// Returns the number of boundaries ([`Frag::boundary`](crate::Frag::boundary)) that enclose hole
    /// `inner` but not hole `outer`.
    ///
    /// That is, the number of boundaries (closure or function bodies) crossed from the definition
    /// `outer` to the reference `inner`. Plain scopes ([`Frag::scope`](crate::Frag::scope)) are not
    /// counted. Use it when `encloses(outer, inner)` is true.
    ///
    /// Captures are decided by [`captured`](crate::rules::captured); use this when the condition
    /// depends on how many boundaries are crossed.
    pub fn boundaries_between(&self, outer: usize, inner: usize) -> usize {
        (self.scopes[inner].iter())
            .filter(|&&s| self.boundary[s] && !self.scopes[outer].contains(&s))
            .count()
    }

    /// Records "if `broken` holds, hole `i` is broken as `kind`", and returns `broken` unchanged.
    ///
    /// - `i` is a position in evaluation order
    /// - It becomes a constraint only when the returned value is put into a constraint
    ///   (`exactly(1) { .. }` and so on). Otherwise it is an [`observe`](Binds::observe) attached
    ///   to a hole, and only records a classification
    /// - Its truth is evaluated after solving. Marks that turned out true are returned by
    ///   [`Solved::marks`](crate::Solved::marks) as `(Hole, kind)`
    /// - When conditions overlap, one hole may get several marks. Separate them by adding
    ///   preconditions with [`Given::given`](crate::Given::given)
    ///
    /// # Examples
    ///
    /// ```
    /// # use proptest::prelude::*; use proptest_z3::prelude::*;
    /// const VAR: Space = Space::new("var");
    /// #[derive(Debug, Clone, Copy, PartialEq)]
    /// struct Undefined;
    ///
    /// // Exactly one reference is not visible; mark that hole as Undefined
    /// let rule = |binds: &Binds| constraint!(
    ///     exactly(1) { for i in binds.refs(VAR) {
    ///         binds.mark(i, Undefined, !visible(binds, i))
    ///     } },
    /// );
    /// ```
    pub fn mark<L: Label>(&self, i: usize, kind: L, broken: Bool) -> Bool {
        self.records
            .borrow_mut()
            .labels
            .push((Some(i), Tag::new(kind), broken.clone()));
        broken
    }

    /// Records only whether `cond` held in the solved result, without making it a constraint.
    ///
    /// Classifies inputs without changing their distribution. The kinds that turned out true are
    /// available through [`Solved::labels`](crate::Solved::labels) /
    /// [`Solved::has`](crate::Solved::has).
    ///
    /// It returns nothing, so it cannot be written inside `constraint!`. Call it first in the rule
    /// body and return the constraint separately.
    ///
    /// # Examples
    ///
    /// ```
    /// # use proptest_z3::prelude::*;
    /// const VAR: Space = Space::new("var");
    /// #[derive(Debug, Clone, Copy, PartialEq)]
    /// enum Shape { Redefined }
    ///
    /// let rule = |binds: &Binds| {
    ///     binds.observe(Shape::Redefined, !distinct_defs(binds, VAR));
    ///     bound(binds, VAR)
    /// };
    /// ```
    pub fn observe<L: Label>(&self, kind: L, cond: Bool) {
        self.records
            .borrow_mut()
            .labels
            .push((None, Tag::new(kind), cond));
    }

    /// Adds a named constraint. Use it to find out why generation is rejected.
    ///
    /// `constraint` is added as a hard constraint, just like the value returned by the rule. When
    /// solving fails, the label names of the constraints in the unsat core are counted as rejection
    /// reasons, available through [`take_rejections`](crate::take_rejections).
    ///
    /// - Calling it several times with the same label groups the constraints under one name (calling
    ///   it per hole in a loop still shows the name only once in a reason)
    /// - The value returned by the rule and the range constraints on names are unnamed and never
    ///   appear in the unsat core. In a conflict with unnamed constraints, only the named side shows
    ///   up, so a reason with a single name does not mean that constraint is unsatisfiable on its
    ///   own. Put every constraint you want to investigate into a `require`
    /// - Do not `require` a constraint and also include it in the value returned by the rule. The
    ///   unnamed copy alone can explain the conflict, so the label disappears from the reason. This
    ///   is why it returns nothing and cannot be written inside `constraint!`; call it in the rule body
    /// - The core returned by Z3 is not necessarily minimal. Remove suspicious constraints one by
    ///   one to confirm
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use proptest_z3::prelude::*;
    /// const VAR: Space = Space::new("var");
    ///
    /// let rule = |binds: &Binds| {
    ///     binds.require("all_bound", bound(binds, VAR));
    ///     binds.require("no_redefinition", distinct_defs(binds, VAR));
    ///     Bool::from_bool(true)
    /// };
    /// ```
    pub fn require<L: Label>(&self, label: L, constraint: Bool) {
        self.records
            .borrow_mut()
            .required
            .push((Tag::new(label).name, constraint));
    }
}
