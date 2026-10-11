//! Measure exactly 4L with a 3L and a 5L jug, in as few moves as possible.
//!
//! Each move fills a jug, empties one, or pours one into the other until it is full or empty.

use z3::{
    Optimize,
    ast::{Bool, Int},
};
use z3_toolkit::prelude::*;

// `[small, large]`
type Jugs = [Int; 2];
// Amounts after a move, and the move's label.
type Step = ([i64; 2], &'static str);

struct Puzzle {
    small: i64,
    large: i64,
    target: i64,
}

impl Puzzle {
    // One move from `current` to `next`, labeled by what was done.
    fn step(&self, current: &Jugs, next: &Jugs) -> Bool {
        let [small, large] = current;
        let [small_next, large_next] = next;
        let to_large = min(small, lin!(self.large - large));
        let to_small = min(large, lin!(self.small - small));
        constraint!(any {
            "fill small" @ all { small_next == self.small && large_next == large },
            "fill large" @ all { small_next == small && large_next == self.large },
            "empty small" @ all { small_next == 0 && large_next == large },
            "empty large" @ all { small_next == small && large_next == 0 },
            "small -> large" @ all { small_next == small - to_large && large_next == large + to_large },
            "large -> small" @ all { small_next == small + to_small && large_next == large - to_small },
        })
    }

    fn reached(&self, jugs: &Jugs) -> Bool {
        let [small, large] = jugs;
        constraint!(small == self.target || large == self.target)
    }

    // The states and moves of a shortest solution, or `None` if there's none within `moves_limit`.
    fn shortest(&self, moves_limit: usize) -> Result<Option<Vec<Step>>, Failure> {
        let states: Vec<Jugs> = fresh_n("jug", moves_limit + 1);
        let moves: Vec<Bool> = (0..moves_limit)
            .map(|turn| self.step(&states[turn], &states[turn + 1]))
            .collect();
        let [goal_turn] = ints("goal_turn");
        let [small, large] = &states[0];

        let opt = Optimize::new();
        constraints! { opt;
            small == 0, large == 0,
            goal_turn in 0..=moves_limit,
            for turn in 0..moves_limit {
                { turn < goal_turn } implies { moves[turn] }
            }
            self.reached(&states.at(&goal_turn)),
        }
        opt.minimize(&goal_turn);

        let m = match opt.solve() {
            Ok(m) => m,
            Err(Failure::Unsat { .. }) => return Ok(None),
            Err(e) => return Err(e),
        };
        let goal_turn: usize = m.get_as(&goal_turn)?;
        let mut path = vec![(m.get(&states[0])?, "start")];
        for turn in 0..goal_turn {
            let done = m.labels(&moves[turn])?;
            path.push((m.get(&states[turn + 1])?, done[0]));
        }
        Ok(Some(path))
    }
}

fn main() -> Result<(), Failure> {
    let puzzle = Puzzle {
        small: 3,
        large: 5,
        target: 4,
    };
    let Some(states) = puzzle.shortest(20)? else {
        println!("no solution");
        return Ok(());
    };
    for (turn, ([small, large], done)) in states.iter().enumerate() {
        println!(
            "{turn}: {done:<15} {}L={small}, {}L={large}",
            puzzle.small, puzzle.large
        );
    }
    Ok(())
}
