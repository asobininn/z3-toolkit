//! Predicates for writing name-resolution rules.
//!
//! Every function takes [`Binds`] and returns a Z3 `Bool` (or a sequence of hole positions used as
//! material for constraints). The arguments `i` and `k` are positions in evaluation order. Combine
//! them inside `constraint!` to write the rule passed to [`fill`](crate::FillExt::fill).
//!
//! # Evaluation order
//!
//! Holes are handled in evaluation order, not in text order.
//!
//! 1. Steps are evaluated in the order they appear
//! 2. Within one step,
//!    - first: the `Use` holes and the holes inside scopes nested in the step (in text order)
//!    - then: the step's own `Def` and `Kill` holes (in text order)
//!
//! So the right-hand side of `let x = x` does not point to the left-hand side of the same
//! statement. Likewise, if `fn f { call f }` is a single statement, `f` is not visible from its
//! body (for recursion, put a step with just the name before the body).
//!
//! # Scopes and boundaries
//!
//! - A name defined inside [`Frag::scope`](crate::Frag::scope) is visible only from that scope and
//!   the scopes inside it. Outer names are visible inside, and defining the same name inside shadows them
//! - A [`Frag::boundary`](crate::Frag::boundary) (closure or function body) behaves exactly like a
//!   plain scope for visibility. The only difference is that [`Binds::boundaries_between`] counts
//!   the boundaries crossed
//! - Scopes are not passed to Z3. They are Rust values fixed at generation time and are used to
//!   narrow down the candidates
//!
//! # Kills cross scopes
//!
//! If there is any `Kill` of the same name in the same namespace between a definition `k` and a
//! reference `i` (in evaluation order), `k` is not visible from `i`, whatever scope or boundary
//! the `Kill` is in. Killing a different definition of the same name in an inner scope also hides
//! the outer definition.
//!
//! # Which functions cross scopes
//!
//! | Function | Treatment |
//! | --- | --- |
//! | [`Binds::defs`] / [`uses`](Binds::uses) / [`kills`](Binds::kills) / [`refs`](Binds::refs) / [`of`](Binds::of) | ignore scopes (whole skeleton) |
//! | [`Binds::same_step`], [`defined_in_same_step`] | only within one step (never cross) |
//! | [`Binds::encloses`], [`reaching_defs`] | lexical (outer definitions are visible inside) |
//! | [`Binds::boundaries_between`], [`captured`] | lexical, counting only boundaries |
//! | [`kills_between`] | ignore both scopes and boundaries |
//! | [`visible`], [`resolves_to`], [`killed`], [`bound`], [`unbound`] | definitions lexical, kills ignore scopes |
//! | [`distinct_defs`] | ignores scopes (shadowing and equal names in sibling scopes count too) |
//!
//! # The three states of a reference
//!
//! A reference is in exactly one of these states. `!visible` is the union of "killed" and "undefined".
//!
//! | State | Expression |
//! | --- | --- |
//! | visible | `visible(binds, i)` |
//! | killed | `killed(binds, i)` |
//! | undefined | `(!visible(binds, i)).given([!killed(binds, i)])` |
//!
//! "Undefined" includes the case where the only definitions are out of sight (in a closed or sibling scope).

use crate::{Binds, Role, Space};
use z3::ast::Bool;
use z3_toolkit::prelude::*;

/// A candidate is a `Def` in the namespace of `i` that comes before `i` in evaluation order and
/// whose scope encloses `i` ([`Binds::encloses`]). Boundaries are seen through. Names and kills are
/// not considered yet (they are decided by the constraints).
///
/// [`visible`] and [`resolves_to`] are written with this and [`kills_between`]. Use it as material
/// for your own constraints when the provided predicates are not enough.
pub fn reaching_defs(binds: &Binds, i: usize) -> impl Iterator<Item = usize> + '_ {
    let space = binds.space[i];
    (0..i).filter(move |&k| {
        binds.role[k] == Role::Def && binds.space[k] == space && binds.encloses(k, i)
    })
}

/// Returns the `Kill` holes in the namespace of `i` that lie strictly between `k` and `i` in
/// evaluation order.
///
/// **Scopes and boundaries do not matter.** Names are not considered yet.
pub fn kills_between(binds: &Binds, k: usize, i: usize) -> impl Iterator<Item = usize> + '_ {
    let space = binds.space[i];
    (k + 1..i).filter(move |&j| binds.role[j] == Role::Kill && binds.space[j] == space)
}

/// Reference `i` points to a visible definition that has not been killed.
///
/// That is, for some `k` in [`reaching_defs`], `k` and `i` have the same name and no `Kill` of that
/// name lies between them ([`kills_between`]). False if there are no candidates. Kills take effect
/// across scopes.
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

/// Reference `i` points to definition `k`.
///
/// That is, `k` is in [`reaching_defs`], the names are equal, no later candidate has the same name,
/// and no `Kill` of that name lies in between. In other words, `i` points to the last live
/// definition of the name in evaluation order (the inner one under shadowing, the last one under
/// redefinition in the same scope). If `k` is not a candidate, a constant false is returned.
///
/// This is the same rule as [`Solved::target`](crate::Solved::target) after solving.
/// `visible(binds, i)` is equivalent to `resolves_to(binds, i, k)` holding for some candidate `k`.
/// Use it when a condition depends on which definition is pointed to, such as unused definitions
/// or shadowing.
///
/// # Examples
///
/// ```
/// # use proptest_z3::prelude::*;
/// const VAR: Space = Space::new("var");
///
/// #[derive(Debug, Clone, Copy, PartialEq)]
/// struct Unused;
///
/// // Mark definitions that no reference points to (not a constraint)
/// let rule = |binds: &Binds| {
///     for k in binds.defs(VAR) {
///         let used = constraint!(any { for i in binds.refs(VAR) { resolves_to(binds, i, k) } });
///         binds.mark(k, Unused, !used);
///     }
///     bound(binds, VAR)
/// };
/// ```
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

/// Reference `i` points to a definition outside a boundary that encloses `i` (a capture).
///
/// That is, [`resolves_to`]`(binds, i, k)` holds for some candidate `k` with
/// [`Binds::boundaries_between`]`(k, i) > 0`.
///
/// - Crossing only plain scopes ([`Frag::scope`](crate::Frag::scope)) is not a capture
/// - References to top-level definitions are counted as captures too. For languages that do not
///   call global variables captures, exclude them when building the expected values
/// - To make the condition depend on the number of boundaries crossed, filter `reaching_defs` with
///   `boundaries_between` and combine `resolves_to`
pub fn captured(binds: &Binds, i: usize) -> Bool {
    let cases: Vec<Bool> = reaching_defs(binds, i)
        .filter(|&k| binds.boundaries_between(k, i) > 0)
        .map(|k| resolves_to(binds, i, k))
        .collect();
    Bool::or(&cases)
}

/// Every reference in `space` points to a visible name ([`visible`]). The basic rule for positive cases.
///
/// References include `Kill`s, so kills are applied to live names.
pub fn bound(binds: &Binds, space: Space) -> Bool {
    constraint!(all { for i in binds.refs(space) { visible(binds, i) } })
}

/// Exactly `n` references in `space` are not visible (`!visible`).
///
/// `!visible` includes "killed". To tell the kinds apart, write them separately with [`killed`]
/// and [`Given::given`](crate::Given::given).
pub fn unbound(binds: &Binds, space: Space, n: usize) -> Bool {
    constraint!(exactly(n) { for i in binds.refs(space) { !visible(binds, i) } })
}

/// There is a definition with the same name in the same namespace in the same step as hole `i`.
///
/// The order of evaluation does not matter. Holes in the same step are always in the same scope,
/// so this never crosses scopes. Use it to build self references such as `let x = x`.
pub fn defined_in_same_step(binds: &Binds, i: usize) -> Bool {
    let same: Vec<Bool> = binds
        .defs(binds.space[i])
        .filter(|&k| k != i && binds.same_step(k, i))
        .map(|k| constraint!(binds.name[k] == binds.name[i]))
        .collect();
    Bool::or(&same)
}

/// Reference `i` points to a killed name.
///
/// Kills take effect across scopes.
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

/// The names of the definitions in `space` are pairwise distinct. True with zero or one definition.
///
/// **Counted across scopes.** A definition of the same name in an inner scope (shadowing) and equal
/// names in sibling scopes count as duplicates.
///
/// - `!distinct_defs(binds, space)` means "there is at least one pair of definitions with the same
///   name somewhere", not necessarily a redefinition in the same scope
/// - To deal only with redefinitions in the same scope, narrow down the pairs with
///   [`Binds::same_step`] or [`Binds::encloses`] and write the constraint with `constraint!`
/// - Cannot be satisfied when there are more definitions than candidates
pub fn distinct_defs(binds: &Binds, space: Space) -> Bool {
    binds
        .name
        .distinct_among(|k| binds.role[k] == Role::Def && binds.space[k] == space)
}
