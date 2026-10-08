use z3::{Solver, ast::Int};
use z3_toolkit::prelude::*;

fn main() {
    let n = 8;
    let s = Solver::new();
    let q: Vec<Int> = (0..n).map(|_| Int::fresh_const("q")).collect();

    constraints! { s;
        for col in &q { col >= 0, col < n },
        distinct(q.iter()),
        for i in 0..n {
            for j in (i + 1)..n {
                // |q[i] - q[j]| == |i - j|
                q[i] - q[j] != (j - i) as i64,
                q[j] - q[i] != (j - i) as i64,
            }
        }
    };
    let m = s.solve().expect("sat");
    let cols = m.get::<Vec<i64>>(&q).expect("every quen has as column");
    for &c in &cols {
        let line: String = (0..n as i64)
            .map(|x| if x == c { 'Q' } else { '.' })
            .collect();
        println!("{line}");
    }
}
