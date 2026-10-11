use z3::Solver;
use z3_toolkit::prelude::*;

fn main() {
    let n = 8;
    let q = ints_n("q", n);

    let s = Solver::new();
    constraints! { s;
        for c in &q { c in 0..n },
        distinct(q.iter()),
        for i in 0..n {
            for j in (i + 1)..n {
                abs(q[i] - q[j]) != j - i
            }
        },
    }

    let m = s.solve().expect("sat");
    let cols = m.get::<Vec<i64>>(&q).expect("every queen has a column");
    for &c in &cols {
        let line: String = (0..n)
            .map(|x| if x as i64 == c { 'Q' } else { '.' })
            .collect();
        println!("{line}");
    }
}
