//! Order seven project tasks so that every dependency comes first, and list every valid order.
//!
//! ```text
//! spec ─┬─ design ── code ─────┬─ test ── release
//!       └─ parts ─── assembly ─┘
//! ```
//!

use z3::Solver;
use z3_toolkit::prelude::*;

fn main() -> Result<(), Failure> {
    const N: usize = 7;
    let names = [
        "spec", "design", "parts", "code", "assembly", "test", "release",
    ];
    let steps = ints::<N>("step");
    let [spec, design, parts, code, assembly, test, release] = &steps;

    let s = Solver::new();
    constraints! { s;
        for step in &steps { step in 0..N }
        distinct(&steps),
        spec < design, spec < parts,
        design < code, parts < assembly,
        code < test, assembly < test,
        test < release
    };

    for model in s.solve_all() {
        let found: [i64; N] = model?.get(&steps).unwrap();
        let mut order = [""; N];
        for i in 0..N {
            order[found[i] as usize] = names[i];
        }
        println!("{}", order.join(" → "));
    }
    Ok(())
}
