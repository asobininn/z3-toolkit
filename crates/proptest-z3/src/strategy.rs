use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

use proptest::{prelude::*, strategy::ValueTree};
use z3::ast::Bool;

use crate::{
    Binds, Fill, NameSpec, Names, Skeleton, Solved,
    label::{count_case, count_rejection},
};

/// An extension trait that adds hole filling to strategies of skeletons.
///
/// Implemented for every type implementing `Strategy<Value = Skeleton>`.
pub trait FillExt: Strategy<Value = Skeleton> + Sized {
    /// Combines a strategy of skeletons, name candidates and a rule into a strategy of filled
    /// inputs ([`Solved`]), solved with [`Fill::Greedy`].
    ///
    /// `rule` takes [`Binds`] and returns, as a Z3 `Bool`, the relations to enforce. It must be
    /// `'static`, so use a `move` closure to capture outside values. The rule is called every time
    /// a skeleton is solved, including each shrinking step.
    ///
    /// # Generation
    ///
    /// Generates a skeleton and candidates, and regenerates them if the rule cannot be satisfied
    /// (unsat). Each regeneration counts as one local reject in proptest, and the test aborts once
    /// the limit (`max_local_rejects`) is exceeded. Use [`take_rejections`](crate::take_rejections)
    /// to find out why.
    ///
    /// # Shrinking
    ///
    /// Shrinks the structure, hints and name candidates, trying only inputs that satisfy the rule.
    ///
    /// - If a shrunk input cannot satisfy the rule, the failure is considered not reproducible in
    ///   that direction, and another way to shrink is tried
    /// - If the solved result (text, marks and labels) was already tried, the test is not called again
    /// - If the shrunk input is the same as before, it is not solved again
    /// - Shrinking does not run to a fixed point; unrelated parts may remain in the counterexample
    ///
    /// Z3 is called whenever the input changes, so shrinking takes longer than with plain proptest.
    ///
    /// # Examples
    ///
    /// ```
    /// # use proptest::prelude::*; use proptest_z3::prelude::*;
    /// const VAR: Space = Space::new("var");
    ///
    /// fn program() -> impl Strategy<Value = Skeleton> {
    ///     prop::collection::vec(VAR.refer().prop_map(|y| frag!["print ", y, "\n"]), 1..8)
    ///         .prop_map(Skeleton::steps)
    /// }
    ///
    /// let inputs = program().fill(NameSpec::new().space(VAR, "[a-c]", 3), |binds| bound(binds, VAR));
    /// ```
    fn fill<R>(self, names: NameSpec, rule: R) -> Filled<Self, R>
    where
        R: Fn(&Binds) -> Bool + 'static,
    {
        self.fill_with(Fill::default(), names, rule)
    }

    /// Same as [`fill`](FillExt::fill), but with a choice of solving strategy [`Fill`].
    fn fill_with<R>(self, fill: Fill, names: NameSpec, rule: R) -> Filled<Self, R>
    where
        R: Fn(&Binds) -> Bool + 'static,
    {
        Filled {
            inner: (self, names),
            fill,
            rule: Arc::new(rule),
        }
    }
}

impl<S: Strategy<Value = Skeleton>> FillExt for S {}

/// The strategy returned by [`fill`](FillExt::fill) / [`fill_with`](FillExt::fill_with). Its value is [`Solved`].
pub struct Filled<S, R> {
    inner: (S, NameSpec),
    fill: Fill,
    rule: Arc<R>,
}

impl<S: Debug, R> Debug for Filled<S, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filled")
            .field("inner", &self.inner)
            .field("fill", &self.fill)
            .finish()
    }
}

impl<S, R> Strategy for Filled<S, R>
where
    S: Strategy<Value = Skeleton>,
    R: Fn(&Binds) -> Bool + 'static,
{
    type Tree = FilledTree<<(S, NameSpec) as Strategy>::Tree, R>;
    type Value = Solved;

    /// Regenerates until a solvable skeleton comes out. Each regeneration counts as a local reject,
    /// and the test aborts once the limit (`max_local_rejects`) is exceeded.
    fn new_tree(
        &self,
        runner: &mut prop::test_runner::TestRunner,
    ) -> prop::strategy::NewTree<Self> {
        loop {
            let inner = self.inner.new_tree(runner)?;
            let (skeleton, names) = inner.current();
            match skeleton.solve(&names, self.fill, &*self.rule) {
                Ok(solved) => {
                    count_case(&solved.declared);
                    return Ok(FilledTree {
                        inner,
                        fill: self.fill,
                        rule: self.rule.clone(),
                        current: solved.clone(),
                        failing: solved,
                        failed: HashSet::new(),
                        passed: HashSet::new(),
                        solved: HashMap::new(),
                    });
                }
                Err(failure) => {
                    count_rejection(&failure);
                    runner.reject_local("unsat (see proptest_z3::take_rejections)")?;
                }
            }
        }
    }
}

/// The `ValueTree` of [`Filled`], which handles shrinking. Not used directly.
pub struct FilledTree<T, R> {
    inner: T,
    fill: Fill,
    rule: Arc<R>,
    current: Solved,
    failing: Solved,
    failed: HashSet<String>,
    passed: HashSet<String>,
    solved: HashMap<String, Option<Solved>>,
}

impl<T, R> FilledTree<T, R>
where
    T: ValueTree<Value = (Skeleton, Names)>,
    R: Fn(&Binds) -> Bool,
{
    fn solve(&mut self) -> Option<Solved> {
        let (skeleton, names) = self.inner.current();
        let (fill, rule) = (self.fill, &*self.rule);
        (self
            .solved
            .entry(input_key(&skeleton, &names))
            .or_insert_with(|| skeleton.solve(&names, fill, rule).ok()))
        .clone()
    }

    fn walk(&mut self, mut dir: Move) -> bool {
        for _ in 0..MAX_MOVES_PER_STEP {
            let moved = match dir {
                Move::Simplify => self.inner.simplify(),
                Move::Complicate => self.inner.complicate(),
            };
            if !moved {
                break;
            }
            let Some(solved) = self.solve() else {
                dir = Move::Complicate;
                continue;
            };
            let key = fingerprint(&solved);
            if self.failed.contains(&key) {
                dir = Move::Simplify;
            } else if self.passed.contains(&key) {
                dir = Move::Complicate;
            } else {
                self.current = solved;
                return true;
            }
        }
        self.current = self.failing.clone();
        false
    }
}

impl<T, R> ValueTree for FilledTree<T, R>
where
    T: ValueTree<Value = (Skeleton, Names)>,
    R: Fn(&Binds) -> Bool,
{
    type Value = Solved;

    fn current(&self) -> Self::Value {
        self.current.clone()
    }

    fn simplify(&mut self) -> bool {
        self.failed.insert(fingerprint(&self.current));
        self.failing = self.current.clone();
        self.walk(Move::Simplify)
    }

    fn complicate(&mut self) -> bool {
        self.passed.insert(fingerprint(&self.current));
        self.walk(Move::Complicate)
    }
}

const MAX_MOVES_PER_STEP: usize = 256;

#[derive(Clone, Copy)]
enum Move {
    Simplify,
    Complicate,
}

fn fingerprint(solved: &Solved) -> String {
    format!("{solved:?}")
}

fn input_key(skeleton: &Skeleton, names: &Names) -> String {
    let frag = &skeleton.0;
    let holes: Vec<_> = (frag.holes.iter())
        .map(|h| {
            let n = names.get(h.space).len();
            (
                h.space,
                h.role,
                h.scope,
                h.step,
                (n > 0).then(|| h.hint.index(n)),
            )
        })
        .collect();
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}",
        frag.pieces, holes, frag.order, frag.scope_parent, names
    )
}
