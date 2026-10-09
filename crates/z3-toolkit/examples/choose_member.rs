//! Pick 3–4 members with the highest total skill, where
//!
//! - total cost is at most 100,
//! - Chloe joins only if Daniel does,
//! - Alex and Ethan never work together,
//! - and at least one member writes Rust.

use z3::{Optimize, ast::Bool};
use z3_toolkit::prelude::*;

struct Member<'a> {
    name: &'a str,
    cost: i64,
    skill: i64,
    is_rustacean: bool,
    pick: Bool,
}

impl<'a> Member<'a> {
    fn new(name: &'a str, cost: i64, skill: i64, is_rustacean: bool) -> Self {
        let pick = fresh(name);
        Self {
            name,
            cost,
            skill,
            is_rustacean,
            pick,
        }
    }
}

fn main() -> Result<(), Failure> {
    let team = [
        Member::new("Alex", 35, 8, false),
        Member::new("Bob", 20, 2, false),
        Member::new("Chloe", 45, 6, true),
        Member::new("Daniel", 40, 2, false),
        Member::new("Ethan", 35, 6, true),
        Member::new("Fiona", 50, 6, false),
    ];
    let [alex, _, chloe, daniel, ethan, _] = &team;
    let budget = lin!(sum(&team, |m| m.cost * m.pick));
    let power = lin!(sum(&team, |m| m.skill * m.pick));
    let has_rustacean = constraint!(any { for m in &team { if m.is_rustacean { m.pick } } });

    let opt = Optimize::new();
    constraints! { opt;
        budget <= 100,
        sum(&team, |m| m.pick) in 3..=4,
        { chloe.pick } implies { daniel.pick },
        !(alex.pick && ethan.pick),
        has_rustacean,
    }
    opt.maximize(&power);

    let model = opt.solve()?;
    for member in team.iter().filter(|m| model.get(&m.pick) == Some(true)) {
        println!("{}", member.name);
    }
    Ok(())
}
