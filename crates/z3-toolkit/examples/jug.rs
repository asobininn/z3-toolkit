use z3::{
    Optimize,
    ast::{Bool, Int},
};
use z3_toolkit::prelude::*;

// `[small, large]`
type State = [i64; 2];
type Jugs = <State as Z3Type>::Repr;

struct Puzzle {
    small: i64,
    large: i64,
    target: i64,
}

impl Puzzle {
    fn step(&self, current: &Jugs, next: &Jugs) -> Bool {
        let [small, large] = current;
        let [small_next, large_next] = next;
        let to_large = min(small, lin!(self.large - large)); // 3L→5Lに移せる量
        let to_small = min(large, lin!(self.small - small)); // 5L→3Lに移せる量
        // 操作群
        constraint!(any {
            small_next == self.small && large_next == large, // 3Lを満タンに
            small_next == small && large_next == self.large, // 5Lを満タンに
            small_next == 0 && large_next == large, // 3Lを空に
            small_next == small && large_next == 0, // 5Lを空に
            small_next == small - to_large && large_next == large + to_large, // 3L→5L
            small_next == small + to_small && large_next == large - to_small, // 5L→3L
        })
    }

    fn reached(&self, jugs: &Jugs) -> Bool {
        let [small, large] = jugs;
        constraint!(small == self.target || large == self.target)
    }

    fn shortest(&self, moves_limit: usize) -> Result<Option<Vec<State>>, Failure> {
        let states: Vec<Jugs> = fresh_n("jug", moves_limit + 1);
        let goal_turn = Int::new_const("goal_turn");
        let [small, large] = &states[0];

        let opt = Optimize::new();
        constraints! { opt;
            small == 0, large == 0,
            goal_turn in 0..=moves_limit,
            for turn in 0..moves_limit {
                { turn < goal_turn } implies { self.step(&states[turn], &states[turn + 1]) }
            }
            // reached(states[goal_turn])がゴール状態である
            any { for turn in 0..=moves_limit {
                goal_turn == turn && self.reached(&states[turn])
            }}
        }
        opt.minimize(&goal_turn);

        let m = match opt.solve() {
            Ok(m) => m,
            Err(Failure::Unsat { .. }) => return Ok(None),
            Err(e) => return Err(e),
        };
        let goal_turn = m.get::<i64>(&goal_turn).unwrap() as usize;
        Ok(m.get(&states[..=goal_turn].to_vec()))
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
    for (turn, [small, large]) in states.iter().enumerate() {
        println!(
            "{turn}: {}L={small}, {}L={large}",
            puzzle.small, puzzle.large
        );
    }
    Ok(())
}
