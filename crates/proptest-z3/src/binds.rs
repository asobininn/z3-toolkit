use std::cell::RefCell;

use z3::ast::{Bool, Int};
use z3_toolkit::prelude::*;

use crate::{
    Label, Role, Space,
    frag::Frag,
    label::{Records, Tag},
};

pub struct Binds {
    /// Roles, in evaluation order.
    pub role: Vec<Role>,
    /// Namespaces, in evaluation order.
    pub space: Vec<Space>,
    /// The names to fill in (indices into the candidates), in evaluation order.
    pub name: Vec<Int>,
    pub(crate) hole: Vec<usize>,
    step: Vec<usize>,
    pub(crate) records: RefCell<Records>,
}

impl Binds {
    pub(crate) fn new(frag: &Frag) -> Self {
        let n = frag.order.len();
        let info = |i: usize| &frag.holes[frag.order[i]];
        Self {
            role: (0..n).map(|i| info(i).role).collect(),
            space: (0..n).map(|i| info(i).space).collect(),
            name: ints_n("name", n),
            hole: frag.order.clone(),
            step: (0..n)
                .map(|i| info(i).step.expect("closed by Skeleton::steps"))
                .collect(),
            records: RefCell::default(),
        }
    }

    /// Number of holes.
    pub fn len(&self) -> usize {
        self.role.len()
    }

    pub fn is_empty(&self) -> bool {
        self.role.is_empty()
    }

    /// Holes in `space` with role `role` (positions in evalution order).
    pub fn of(&self, space: Space, role: Role) -> impl Iterator<Item = usize> + '_ {
        (0..self.len()).filter(move |&i| self.space[i] == space && self.role[i] == role)
    }

    /// `Def` in `space`.
    pub fn defs(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        self.of(space, Role::Def)
    }

    /// `Use`s in `space`.
    pub fn uses(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        self.of(space, Role::Use)
    }

    /// References in `space`. the holes that must point to a live name.
    pub fn refs(&self, space: Space) -> impl Iterator<Item = usize> + '_ {
        (0..self.len()).filter(move |&i| self.space[i] == space && self.role[i].is_ref())
    }

    /// Whether `p` and `q` belong to the same step.
    pub fn same_step(&self, p: usize, q: usize) -> bool {
        self.step[p] == self.step[q]
    }

    /// Records an expression starting that hole `i` breaks rule `kind`, and return it unchanged.
    pub fn mark<L: Label>(&self, i: usize, kind: L, broken: Bool) -> Bool {
        self.records
            .borrow_mut()
            .labels
            .push((Some(i), Tag::new(kind), broken.clone()));
        broken
    }
}
