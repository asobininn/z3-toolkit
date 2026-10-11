use z3::{
    Optimize, SatResult, Solver,
    ast::{Bool, Int},
};
use z3_toolkit::prelude::*;

fn int(val: i64) -> Int {
    Int::from_i64(val)
}

/// `lhs ≡ rhs` then `true`
fn equiv(lhs: &Bool, rhs: &Bool) -> bool {
    let s = Solver::new();
    s.assert(lhs.eq(rhs).not());
    s.check() == SatResult::Unsat
}

#[test]
fn engines_and_labels() {
    let opt = Optimize::new();
    let [x] = ints("x");
    constraints! { opt; x in 0..=10 }
    opt.maximize(&x);
    assert_eq!(opt.solve().unwrap().get(&x), Ok(10));
    // label
    let s = Solver::new();
    constraints! { s; x >= 0, "low": x >= 5, "high": x <= 3 };
    match s.solve() {
        Err(Failure::Unsat { mut core }) => {
            core.sort();
            assert_eq!(core, ["high", "low"]);
        }
        other => panic!("expected unsat, got {other:?}"),
    }
}

#[test]
fn variables_and_readback() {
    let [x]: [Int; 1] = fresh("val");
    let [y]: [Int; 1] = fresh("val");
    let (grid, flag): ([[Int; 2]; 2], Bool) = fresh("g");
    let (row, flags) = (ints_n("row", 3), bools_n("f", 2));

    let s = Solver::new();
    constraints! { s;
        x == 1, y == 2, flag,
        for (i, line) in grid.iter().enumerate() {
            for (j, c) in line.iter().enumerate() {
                c == 2 * i + j
            }
        }
        for (j, c) in row.iter().enumerate() {
            c == j
        }
        flags[0], !flags[1],
    };
    let m = s.solve().unwrap();
    assert_eq!(m.get(&(&x, &y, &flag)), Ok((1, 2, true)));
    assert_eq!(m.get(&grid), Ok([[0, 1], [2, 3]]));
    assert_eq!(m.get(&row), Ok(vec![0, 1, 2]));
    assert_eq!(m.get(&row[1..]), Ok(vec![1, 2]));
    assert_eq!(m.get(&flags), Ok(vec![true, false]));
    // get_as
    assert_eq!(m.get_as::<usize>(&y), Ok(2usize));
    assert_eq!(m.get_as(&(&grid, &flag)), Ok(([[0u8, 1], [2, 3]], true)));
    assert!(matches!(
        m.get_as::<u8>(&int(-1)),
        Err(ReadError::Convert { .. })
    ))
}

#[test]
fn semantics() {
    let [p, q, r] = bools("p");
    let [x, y] = ints("x");
    let xs: [Int; 3] = fresh("xs");
    let fs: [Bool; 3] = fresh("fs");
    let k: usize = 2;
    let count: Int = fs.iter().map(|f| f.ite(&int(1), &int(0))).sum();
    let (yes, no) = (true, false);
    let grid: [[Int; 2]; 2] = fresh("st");
    let row = ints_n("row", 2);

    let cases: Vec<(&str, Bool, Bool)> = vec![
        // leaf
        ("cmp", constraint!(x < y), x.lt(&y)),
        ("literal on the left", constraint!(0 <= x), x.ge(0)),
        ("arith", constraint!(2 * x + 1 == y), (&x * 2 + 1).eq(&y)),
        ("usize lifted", constraint!(x == k), (x).eq(2)),
        ("bool as 0/1", constraint!(p + q == 1), p.xor(&q)),
        ("bool == bool", constraint!(p == q), p.eq(&q)),
        // logical op
        (
            "&& || !=",
            constraint!(p && !q || r),
            Bool::or(&[Bool::and(&[&p, &q.not()]), r.clone()]),
        ),
        // block category
        ("all", constraint!(all { p, q }), Bool::and(&[&p, &q])),
        (
            "not any",
            constraint!(not { any { p, q } }),
            Bool::or(&[&p, &q]).not(),
        ),
        (
            "implies",
            constraint!({ p } implies { q }),
            Bool::or(&[&p.not(), &q]),
        ),
        (
            "! around implies",
            constraint!(!({ p } implies { q }) && r),
            Bool::and(&[&p, &q.not(), &r]),
        ),
        (
            "&& with block",
            constraint!(p && any { q, r }),
            Bool::and(&[p.clone(), Bool::or(&[&q, &r])]),
        ),
        // rust control flow
        (
            "for under any",
            constraint!(any { for f in &fs { f } }),
            Bool::or(&fs),
        ),
        ("if", constraint!(if yes { p } else { q }), p.clone()),
        ("else", constraint!(if no { p } else { q }), q.clone()),
        // quantity / difference
        (
            "exactly",
            constraint!(exactly(1) { for f in &fs { f } }),
            count.eq(1),
        ),
        (
            "at_most",
            constraint!(at_most(1) { for f in &fs { f } }),
            count.le(1),
        ),
        (
            "at_least",
            constraint!(at_least(2) { for f in &fs { f } }),
            count.ge(2),
        ),
        (
            "distinct",
            constraint!(distinct(xs.iter())),
            Bool::and(&[xs[0].ne(&xs[1]), xs[0].ne(&xs[2]), xs[1].ne(&xs[2])]),
        ),
        // in
        (
            "in ..=",
            constraint!(x in 1..=3),
            Bool::and(&[x.ge(1), x.le(3)]),
        ),
        (
            "in ..",
            constraint!(x in 1..3),
            Bool::and(&[x.ge(1), x.lt(3)]),
        ),
        (
            "in array",
            constraint!(x in [2, 5]),
            Bool::or(&[x.eq(2), x.eq(5)]),
        ),
        (
            "in empty",
            constraint!(x in Vec::<i64>::new()),
            Bool::from_bool(false),
        ),
        // shape eq
        (
            "array == array",
            constraint!(grid[0] == grid[1]),
            constraint!(grid[0][0] == grid[1][0] && grid[0][1] == grid[1][1]),
        ),
        (
            "array == concrect",
            constraint!(grid[0] == [0, 1]),
            constraint!(grid[0][0] == 0 && grid[0][1] == 1),
        ),
        (
            "array == vec",
            constraint!([0, 1] == row && grid[0] == row),
            constraint!(row[0] == 0 && row[1] == 1 && grid[0][0] == 0 && grid[0][1] == 1),
        ),
        (
            "tuple",
            constraint!((x, p) == (k, true)),
            constraint!(x == 2 && p),
        ),
        (
            "nested !=",
            constraint!(grid != [[0, 0], [1, 1]]),
            constraint!(not { all { grid[0] == [0, 0], grid[1] == [1, 1] } }),
        ),
        (
            "vec len mismatch",
            constraint!(row == [1]),
            Bool::from_bool(false),
        ),
        // ite / at / with
        (
            "ite",
            constraint!(ite(&p, &x, &0) == y),
            constraint!(any { all {p, x == y}, all { !p, y == 0} }),
        ),
        (
            "ite shape",
            constraint!(ite(&p, &grid[0], &[0, 1]) == grid[1]),
            constraint!(any { all { p, grid[0] == grid[1] }, all { !p, grid[1] == [0, 1] } }),
        ),
        (
            "at",
            constraint!(x in 0..3 && xs.at(&x) == y),
            constraint!(any { for i in 0..3 { all { x == i, xs[i] ==y } } }),
        ),
        (
            "with",
            constraint!(xs.with(&1, &y) == [xs[0], y, xs[2]]),
            Bool::from_bool(true),
        ),
        (
            "with then at",
            constraint!({ x in 0..3 } implies { xs.with(&x, &0).at(&x) == 0 }),
            Bool::from_bool(true),
        ),
        // display label keeps the value
        (
            "@",
            constraint!(!("a" @ p) || "b" @ x < y),
            constraint!(!p || x < y),
        ),
        // linear
        ("sum", constraint!(sum(&fs) == 2), count.eq(2)),
        (
            "sum closure",
            constraint!(sum(&xs, |v| 2 * v) == 6),
            (&xs[0] * 2 + &xs[1] * 2 + &xs[2] * 2).eq(6),
        ),
        (
            "dot",
            constraint!(dot([1, 2, 3], &xs) == 0),
            (&xs[0] + &xs[1] * 2 + &xs[2] * 3).eq(0),
        ),
        (
            "lin!",
            lin!(3 * p + x).eq(&y),
            (p.ite(&int(3), &int(0)) + &x).eq(&y),
        ),
        (
            "abs max min",
            constraint!(abs(x - y) == max(x, y) - min(x, y)),
            Bool::from_bool(true),
        ),
        (
            "abs of usize diff",
            constraint!(abs(0usize - k) == k),
            Bool::from_bool(true),
        ),
    ];
    for (name, lhs, rhs) in &cases {
        assert!(equiv(lhs, rhs), "{name}")
    }
}

#[test]
fn differs_semantics() {
    let [x, y] = ints("x");
    let [p, q] = bools("p");
    let xs: [Int; 2] = fresh("xs");
    let s = Solver::new();
    constraints! { s; x == 1, y == 2, xs[0] == 3, xs[1] == 4, p, !q }
    let m = s.solve().unwrap();

    let cases: Vec<(&str, Bool, Bool)> = vec![
        ("int", differs(&m, [&x, &y]), constraint!(x != 1 || y != 2)),
        ("bool", differs(&m, [&p, &q]), constraint!(!p || q)),
        ("empty", differs::<Int>(&m, []), Bool::from_bool(false)),
        (
            "iter",
            constraint!(differs(&m, xs.iter())),
            constraint!(xs[0] != 3 || xs[1] != 4),
        ),
    ];
    for (name, lhs, rhs) in &cases {
        assert!(equiv(lhs, rhs), "{name}")
    }
}

#[test]
fn solve_all_works() {
    let [x] = ints("x");
    let s = Solver::new();
    constraints! { s; x in 0..3, "label": x >= 0 }
    assert_eq!(s.solve_all().count(), 3);
    assert_eq!(s.solve_all().count(), 0);
    // over
    let [y] = ints("y");
    let s = Solver::new();
    constraints! { s; x in 0..3, y in 0..3 }
    assert_eq!(s.solve_all().over([&x]).count(), 3);
    // `Optimize` yields the best first
    let opt = Optimize::new();
    constraints! { opt; x in 0..3 }
    opt.maximize(&x);
    let order: Vec<i64> = opt
        .solve_all()
        .map(|m| m.unwrap().get(&x).unwrap())
        .collect();
    assert_eq!(order, [2, 1, 0]);
}

#[test]
fn sove_all_reports_unknown_once() {
    struct GiveUp;
    impl Engine for GiveUp {
        fn assert_one(&self, _: &Bool) {}
        fn assert_tracked(&self, _: &Bool, _: &Bool) {}
        fn check_sat(&self) -> SatResult {
            SatResult::Unknown
        }
        fn model(&self) -> Option<z3::Model> {
            None
        }
        fn unsat_core(&self) -> Vec<Bool> {
            Vec::new()
        }
        fn reason_unknown(&self) -> Option<String> {
            Some("test".to_owned())
        }
    }
    let mut it = GiveUp.solve_all();
    assert!(matches!(it.next(), Some(Err(Failure::Unknown { .. }))));
    assert!(it.next().is_none());
}

#[test]
fn labels_readback() {
    let [x] = ints("x");
    let size = constraint!(any { "small" @ x < 3, "mid" @ x in 3..=7, "big" @ x > 7 });
    let odd = constraint!("odd" @ x % 2 == 1);
    let s = Solver::new();
    constraints! { s; size, x == 9 }
    let m = s.solve().unwrap();
    assert_eq!(m.labels(&size), Ok(vec!["big"]));
    assert_eq!(m.labels(&[&odd, &size]), Ok(vec!["odd", "big"]));
}
