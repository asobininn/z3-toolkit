use std::borrow::Borrow;

use z3::{
    Model, Optimize, SatResult, Solver,
    ast::{Ast, Bool},
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

/// Common ops of `Solver` / `Optimize`. Usually driven by [`constraints!`].
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
