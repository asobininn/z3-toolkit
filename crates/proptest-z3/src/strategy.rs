use std::{fmt::Debug, sync::Arc};

use proptest::{prelude::*, strategy::ValueTree};
use z3::ast::Bool;

use crate::{Binds, Fill, NameSpec, Names, Skeleton, Solved};

pub trait FillExt: Strategy<Value = Skeleton> + Sized {
    fn fill<R>(self, names: NameSpec, rule: R) -> Filled<Self, R>
    where
        R: Fn(&Binds) -> Bool + 'static,
    {
        self.fill_with(Fill::Optimal, names, rule)
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

    fn new_tree(
        &self,
        runner: &mut prop::test_runner::TestRunner,
    ) -> prop::strategy::NewTree<Self> {
        loop {
            let inner = self.inner.new_tree(runner)?;
            let (skeleton, names) = inner.current();
            match skeleton.solve(&names, Fill::Optimal, &*self.rule) {
                Ok(solved) => {
                    return Ok(FilledTree {
                        inner,
                        rule: self.rule.clone(),
                        current: solved,
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
    rule: Arc<R>,
    current: Solved,
}

impl<T, R> FilledTree<T, R>
where
    T: ValueTree<Value = (Skeleton, Names)>,
    R: Fn(&Binds) -> Bool,
{
    fn solve(&mut self) -> Option<Solved> {
        let (skeleton, names) = self.inner.current();
        skeleton
            .solve(&names, Fill::Optimal, self.rule.as_ref())
            .ok()
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
        while self.inner.simplify() {
            if let Some(solved) = self.solve() {
                self.current = solved;
                return true;
            }
            if !self.inner.complicate() {
                break;
            }
        }
        // current は最後に失敗した値のまま
        false
    }

    fn complicate(&mut self) -> bool {
        while self.inner.complicate() {
            if let Some(solved) = self.solve() {
                self.current = solved;
                return true;
            }
        }
        false
    }
}
