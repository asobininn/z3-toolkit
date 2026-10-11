# z3-toolkit

An ergonomic constraint DSL for the Rust [`z3`](https://crates.io/crates/z3) crate.

It keeps z3's own `Int`, `Bool`, `Solver` and `Optimize`, and adds one-line variable declarations, linear helpers, solution enumeration, and typed readback around them.

## Setup

```shell
cargo add z3-toolkit
```

```toml
[dependencies]
z3 = "0.21"
z3-toolkit = "0.1"
```

## Usage

```rust
use z3_toolkit::prelude::*;

let [x, y] = ints("v");
let s = z3::Solver::new();
constraints! { s;
    x + y == 10,
    y - 2 == 2
}
s.solve()?;
```

## Examples

```rust
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
        Self { name, cost, skill, is_rustacean, pick, }
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
```

| Example | Description |
| --- | --- |
| [choose_member](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples/choose_member.rs) | Pick the most skilled team within a budget, with rules on who works with whom. |
| [crack_safe](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples/crack_safe.rs) | Count every 3-digit code that fits clues from three witnesses, only one honest. |
| [order_tasks](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples/order_tasks.rs) | List every order of project tasks that respects their dependencies. |
| [find_culprit](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples/find_culprit.rs) | Find the culprit among four suspects from their statements. |
| [n_queen](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples/n_queen.rs) | Place 8 queens so that none attack each other. |
| [knapsack](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples/knapsack.rs) | Pack the most valuable items under a weight limit. |

Run one with `cargo run -p z3-toolkit --example <name>`.

## Features

| | |
| --- | --- |
| Variables | `ints`, `bools`, `fresh` (arrays, nested arrays, tuples), `ints_n` / `bools_n` for runtime lengths |
| Constraints | `constraints!` asserts into a `Solver` / `Optimize`; `constraint!` builds one `Bool` |
| DSL | comparisons and `&&` / `\|\|` / `!`, `x in 1..=3`, `{ p } implies { q }`, `all` / `any` / `not`, `exactly` / `at_most` / `at_least`, `distinct`, `for` / `if`, `"label": item` |
| Linear terms | `sum`, `dot`, `lin!`, `abs`, `max`, `min`; `Bool` counts as 0/1, Rust integers are lifted |
| Solving | `solve()` returns `Result<Model, Failure>`; labeled constraints appear in the unsat core |
| Enumerating | `solve_all()` iterates every solution, `.over(vars)` to project; `differs` for hand-written loops |
| Readback | `model.get(&digits)?` reads `Int` / `Bool` in arrays, `Vec`, slices and tuples; `get_as::<usize>` converts via `TryFrom` |

Only `bool` / `i64` (Z3 `Bool` / `Int`) are supported for now.
