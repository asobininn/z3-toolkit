//! A safe has a 3-digit code with no repeated digits, and the code is a multiple of 3.
//! Exactly one of three witnesses is honest:
//!
//! - Alice: "The hundreds digit is even."
//! - Bob:   "The tens digit is greater than 5."
//! - Carol: "The ones digit is 7."
//!
//! How many codes are still possible?

use z3::{Solver, ast::Int};
use z3_toolkit::prelude::*;

fn main() {
    let digits = ints::<3>("digits");
    let [hundreds, tens, ones] = &digits;
    let code = dot([100, 10, 1], &digits);

    let alice = constraint!(hundreds % 2 == 0);
    let bob = constraint!(tens > 5);
    let carol = constraint!(ones == 7);

    let s = Solver::new();
    constraints! { s;
        hundreds in 1..=9, tens in 0..=9, ones in 0..= 9,
        distinct(&digits),
        code % 3 == 0,
        exactly(1) { alice, bob, carol },
    }

    let mut codes: Vec<_> = s
        .solve_all::<Int>()
        .flatten()
        .map(|m| m.get::<i64>(&code).unwrap())
        .collect();
    codes.sort();
    println!("{codes:?}");
}
