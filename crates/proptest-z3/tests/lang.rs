use std::collections::{HashMap, HashSet};

use proptest::{prelude::*, strategy::ValueTree, test_runner::TestRunner};
use proptest_z3::{prelude::*, take_label_stats, take_rejections};

const VAR: Space = Space::new("var");
const FUNC: Space = Space::new("func");

// ==== Generate ====================

fn program() -> impl Strategy<Value = Skeleton> {
    prop::collection::vec(stmt(), 1..8).prop_map(steps)
}

fn program_with_kill() -> impl Strategy<Value = Skeleton> {
    let head = prop::collection::vec(VAR.def().prop_map(|x| frag!["def ", x]), 1..3);
    let tail = (VAR.kill(), VAR.refer()).prop_map(|(k, u)| [frag!["del ", k], frag!["print ", u]]);
    (head, prop::collection::vec(stmt(), 0..5), tail)
        .prop_map(|(head, body, tail)| steps(head.into_iter().chain(body).chain(tail)))
}

fn steps(stmts: impl IntoIterator<Item = Frag>) -> Skeleton {
    Skeleton::steps(stmts.into_iter().map(|s| frag![s, "\n"]))
}

fn stmt() -> impl Strategy<Value = Frag> {
    let leaf = prop_oneof![
        VAR.def().prop_map(|x| frag!["def ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["set ", x, " ", y]),
        VAR.refer().prop_map(|x| frag!["print ", x]),
        FUNC.def().prop_map(|f| frag!["fn ", f]),
        FUNC.refer().prop_map(|f| frag!["call ", f]),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop::collection::vec(inner, 0..4).prop_map(|body| {
            frag![
                "{ ",
                Frag::scope(body.into_iter().map(|s| frag![s, " "])),
                "}"
            ]
        })
    })
}

fn names() -> NameSpec {
    NameSpec::new()
        .space(VAR, "[a-c]", 3)
        .space(FUNC, "[a-c]", 3)
}

// ==== Checker ====================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Bug {
    Unbound,
    Killed,
}

#[derive(Debug, PartialEq)]
enum Error {
    Undefined(usize),
    Killed(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Reassigned,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Used;

fn the_mark(s: &Solved) -> (usize, Bug) {
    let [(hole, bug)] = s.marks::<Bug>()[..] else {
        panic!("one mark expected: {s:?}")
    };
    (s.span(hole).start, bug)
}

fn resolve(text: &str) -> (Vec<Error>, HashMap<usize, usize>) {
    let mut words =
        (text.split_whitespace()).map(|w| (w, w.as_ptr() as usize - text.as_ptr() as usize));
    let mut frames = vec![HashMap::new()];
    let mut errors = vec![];
    let mut targets = HashMap::new();
    while let Some((word, _)) = words.next() {
        let space = if matches!(word, "fn" | "call") {
            "func"
        } else {
            "var"
        };
        let mut name = || words.next().expect("a name after the keyword");
        let mut refer = |frames: &Vec<HashMap<_, Option<usize>>>, (name, at): (&str, usize)| {
            let state: Vec<Option<usize>> = frames
                .iter()
                .filter_map(|f| f.get(&(space, name)).copied())
                .collect();
            // match (state.contains(&true), state.is_empty()) {
            match (state.iter().rev().find_map(|&def| def), state.is_empty()) {
                (Some(def), _) => drop(targets.insert(at, def)),
                (None, true) => errors.push(Error::Undefined(at)),
                (None, false) => errors.push(Error::Killed(at)),
            }
        };
        match word {
            "{" => frames.push(HashMap::new()),
            "}" => drop(frames.pop()),
            "def" | "fn" => {
                let (x, at) = name();
                frames.last_mut().unwrap().insert((space, x), Some(at));
            }
            "set" => {
                let (x, y) = (name(), name());
                refer(&frames, y);
                frames.last_mut().unwrap().insert((space, x.0), Some(x.1));
            }
            "print" | "call" => refer(&frames, name()),
            "del" => {
                let x = name();
                refer(&frames, x);
                frames
                    .iter_mut()
                    .filter_map(|f| f.get_mut(&(space, x.0)))
                    .for_each(|alive| *alive = None);
            }
            other => panic!("unknown keyword `{other}`"),
        }
    }
    (errors, targets)
}

fn check(text: &str) -> Vec<Error> {
    resolve(text).0
}

// ==== Tests ====================

proptest! {
    /// Positive case: every reference points to a visible name.
    #[test]
    fn bound_has_no_errors(s in program().fill(names(), |binds| bound(binds, VAR) & bound(binds, FUNC))) {
        prop_assert_eq!(check(&s.text), vec![], "{:?}", s);
    }

    /// Negative case: exactly one reference is undefined, and the error is reported
    /// at the position of the marked hole.
    #[test]
    fn one_unbound_is_found_at_mark(s in program().fill(names(), |binds| constraint!(
        bound(binds, FUNC),
        exactly(1) { for i in binds.refs(VAR) {
            binds.mark(i, Bug::Unbound, !visible(binds, i))
        }},
    ))) {
        let (at, _) = the_mark(&s);
        let errors = check(&s.text);
        prop_assert!(errors == [Error::Undefined(at)], "{:?}", s);
    }

    /// Negative case: uses a deleted name in exactly one place; the error kind must match too.
    #[test]
    fn use_after_kill(s in program_with_kill().fill(names(), |binds| constraint!(
        bound(binds, FUNC),
        all { for i in binds.refs(VAR) { visible(binds, i) || killed(binds, i) } },
        exactly(1) { for i in binds.refs(VAR) {
            binds.mark(i, Bug::Killed, killed(binds, i))
        }}
    ))) {
        let (at, bug) = the_mark(&s);
        prop_assert_eq!(bug, Bug::Killed);
        prop_assert_eq!(check(&s.text), vec![Error::Killed(at)], "{:?}", s);
    }

    /// Classification: `observe` reads back labels without changing the distribution.
    #[test]
    fn observe_classifies(s in program().fill(names(), |binds| {
        binds.observe(Shape::Reassigned, !distinct_defs(binds, VAR));
        bound(binds, VAR) & bound(binds, FUNC)
    })) {
        let defs: Vec<&str> = s.holes().filter(|(_, h)| h.space == VAR && h.role == Role::Def)
            .map(|(_, h)| h.name.as_str()).collect();
        let unique: HashSet<_> = defs.iter().collect();
        prop_assert_eq!(s.has(Shape::Reassigned), unique.len() < defs.len(), "{:?}", s);
    }

    //`resolves_to` agrees with `Solved::target`.
    #[test]
    fn resolves_to_matches_target(s in program_with_kill().fill(names(), |binds| {
        for space in [VAR, FUNC] {
            for k in binds.defs(space) {
                binds.mark(k, Used, constraint!(any { for i in binds.refs(space) {
                    resolves_to(binds, i, k)
                }}));
            }
        }
        Bool::from_bool(true)
    })) {
        let used: HashSet<Hole> = s.marks::<Used>().into_iter().map(|(hole, _)| hole).collect();
        let expected: HashSet<Hole> = s.holes().filter_map(|(hole, _)| s.target(hole)).collect();
        prop_assert_eq!(used, expected, "{:?}", s);
    }


    #[test]
    fn targets_match_check(s in program().fill(names(), |binds| bound(binds, VAR) & bound(binds, FUNC))) {
        let (_, expected) = resolve(&s.text);
        for (hole, filled) in s.holes().filter(|(_, h)| h.role.is_ref()) {
            let target = s.target(hole).map(|def| s.span(def).start);
            prop_assert_eq!(target, expected.get(&filled.span.start).copied(), "{:?}", s);
        }
    }
}

/// A violation that is always broken together with another one when its precondition fails
/// can occur on its own once it is guarded with `given`.
#[test]
fn given_separates_implied_violations() {
    let run = |use_given: bool| {
        let strategy = program_with_kill().fill(names(), move |binds| {
            constraint!(
                bound(binds, FUNC),
                exactly(1) { for i in binds.refs(VAR) {
                    binds.mark(i, Bug::Killed, killed(binds, i)),
                    if use_given {
                        binds.mark(i, Bug::Unbound, (!visible(binds, i)).given([!killed(binds, i)]))
                    } else {
                        binds.mark(i, Bug::Unbound, !visible(binds, i))
                    }
                } },
            )
        });
        let mut runner = TestRunner::deterministic();
        (0..100)
            .flat_map(|_| {
                strategy
                    .new_tree(&mut runner)
                    .unwrap()
                    .current()
                    .marks::<Bug>()
            })
            .map(|(_, bug)| bug)
            .collect::<Vec<_>>()
    };
    assert!(!run(false).contains(&Bug::Killed));
    assert!(run(true).contains(&Bug::Killed));
}

/// Occurrences are counted with `label_stats`.
#[test]
fn label_stats_count_cases() {
    let strategy = program().fill(names(), |binds| {
        binds.observe(Shape::Reassigned, !distinct_defs(binds, VAR));
        bound(binds, VAR) & bound(binds, FUNC)
    });
    let mut runner = TestRunner::deterministic();
    for _ in 0..50 {
        strategy.new_tree(&mut runner).unwrap();
    }
    let [stat] = &take_label_stats()[..] else {
        panic!()
    };
    assert_eq!((stat.label.as_str(), stat.declared), ("Reassigned", 50));
    assert!(0 < stat.occurred && stat.occurred < 50, "{stat:?}");
}

#[test]
fn rejections_are_counted_by_core() {
    use proptest::test_runner::Config;
    for fill in [Fill::Greedy, Fill::Optimal] {
        let strategy = program().fill_with(fill, names(), |binds| {
            binds.require("bound", bound(binds, VAR));
            binds.require("one_unbound", unbound(binds, VAR, 1));
            Bool::from_bool(true)
        });
        let mut runner = TestRunner::new(Config {
            max_local_rejects: 20,
            ..Config::default()
        });
        assert!(strategy.new_tree(&mut runner).is_err());
        let rejections = take_rejections();
        let total: usize = rejections.iter().map(|(_, count)| count).sum();
        assert!(total > 0, "{fill:?}");
        for (core, _) in &rejections {
            assert!(!core.is_empty(), "{fill:?}: unnamed core");
            assert!(
                core.iter()
                    .all(|name| name == "bound" || name == "one_unbound"),
                "{fill:?}: {core:?}"
            );
        }
    }
}
