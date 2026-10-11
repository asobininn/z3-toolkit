use z3::Optimize;
use z3_toolkit::prelude::*;

struct Item<'a> {
    name: &'a str,
    weight: i64,
    value: i64,
}

impl<'a> Item<'a> {
    fn new(name: &'a str, weight: i64, value: i64) -> Self {
        Self {
            name,
            weight,
            value,
        }
    }
}

struct Knapsack<'a> {
    items: Vec<Item<'a>>,
    capacity: i64,
}

impl<'a> Knapsack<'a> {
    fn solve(&self) -> Result<Solution<'_>, Failure> {
        let picks = bools_n("pick", self.items.len());
        let weight = dot(self.items.iter().map(|i| i.weight), &picks);
        let value = dot(self.items.iter().map(|i| i.value), &picks);

        let opt = Optimize::new();
        constraints! { opt; weight <= self.capacity };
        opt.maximize(&value);

        let m = opt.solve()?;
        let taken: Vec<bool> = m.get(&picks)?;
        Ok(Solution {
            chosen: self
                .items
                .iter()
                .zip(taken)
                .filter(|(_, t)| *t)
                .map(|(i, _)| i)
                .collect(),
            total_value: m.get(&value)?,
            total_weight: m.get(&weight)?,
        })
    }
}

struct Solution<'a> {
    chosen: Vec<&'a Item<'a>>,
    total_value: i64,
    total_weight: i64,
}

fn main() {
    let problem = Knapsack {
        items: vec![
            Item::new("A", 12, 4),
            Item::new("B", 2, 2),
            Item::new("C", 1, 1),
            Item::new("D", 1, 2),
            Item::new("E", 4, 10),
        ],
        capacity: 15,
    };
    match problem.solve() {
        Ok(s) => {
            let names: Vec<&str> = s.chosen.iter().map(|i| i.name).collect();
            println!(
                "chosen: {names:?} / value: {}, weight: {}",
                s.total_value, s.total_weight
            );
        }
        Err(e) => println!("failure: {e}"),
    }
}
