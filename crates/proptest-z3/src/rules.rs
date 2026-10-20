use crate::{Binds, Role, Space};
use z3::ast::Bool;
use z3_toolkit::prelude::*;

fn reaching_defs(binds: &Binds, i: usize) -> impl Iterator<Item = usize> + '_ {
    let space = binds.space[i];
    (0..i).filter(move |&k| {
        binds.role[k] == Role::Def && binds.space[k] == space && binds.encloses(k, i)
    })
}

/// Hole `i` (a reference) points to alive visible name.
pub fn visible(binds: &Binds, i: usize) -> Bool {
    let alive: Vec<Bool> = reaching_defs(binds, i)
        .map(|k| constraint!(binds.name[k] == binds.name[i]))
        .collect();
    Bool::or(&alive)
}

/// All references in `space` point to visible names
pub fn bound(binds: &Binds, space: Space) -> Bool {
    constraint!(all { for i in binds.refs(space) { visible(binds, i) } })
}

/// Exaclty `n` references in `space` point to names that are not visible.
pub fn unbound(binds: &Binds, space: Space, n: usize) -> Bool {
    constraint!(exactly(n) { for i in binds.refs(space) { !visible(binds, i) } })
}

/// There is a `Def` of the same name in the same step as hole `i`.
pub fn defined_in_same_step(binds: &Binds, i: usize) -> Bool {
    let same: Vec<Bool> = binds
        .defs(binds.space[i])
        .filter(|&k| k != i && binds.same_step(k, i))
        .map(|k| constraint!(binds.name[k] == binds.name[i]))
        .collect();
    Bool::or(&same)
}
