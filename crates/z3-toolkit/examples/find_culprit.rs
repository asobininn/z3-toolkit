//! Exactly one of four suspects is the culprit. They say:
//!
//! - Alice: "Bob isn't the culprit."
//! - Bob:   "Carol is the culprit."
//! - Carol: "Dave is lying."
//! - Dave:  "Exactly one of Alice and Bob is the culprit."
//!
//! The culprit always lies, and exactly two of the other three tell the truth.

use z3::Solver;
use z3_toolkit::prelude::*;

fn main() -> Result<(), Failure> {
    const N: usize = 4;
    let names = ["Alice", "Bob", "Carol", "Dave"];
    let guilty = bools::<N>("guilty");
    let [alice, bob, carol, _] = &guilty;

    let dave_says = constraint!(exactly(1) { alice, bob });
    let says = [
        constraint!(!bob),
        constraint!(carol),
        constraint!(!dave_says),
        dave_says,
    ];

    let s = Solver::new();
    constraints! { s;
        exactly(1) { for g in &guilty { g } }
        exactly(2) { for claim in &says { claim } }
        for i in 0..N { { guilty[i] } implies { !says[i] }  }
    }

    let model = s.solve()?;
    let guilty: [bool; N] = model.get(&guilty)?;
    let says: [bool; N] = model.get(&says)?;
    for i in 0..N {
        let role = if guilty[i] { "culprit" } else { "innocent" };
        let claim = if says[i] { "true" } else { "false" };
        println!("{}: {role}, says {claim}", names[i]);
    }
    Ok(())
}
