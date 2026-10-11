//! Four runners finished a race in 11 to 14 seconds, all at different times. Who won?
//!
//! - The winning time was 11 seconds.
//! - Alice beat Bob.
//! - Carol didn't win, and Dave didn't come last.
//! - Carol finished exactly one second ahead of Dave.

use z3::Solver;
use z3_toolkit::prelude::*;

fn main() -> Result<(), Failure> {
    const N: usize = 4;
    let [alice, bob, carol, dave]: [usize; N] = [0, 1, 2, 3];
    let names = ["Alice", "Bob", "Carol", "Dave"];
    let times = ints::<N>("time");
    let (winner, last) = (times.argmin(), times.argmax());

    let s = Solver::new();
    constraints! { s;
        for t in &times { t in 11..=14 }
        distinct(&times),
        times.at(&winner) == 11,
        times[alice] < times[bob],
        winner != carol,
        last != dave,
        times[carol] + 1 == times[dave],
    }

    let m = s.solve()?;
    let times: [i64; N] = m.get(&times)?;
    let winner: usize = m.get_as(&winner)?;
    for (name, time) in names.iter().zip(times) {
        println!("{name}: {time}s");
    }
    println!("winner: {}", names[winner]);
    Ok(())
}
