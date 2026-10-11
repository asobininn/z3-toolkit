use z3::{Model, Solver};
use z3_toolkit::prelude::*;

fn main() {
    let [x, y] = ints("v");
    let s = Solver::new();
    constraints! { s; x in 0..3, y in 0..3, x < y };
    let res: Vec<Result<Model, Failure>> = s.solve_all().collect();
    assert_eq!(res.into_iter().flatten().count(), 3);
}
