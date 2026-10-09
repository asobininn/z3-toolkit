use z3::{Solver, ast::Int};
use z3_toolkit::prelude::*;

fn main() -> Result<(), Failure> {
    let [x, y] = ints("v");
    let s = Solver::new();
    constraints! { s; x in 0..3, y in 0..3 }
    assert_eq!(s.solve_all::<Int>().over([&x]).count(), 3);

    Ok(())
}
