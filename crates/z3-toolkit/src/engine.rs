use std::{borrow::Borrow, iter::FusedIterator};

use z3::{
    Model, Optimize, SatResult, Solver,
    ast::{Ast, Bool, Dynamic},
};

const LABEL_PREFIX: &str = "z3-toolkit:";

/// Why [`Engine::solve`] returned no model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// `core`: labels of the conflicting labeled constraint. Unlabeled ones not appear.
    Unsat { core: Vec<String> },
    /// Giveup  (timeout, incompletness, ..), or `Sat` without a model.
    Unknown { reason: String },
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Unsat { core } if core.is_empty() => write!(f, "unsatisfiable"),
            Failure::Unsat { core } => write!(f, "unsatisfiable (core: {})", core.join(", ")),
            Failure::Unknown { reason } => write!(f, "unknown ({reason})"),
        }
    }
}

impl std::error::Error for Failure {}

/// Common ops of `Solver` / `Optimize`. Usually driven by [`constraints!`](crate::constraints).
pub trait Engine {
    fn assert_one(&self, constraint: &Bool);
    fn assert_tracked(&self, constraint: &Bool, tracker: &Bool);
    fn check_sat(&self) -> SatResult;
    fn model(&self) -> Option<Model>;
    fn unsat_core(&self) -> Vec<Bool>;
    fn reason_unknown(&self) -> Option<String>;

    fn assert_all<I>(&self, constraints: I)
    where
        I: IntoIterator,
        I::Item: Borrow<Bool>,
    {
        for c in constraints {
            self.assert_one(c.borrow());
        }
    }

    /// Tracked so that `label` shows up in [`Failure::Unsat`]'s core.
    fn assert_labeled(&self, label: &str, constraint: &Bool) {
        let tracker = Bool::new_const(format!("{LABEL_PREFIX}{label}"));
        self.assert_tracked(constraint, &tracker);
    }

    /// `check_sat` + model as a `Result`.
    ///
    /// ## Examples
    /// ```
    /// # use z3::Solver; use z3_toolkit::prelude::*;
    /// let [x] = ints("x");
    /// let s = Solver::new();
    /// constraints! { s; "low": x > 5, "high": x < 3 };
    /// let Err(Failure::Unsat { core }) = s.solve() else { panic!() };
    /// # assert_eq!(core.len(), 2);
    /// ```
    fn solve(&self) -> Result<Model, Failure> {
        match self.check_sat() {
            SatResult::Sat => self.model().ok_or_else(|| Failure::Unknown {
                reason: "no model available".to_owned(),
            }),
            SatResult::Unsat => Err(Failure::Unsat {
                core: self
                    .unsat_core()
                    .iter()
                    .filter_map(|b| {
                        b.decl()
                            .name()
                            .strip_prefix(LABEL_PREFIX)
                            .map(str::to_owned)
                    })
                    .collect(),
            }),
            SatResult::Unknown => Err(Failure::Unknown {
                reason: self.reason_unknown().unwrap_or_default(),
            }),
        }
    }

    /// Every solution, as an iterator of models.
    ///
    /// ## Examples
    /// ```
    /// # use z3::{ Model, Solver, ast::Int }; use z3_toolkit::prelude::*;
    /// let [x, y] = ints("v");
    /// let s = Solver::new();
    /// constraints! { s; x in 0..3, y in 0..3, x < y };
    /// let res: Vec<Result<Model, Failure>> = s.solve_all().collect();
    /// assert_eq!(res.into_iter().flatten().count(), 3);
    /// ```
    fn solve_all(&self) -> Solutions<'_, Self> {
        Solutions {
            engine: self,
            vars: None,
            done: false,
        }
    }
}

impl Engine for Solver {
    fn assert_one(&self, constraint: &Bool) {
        self.assert(constraint);
    }
    fn assert_tracked(&self, constraint: &Bool, tracker: &Bool) {
        self.assert_and_track(constraint, tracker);
    }
    fn check_sat(&self) -> SatResult {
        self.check()
    }
    fn model(&self) -> Option<Model> {
        self.get_model()
    }
    fn unsat_core(&self) -> Vec<Bool> {
        self.get_unsat_core()
    }
    fn reason_unknown(&self) -> Option<String> {
        self.get_reason_unknown()
    }
}

impl Engine for Optimize {
    fn assert_one(&self, constraint: &Bool) {
        self.assert(constraint);
    }
    fn assert_tracked(&self, constraint: &Bool, tracker: &Bool) {
        self.assert_and_track(constraint, tracker);
    }
    fn check_sat(&self) -> SatResult {
        self.check(&[])
    }
    fn model(&self) -> Option<Model> {
        self.get_model()
    }
    fn unsat_core(&self) -> Vec<Bool> {
        self.get_unsat_core()
    }
    fn reason_unknown(&self) -> Option<String> {
        self.get_reason_unknown()
    }
}

/// Iterator returned by [`Engine::solve_all`]
pub struct Solutions<'a, E: ?Sized> {
    engine: &'a E,
    vars: Option<Vec<Dynamic>>,
    done: bool,
}

impl<E: ?Sized> Solutions<'_, E> {
    /// Compare only `vars` when blocking, so other constants don't make new solutions.
    ///
    /// ## Examples
    /// ```
    /// # use z3::{Solver, ast::Int}; use z3_toolkit::prelude::*;
    /// let [x, y] = ints("v");
    /// let s = Solver::new();
    /// constraints! { s; x in 0..3, y in 0..3 }
    /// assert_eq!(s.solve_all().over([&x]).count(), 3);
    /// ```
    pub fn over<'a, A: Ast + 'a>(mut self, vars: impl IntoIterator<Item = &'a A>) -> Self {
        self.vars = Some(vars.into_iter().map(|v| Dynamic::from_ast(v)).collect());
        self
    }
}

fn differs_all(model: &Model) -> Bool {
    let ne: Vec<Bool> = model
        .iter()
        .filter(|d| d.arity() == 0 && !d.name().starts_with(LABEL_PREFIX))
        .map(|d| {
            let c = d.apply(&[]);
            c.ne(model
                .eval(&c, true)
                .expect("model can evalute its own constants"))
        })
        .collect();
    Bool::or(&ne)
}

impl<E: Engine + ?Sized> Iterator for Solutions<'_, E> {
    type Item = Result<Model, Failure>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.engine.solve() {
            Ok(m) => {
                let block = match &self.vars {
                    Some(vars) => crate::differs(&m, vars),
                    None => differs_all(&m),
                };
                self.engine.assert_one(&block);
                Some(Ok(m))
            }
            Err(Failure::Unsat { .. }) => {
                self.done = true;
                None
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

impl<E: Engine + ?Sized> FusedIterator for Solutions<'_, E> {}
