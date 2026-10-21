use crate::{Binds, Role, Space};
use z3::ast::Bool;
use z3_toolkit::prelude::*;

/// `Def`s visible from the reference `i`.
pub(crate) fn reaching_defs(binds: &Binds, i: usize) -> impl Iterator<Item = usize> + '_ {
    let space = binds.space[i];
    (0..i).filter(move |&k| {
        binds.role[k] == Role::Def && binds.space[k] == space && binds.encloses(k, i)
    })
}

/// `Kill`s between `k` and `i` in the same namespace as `i`.
pub(crate) fn kills_between(binds: &Binds, k: usize, i: usize) -> impl Iterator<Item = usize> + '_ {
    let space = binds.space[i];
    (k + 1..i).filter(move |&j| binds.role[j] == Role::Kill && binds.space[j] == space)
}

/// Hole `i` (a reference) points to alive visible name.
pub fn visible(binds: &Binds, i: usize) -> Bool {
    let alive: Vec<Bool> = reaching_defs(binds, i)
        .map(|k| {
            let kills: Vec<Bool> = kills_between(binds, k, i)
                .map(|j| constraint!(binds.name[j] == binds.name[i]))
                .collect();
            constraint!(binds.name[k] == binds.name[i], !Bool::or(&kills))
        })
        .collect();
    Bool::or(&alive)
}

/// All references in `space` point to visible names
pub fn bound(binds: &Binds, space: Space) -> Bool {
    constraint!(all { for i in binds.refs(space) { visible(binds, i) } })
}

/// Exactly `n` references in `space` point to names that are not visible.
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

/// The name in hole `i` was visible earlier but has been `Kill`ed.
pub fn killed(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .flat_map(|k| kills_between(binds, k, i).map(move |j| (k, j)))
        .map(|(k, j)| {
            constraint!(
                binds.name[k] == binds.name[i],
                binds.name[j] == binds.name[i]
            )
        })
        .collect();
    constraint!(Bool::or(&cases), !visible(binds, i))
}

/// The names of the `Def`s in `space` are pairwise distinct.
///
/// To include reassignments, use `!distinct_defs(binds, space)`.
pub fn distinct_defs(binds: &Binds, space: Space) -> Bool {
    binds
        .name
        .distinct_among(|k| binds.role[k] == Role::Def && binds.space[k] == space)
}
