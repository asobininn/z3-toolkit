use crate::{Binds, Role, Space};
use z3::ast::Bool;
use z3_toolkit::prelude::*;

/// Candidate `Def`s for the reference `i`: those in the same namespace that come before `i`
/// in evaluation order and whose scope encloses `i`.
pub fn reaching_defs(binds: &Binds, i: usize) -> impl Iterator<Item = usize> + '_ {
    let space = binds.space[i];
    (0..i).filter(move |&k| {
        binds.role[k] == Role::Def && binds.space[k] == space && binds.encloses(k, i)
    })
}

/// `Kill`s strictly between `k` and `i` in evaluation order, in the namespace of `i`.
pub fn kills_between(binds: &Binds, k: usize, i: usize) -> impl Iterator<Item = usize> + '_ {
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

/// Reference `i` resolves to definition `k`: same name, no later candidate with the same
/// name, and no `Kill` of that name in between.
pub fn resolves_to(binds: &Binds, i: usize, k: usize) -> Bool {
    if !reaching_defs(binds, i).any(|q| q == k) {
        return Bool::from_bool(false);
    }
    let same = |j: usize| constraint!(binds.name[j] == binds.name[i]);
    let later: Vec<Bool> = reaching_defs(binds, i)
        .filter(|&q| k < q)
        .map(&same)
        .collect();
    let kills: Vec<Bool> = kills_between(binds, k, i).map(&same).collect();
    constraint!(same(k), !Bool::or(&later), !Bool::or(&kills))
}

/// Reference `i` resolves to a definition outside a boundary that encloses `i` (a capture).
pub fn captured(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .filter(|&k| binds.boundaries_between(k, i) > 0)
        .map(|k| resolves_to(binds, i, k))
        .collect();
    Bool::or(&cases)
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
