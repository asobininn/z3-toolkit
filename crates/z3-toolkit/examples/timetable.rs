//! Fit four meetings into four one-hour slots. The rules conflict,
//! and the unsat core names the ones that can't all hold.

use z3::Solver;
use z3_toolkit::prelude::*;

fn main() -> Result<(), Failure> {
    const N: usize = 4;
    let names = ["standup", "design", "review", "retro"];
    let slots = ints::<N>("slot");
    let [standup, design, review, retro] = &slots;

    let s = Solver::new();
    constraints! { s;
        for slot in &slots { slot in 0..N }
        distinct(&slots),
        "standup first": standup == 0,
        "design in the morning" : design < 2,
        "review after design": review > design,
        "review before lunch": review < 2,
        "retro last": retro == 3,
    }
    match s.solve() {
        Ok(m) => {
            let slots: [usize; N] = m.get_as(&slots)?;
            for (name, slot) in names.iter().zip(slots) {
                println!("{}:00 {name}", 9 + slot);
            }
        }
        Err(Failure::Unsat { core }) => println!("conflicting rules: [{}]", core.join(", ")),
        Err(e) => return Err(e),
    }
    Ok(())
}
