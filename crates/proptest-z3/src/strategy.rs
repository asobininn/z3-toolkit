use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

use proptest::{prelude::*, strategy::ValueTree};
use z3::ast::Bool;

use crate::{Binds, Fill, NameSpec, Names, Skeleton, Solved};

pub trait FillExt: Strategy<Value = Skeleton> + Sized {
    fn fill<R>(self, names: NameSpec, rule: R) -> Filled<Self, R>
    where
        R: Fn(&Binds) -> Bool + 'static,
    {
        self.fill_with(Fill::default(), names, rule)
    }

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

/// The strategy returned by `fill(..)`.
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
                Err(_failure) => {
                    runner.reject_local("unsat")?;
                }
            }
        }
    }
}

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
